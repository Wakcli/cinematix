//! `cinematix render`: the film's frame range drawn into an offscreen target at a requested
//! resolution, one PNG per frame, optionally encoded by FFmpeg.
//!
//! The loop is one frame per redraw, the way `screenshot-sequence` works: the playhead is set to
//! `frame / fps`, the film camera renders the world through the ordinary frame path, the readback
//! is saved before the next frame starts. The editor's own UI is not drawn while a render runs.
//! The world must be still (`HORIZON_STILL=1` or `/tick freeze`), as `screenshot-sequence`
//! requires, and the renderer must be offscreen (`--offscreen`): `Renderer::resize` can retarget
//! an offscreen frame to any size, but a window's swapchain follows the window.

use super::Game;
use horizon_client::{
    Result,
    gfx::{ScreenshotRequest, pack_uniforms::FrameTiming},
    text::Text,
};
use serde_json::{Value, json};
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Instant;

const MAX_FRAMES: u32 = 10_000;
const MIN_SIZE: u32 = 16;
const MAX_SIZE: u32 = 8192;

pub(super) struct Render {
    start: u32,
    end: u32,
    fps: u32,
    width: u32,
    height: u32,
    dir: PathBuf,
    video: Option<PathBuf>,
    next: u32,
    /// Frames spent waiting for the film's own edits to remesh, bounded so a job cannot hang.
    stalls: u32,
    /// Redraws still to give a freshly remeshed layer before the frame is drawn.
    settle: u32,
    /// Frames that already existed when a resumed job started.
    resumed: u32,
    reply: Option<Sender<String>>,
    saved_size: [u32; 2],
    saved_playhead: f64,
    saved_playing: bool,
    started: Instant,
}

impl Render {
    /// What `cinematix status` reports while a job runs.
    pub(super) fn progress(&self) -> Value {
        json!({
            "start": self.start,
            "end": self.end,
            "next": self.next,
            "done": self.next.saturating_sub(self.start),
            "fps": self.fps,
            "size": [self.width, self.height],
            "path": self.dir.display().to_string(),
            "video": self.video.as_ref().map(|v| v.display().to_string()),
        })
    }

    /// The shader pack's clock: this film frame, counted from the render's first.
    pub(super) fn timing(&self) -> FrameTiming {
        let n = self.next.saturating_sub(self.start).saturating_add(1);
        FrameTiming::nth(n, 1000.0 / f64::from(self.fps))
    }

    fn frame_path(&self, frame: u32) -> PathBuf {
        self.dir.join(format!("frame_{frame:06}.png"))
    }
}

/// The first frame a resumed job still has to draw, and how many of its range already exist.
fn scan_frames(dir: &Path, start: u32, end: u32) -> (u32, u32) {
    let mut have = std::collections::HashSet::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(n) = name
                .strip_prefix("frame_")
                .and_then(|n| n.strip_suffix(".png"))
                .and_then(|n| n.parse::<u32>().ok())
            {
                have.insert(n);
            }
        }
    }
    let next = (start..end).find(|n| !have.contains(n)).unwrap_or(end);
    let resumed = (start..next).filter(|n| have.contains(n)).count() as u32;
    (next, resumed)
}

pub(super) fn is_video(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("mp4" | "mov" | "mkv")
    )
}

/// Where a `cinematix render` path writes: a directory of PNGs, or the frames folder beside a
/// video file.
fn output_dir(path: &Path) -> PathBuf {
    if is_video(path) {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "film".into());
        path.with_file_name(format!("{stem}.frames"))
    } else {
        path.to_owned()
    }
}

impl Game<'_> {
    /// `cinematix render START END FPS WIDTH HEIGHT PATH`. `START` and `END` are output frames,
    /// `END` exclusive; the film time is `frame / FPS`.
    pub(super) fn start_cinematix_render(
        &mut self,
        start: u32,
        end: u32,
        fps: u32,
        width: u32,
        height: u32,
        path: &str,
        resume: bool,
        reply: Option<&Sender<String>>,
    ) -> Result<Option<Value>> {
        if end <= start {
            return Err("render END must be after START".into());
        }
        if end - start > MAX_FRAMES {
            return Err(format!("render is at most {MAX_FRAMES} frames").into());
        }
        if !(1..=240).contains(&fps) {
            return Err("render FPS must be 1 to 240".into());
        }
        if !(MIN_SIZE..=MAX_SIZE).contains(&width) || !(MIN_SIZE..=MAX_SIZE).contains(&height) {
            return Err(format!("render size must be {MIN_SIZE} to {MAX_SIZE}").into());
        }
        if width % 2 != 0 || height % 2 != 0 {
            return Err("render width and height must be even (H.264's chroma planes)".into());
        }
        if self.screen != super::Screen::Cinematix {
            return Err("open the Cinematix editor before rendering the film".into());
        }
        if self.cinematix_export.is_some() {
            return Err("a Cinematix render is already running".into());
        }
        if self.screenshot_sequence.is_some() {
            return Err("a screenshot sequence is running".into());
        }
        if let Some(refusal) = self.stillness_refusal(false) {
            return Err(format!("cinematix render: {refusal}").into());
        }
        let renderer = self.renderer.as_ref().ok_or("no renderer yet")?;
        if !renderer.offscreen() {
            return Err(
                "cinematix render needs --offscreen: a window's frame follows the window's size"
                    .into(),
            );
        }
        if self.pending_screenshot.is_some() || renderer.screenshot.is_some() {
            return Err("a screenshot is already pending".into());
        }

        let path = PathBuf::from(path);
        let dir = output_dir(&path);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?;
        let video = is_video(&path).then_some(path.clone());
        let saved_size = [renderer.config.width, renderer.config.height];
        let saved_playhead = self.cinematix.playhead.t;
        let saved_playing = self.cinematix.playing;
        let (next, resumed) = if resume {
            scan_frames(&dir, start, end)
        } else {
            (start, 0)
        };

        let renderer = self.renderer.as_mut().ok_or("no renderer yet")?;
        renderer.resize(width, height);
        if renderer.config.width != width || renderer.config.height != height {
            renderer.resize(saved_size[0], saved_size[1]);
            return Err(format!("this device cannot draw {width}x{height}").into());
        }
        self.cinematix.playing = false;
        self.cinematix_export = Some(Render {
            start,
            end,
            fps,
            width,
            height,
            dir: dir.clone(),
            video: video.clone(),
            next,
            stalls: 0,
            settle: 0,
            resumed,
            reply: reply.cloned(),
            saved_size,
            saved_playhead,
            saved_playing,
            started: Instant::now(),
        });
        if reply.is_some() {
            // The socket's reply waits for the last frame, as screenshot-sequence's does.
            return Ok(None);
        }
        self.system_message(Text::tr(
            "horizon.cinematix.render_started",
            &[&(end - start), &dir.display().to_string()],
        ));
        Ok(Some(json!({
            "ok": true, "started": true,
            "frames": end - start, "fps": fps, "size": [width, height],
            "path": dir.display().to_string(),
            "next": next, "resumed": resumed,
        })))
    }

    /// One frame of a running render, on the frame path: set the playhead, draw, save the
    /// readback, then advance or finish. Returns whether a frame was produced.
    pub(super) fn render_cinematix_frame(&mut self) -> Result<bool> {
        if self
            .cinematix_export
            .as_ref()
            .is_some_and(|r| r.next >= r.end)
        {
            self.finish_cinematix_render();
            return Ok(true);
        }
        let (frame, fps, path) = {
            let render = self
                .cinematix_export
                .as_ref()
                .expect("a running render");
            (render.next, render.fps, render.frame_path(render.next))
        };
        self.cinematix.playhead.t = f64::from(frame) / f64::from(fps);
        // The layer can mark sections dirty at this very frame (an op activating here). Give the
        // world update and the mesher two redraws to land the remesh, then wait for the queues;
        // the frame is retried, never skipped.
        if self.sync_cinematix_world() {
            self.cinematix_export
                .as_mut()
                .expect("a running render")
                .settle = 2;
        }
        {
            let render = self.cinematix_export.as_mut().expect("a running render");
            if render.settle > 0 {
                render.settle -= 1;
                return Ok(false);
            }
        }
        if self.streaming.busy()
            || !self.world.dirty_sections.is_empty()
            || !self.pending_uploads.is_empty()
        {
            let render = self.cinematix_export.as_mut().expect("a running render");
            render.stalls += 1;
            if render.stalls > 1200 {
                self.end_cinematix_render(Err(
                    "the world did not finish meshing the film's block edits".into(),
                ));
            }
            return Ok(false);
        }
        if let Some(refusal) = self.stillness_refusal(false) {
            self.end_cinematix_render(Err(refusal));
            return Ok(true);
        }
        {
            let renderer = self.renderer.as_mut().ok_or("no renderer")?;
            renderer.screenshot = Some(ScreenshotRequest {
                path,
                size: None,
                scaled: false,
                background: false,
            });
            renderer.screenshot_result = None;
        }
        let drawn = self.render_frame();
        let renderer = self.renderer.as_mut().ok_or("no renderer")?;
        let queued = renderer.screenshot.take().is_some();
        match drawn {
            Ok(false) => return Ok(false),
            Err(error) => {
                self.end_cinematix_render(Err(error.to_string()));
                return Ok(true);
            }
            Ok(true) => {}
        }
        if queued {
            self.end_cinematix_render(Err("the frame was not drawn".into()));
            return Ok(true);
        }
        if let Err(error) = renderer.finish_sequence_frame() {
            self.end_cinematix_render(Err(error.to_string()));
            return Ok(true);
        }
        let saved = match renderer.screenshot_result.take() {
            Some(Ok(pending)) => pending
                .save_with_details()
                .map(|saved| saved.size)
                .map_err(|error| error.to_string()),
            Some(Err(error)) => Err(error),
            None => Err(IoError::other("the frame's PNG wasn't written".to_string()).to_string()),
        };
        let size = match saved {
            Ok(size) => size,
            Err(error) => {
                self.end_cinematix_render(Err(error.to_string()));
                return Ok(true);
            }
        };
        let render = self.cinematix_export.as_mut().expect("a running render");
        if size != [render.width, render.height] {
            let error = format!(
                "the frame came back {}x{}, not {}x{}",
                size[0], size[1], render.width, render.height
            );
            self.end_cinematix_render(Err(error));
            return Ok(true);
        }
        render.next += 1;
        while render.next < render.end && render.frame_path(render.next).exists() {
            render.resumed += 1;
            render.next += 1;
        }
        render.stalls = 0;
        render.settle = 0;
        if render.next >= render.end {
            self.finish_cinematix_render();
        }
        Ok(true)
    }

    fn finish_cinematix_render(&mut self) {
        let Some(render) = self.cinematix_export.take() else {
            return;
        };
        self.restore_cinematix_render_target(&render);
        let elapsed_ms = render.started.elapsed().as_millis() as u64;
        let mut value = json!({
            "ok": true,
            "path": render.dir.display().to_string(),
            "frames": render.end - render.start,
            "fps": render.fps,
            "size": [render.width, render.height],
            "elapsed_ms": elapsed_ms,
            "stalls": render.stalls,
            "resumed": render.resumed,
        });
        if let Some(video) = &render.video {
            value["video"] = json!(video.display().to_string());
            let args = cinematix_render::image_sequence_args(
                &render.dir.display().to_string(),
                &video.display().to_string(),
                render.fps,
                cinematix_render::Codec::H264,
            );
            match std::process::Command::new("ffmpeg").args(&args).status() {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    value["ok"] = json!(false);
                    value["error"] = json!(format!("ffmpeg exited with {status}"));
                }
                Err(error) => {
                    value["ok"] = json!(false);
                    value["error"] = json!(format!("ffmpeg could not start: {error}"));
                }
            }
        }
        self.report_cinematix_render(render, Ok(value));
    }

    /// Aborts a running render (an error, a world that moved, `quit`): the frames written so far
    /// stay on disk.
    pub(super) fn end_cinematix_render(&mut self, outcome: std::result::Result<Value, String>) {
        let Some(render) = self.cinematix_export.take() else {
            return;
        };
        self.restore_cinematix_render_target(&render);
        self.report_cinematix_render(render, outcome);
    }

    fn restore_cinematix_render_target(&mut self, render: &Render) {
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(render.saved_size[0], render.saved_size[1]);
        }
        self.cinematix.playhead.t = render.saved_playhead;
        self.cinematix.playing = render.saved_playing;
        // The world layer follows the playhead back: a block that was active for frame N is gone
        // again in the editor's own time.
        self.cinematix_world_time = None;
        let _ = self.sync_cinematix_world();
    }

    fn report_cinematix_render(
        &mut self,
        render: Render,
        outcome: std::result::Result<Value, String>,
    ) {
        let written = render.next.saturating_sub(render.start + render.resumed);
        let value = match outcome {
            Ok(mut value) => {
                value["written"] = json!(written);
                value
            }
            Err(error) => json!({
                "ok": false, "error": error, "written": written,
                "path": render.dir.display().to_string(),
            }),
        };
        match render.reply {
            Some(reply) => {
                let _ = reply.send(value.to_string());
            }
            None => {
                if value["ok"].as_bool() == Some(true) {
                    self.system_message(Text::tr(
                        "horizon.cinematix.render_done",
                        &[&written, &render.dir.display().to_string()],
                    ));
                } else {
                    let error = value["error"].as_str().unwrap_or("unknown error");
                    self.system_message(Text::tr("horizon.cinematix.render_error", &[&error]));
                }
            }
        }
    }

    /// While a render runs, controls that would change the world, the camera or the settings are
    /// refused; the status and a cancel are let through.
    pub(super) fn cinematix_render_gate(&mut self, words: &[&str]) -> Result<()> {
        if self.cinematix_export.is_none() {
            return Ok(());
        }
        match words {
            ["quit"] => {
                self.end_cinematix_render(Err("cancelled: the game is quitting".into()));
                Ok(())
            }
            ["cinematix", "render", "cancel"] => Ok(()),
            ["cinematix", "status"] | ["help"] | ["hello"] | ["status"] | ["state"] => Ok(()),
            _ => Err("a Cinematix render is running: wait for its reply, or `cinematix render cancel`"
                .into()),
        }
    }

}

#[cfg(test)]
mod tests {
    use super::{is_video, output_dir, scan_frames};
    use std::path::{Path, PathBuf};

    #[test]
    fn a_videos_frames_live_beside_it() {
        assert_eq!(
            output_dir(Path::new("/tmp/film.mp4")),
            PathBuf::from("/tmp/film.frames")
        );
        assert_eq!(
            output_dir(Path::new("/tmp/film")),
            PathBuf::from("/tmp/film")
        );
    }

    #[test]
    fn a_resumed_job_starts_at_the_first_missing_frame() {
        let dir = std::env::temp_dir().join(format!("cinematix-resume-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for n in [0u32, 1, 3] {
            std::fs::write(dir.join(format!("frame_{n:06}.png")), b"x").unwrap();
        }
        std::fs::write(dir.join("frame_000002.txt"), b"x").unwrap();
        assert_eq!(scan_frames(&dir, 0, 5), (2, 2));
        assert_eq!(scan_frames(&dir, 0, 2), (2, 2));
        assert_eq!(scan_frames(&dir, 4, 6), (4, 0));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn only_video_extensions_are_videos() {
        assert!(is_video(Path::new("a.mp4")));
        assert!(is_video(Path::new("a.MOV")) == false);
        assert!(!is_video(Path::new("a.png")));
        assert!(!is_video(Path::new("a")));
    }
}
