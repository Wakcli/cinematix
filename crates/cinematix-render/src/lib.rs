use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Codec {
    H264,
    Hevc,
    ProRes422,
    ProRes4444,
    PngSequence,
}

impl Codec {
    pub fn has_alpha(self) -> bool {
        matches!(self, Codec::ProRes4444 | Codec::PngSequence)
    }

    pub fn pixel_format(self) -> &'static str {
        match self {
            Codec::H264 | Codec::Hevc => "yuv420p",
            Codec::ProRes422 => "yuv422p10le",
            Codec::ProRes4444 => "yuva444p10le",
            Codec::PngSequence => "rgba",
        }
    }

    pub fn encoder(self) -> Option<&'static str> {
        match self {
            Codec::H264 => Some("libx264"),
            Codec::Hevc => Some("libx265"),
            Codec::ProRes422 | Codec::ProRes4444 => Some("prores_ks"),
            Codec::PngSequence => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub codec: Codec,
}

impl Output {
    pub fn frame_bytes(&self) -> u64 {
        self.width as u64 * self.height as u64 * 4
    }

    pub fn bitrate_bytes_per_sec(&self) -> u64 {
        self.frame_bytes() * self.fps as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: u32,
    pub end: u32,
}

impl Range {
    pub fn new(start: u32, end: u32) -> Self {
        if end < start { Self { start: end, end: start } } else { Self { start, end } }
    }

    pub fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains(&self, f: u32) -> bool {
        f >= self.start && f < self.end
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderJob {
    pub output: Output,
    pub range: Range,
    pub subframes: u16,
    pub path: String,
}

impl RenderJob {
    pub fn new(output: Output, range: Range, path: impl Into<String>) -> Self {
        Self { output, range, subframes: 1, path: path.into() }
    }

    pub fn frame_time(&self, f: u32) -> f64 {
        f as f64 / self.output.fps as f64
    }

    pub fn sample_times(&self, f: u32) -> Vec<f64> {
        let n = self.subframes.max(1) as u32;
        let t0 = self.frame_time(f);
        let dt = 1.0 / (self.output.fps as f64 * n as f64);
        (0..n).map(|s| t0 + s as f64 * dt).collect()
    }

    pub fn frames(&self) -> impl Iterator<Item = u32> + '_ {
        self.range.start..self.range.end
    }

    pub fn total_bytes(&self) -> u64 {
        self.range.len() as u64 * self.output.frame_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Progress {
    pub next_frame: u32,
    pub bytes_written: u64,
}

impl Progress {
    pub fn advance(&mut self, job: &RenderJob) {
        self.next_frame = self.next_frame.saturating_add(1).min(job.range.end);
        self.bytes_written = self.bytes_written.saturating_add(job.output.frame_bytes());
    }

    pub fn complete(&self, job: &RenderJob) -> bool {
        self.next_frame >= job.range.end
    }

    pub fn is_empty(&self) -> bool {
        self.next_frame == 0 && self.bytes_written == 0
    }
}

pub fn ffmpeg_args(job: &RenderJob) -> Vec<String> {
    let o = &job.output;
    let mut a: Vec<String> = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "rgba".into(),
        "-s".into(),
        format!("{}x{}", o.width, o.height),
        "-r".into(),
        o.fps.to_string(),
        "-i".into(),
        "-".into(),
    ];
    if let Some(enc) = o.codec.encoder() {
        a.extend(["-c:v".into(), enc.into(), "-pix_fmt".into(), o.codec.pixel_format().into()]);
        if matches!(o.codec, Codec::ProRes422) {
            a.extend(["-profile:v".into(), "2".into()]);
        }
        if matches!(o.codec, Codec::ProRes4444) {
            a.extend(["-profile:v".into(), "4".into()]);
        }
        a.extend(["-y".into(), job.path.clone()]);
    }
    a
}

pub fn png_pattern(dir: &str, prefix: &str) -> String {
    let d = dir.trim_end_matches('/');
    format!("{d}/{prefix}_%06d.png")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job() -> RenderJob {
        let o = Output { width: 1920, height: 1080, fps: 24, codec: Codec::H264 };
        RenderJob::new(o, Range::new(0, 48), "/tmp/out.mp4")
    }

    #[test]
    fn frame_count_matches_range() {
        assert_eq!(job().frames().count(), 48);
    }

    #[test]
    fn frame_times_are_exact() {
        let j = job();
        assert_eq!(j.frame_time(0), 0.0);
        assert!((j.frame_time(24) - 1.0).abs() < 1e-12);
        assert!((j.frame_time(12) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn subframes_span_one_frame() {
        let mut j = job();
        j.subframes = 4;
        let s = j.sample_times(10);
        assert_eq!(s.len(), 4);
        assert!((s[1] - s[0] - 1.0 / 96.0).abs() < 1e-12);
        assert!((s[3] - s[0]).abs() < 1.0 / 24.0 + 1e-12);
    }

    #[test]
    fn range_normalises_and_is_half_open() {
        let r = Range::new(10, 4);
        assert_eq!((r.start, r.end), (4, 10));
        assert!(r.contains(4) && r.contains(9) && !r.contains(10));
    }

    #[test]
    fn progress_resumes_at_next_frame() {
        let j = job();
        let mut p = Progress::default();
        for _ in 0..10 {
            p.advance(&j);
        }
        assert_eq!(p.next_frame, 10);
        assert_eq!(p.bytes_written, 10 * j.output.frame_bytes());
        assert!(!p.complete(&j));
        for _ in 0..40 {
            p.advance(&j);
        }
        assert_eq!(p.next_frame, 48);
        assert!(p.complete(&j));
    }

    #[test]
    fn prores4444_carries_alpha_but_h264_cannot() {
        assert!(Codec::ProRes4444.has_alpha());
        assert!(!Codec::H264.has_alpha());
        assert_eq!(Codec::ProRes4444.pixel_format(), "yuva444p10le");
        assert_eq!(Codec::ProRes422.pixel_format(), "yuv422p10le");
    }

    #[test]
    fn ffmpeg_args_are_well_formed() {
        let a = ffmpeg_args(&job());
        let s = a.join(" ");
        assert!(s.contains("-f rawvideo -pix_fmt rgba -s 1920x1080 -r 24 -i -"));
        assert!(s.contains("-c:v libx264 -pix_fmt yuv420p"));
        assert!(s.ends_with("/tmp/out.mp4"));
    }

    #[test]
    fn png_sequence_avoids_ffmpeg_encoder() {
        let mut j = job();
        j.output.codec = Codec::PngSequence;
        assert!(ffmpeg_args(&j).iter().all(|a| a != "-c:v"));
        assert_eq!(png_pattern("/out/frames/", "shot01"), "/out/frames/shot01_%06d.png");
    }

    #[test]
    fn throughput_estimate_is_sane() {
        let j = job();
        assert_eq!(j.output.frame_bytes(), 1920 * 1080 * 4);
        assert_eq!(j.output.bitrate_bytes_per_sec(), 1920 * 1080 * 4 * 24);
        assert_eq!(j.total_bytes(), 48 * 1920 * 1080 * 4);
    }
}
