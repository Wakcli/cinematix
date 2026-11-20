//! `.cmx`, the film file: a zip container with `project.toml` (format version, app, timestamp)
//! and `scene.json` (the scene, every channel, the world layer and the asset references).
//!
//! JSON because a film must stay readable and mergeable outside the game; a zip because assets
//! will be blobs beside it (`assets/<sha256>.<ext>`), deduped by content. `format_version` moves
//! forward only through `upgrade`, which refuses a file from a newer build rather than guessing.

use crate::document::Document;
use crate::document::props_of;
use cinematix_anim::Channel;
use cinematix_scene::{NodeKind, Scene};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const EXTENSION: &str = "cmx";
pub const FORMAT_VERSION: i64 = 2;
const PROJECT: &str = "project.toml";
const SCENE: &str = "scene.json";

#[derive(Serialize, Deserialize)]
struct SceneFile {
    scene: Scene,
    channels: Vec<Channel>,
}

#[derive(Serialize, Deserialize)]
struct Project {
    format_version: i64,
    app: String,
    app_version: String,
    saved_unix: u64,
}

/// What the caller knows and the pure crate must not guess: who wrote the file and when.
#[derive(Clone, Copy, Debug)]
pub struct Stamp<'a> {
    pub app_version: &'a str,
    pub saved_unix: u64,
}

impl Default for Stamp<'_> {
    fn default() -> Self {
        Self {
            app_version: env!("CARGO_PKG_VERSION"),
            saved_unix: 0,
        }
    }
}

fn zip_error(error: impl std::fmt::Display) -> String {
    format!("film zip: {error}")
}

fn read_entry(archive: &mut ZipArchive<File>, name: &str) -> Result<String, String> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| format!("film has no {name}"))?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|error| format!("film {name}: {error}"))?;
    Ok(text)
}

/// A project version newer than this build understands is refused, not partially read. Version 1
/// films load unchanged: version 2 only added the `Spline` interpolation key, which a version 1
/// file cannot contain, so the upcast is "nothing to rewrite".
fn upgrade(version: i64) -> Result<(), String> {
    if version > FORMAT_VERSION {
        return Err(format!(
            "film format {version} is newer than this build's {FORMAT_VERSION}; update Cinematix"
        ));
    }
    if version < 1 {
        return Err(format!("film format {version} is not a Cinematix film"));
    }
    Ok(())
}

/// Writes the document to `path` as a `.cmx`, creating the parent folder.
pub fn save(path: &Path, doc: &Document, stamp: Stamp<'_>) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let project = Project {
        format_version: FORMAT_VERSION,
        app: "cinematix".to_owned(),
        app_version: stamp.app_version.to_owned(),
        saved_unix: stamp.saved_unix,
    };
    let project = toml::to_string(&project).map_err(|error| format!("project.toml: {error}"))?;
    let scene = SceneFile {
        scene: doc.scene.clone(),
        channels: doc.channels.clone(),
    };
    let scene =
        serde_json::to_vec_pretty(&scene).map_err(|error| format!("scene.json: {error}"))?;

    let file = File::create(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(PROJECT, options).map_err(zip_error)?;
    zip.write_all(project.as_bytes()).map_err(zip_error)?;
    zip.start_file(SCENE, options).map_err(zip_error)?;
    zip.write_all(&scene).map_err(zip_error)?;
    zip.finish().map_err(zip_error)?;
    Ok(())
}

/// Reads a `.cmx` back into a document. Every channel is required to target a property its node
/// has, so a hand-edited file fails loudly instead of quietly animating nothing.
pub fn load(path: &Path) -> Result<Document, String> {
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut archive = ZipArchive::new(file).map_err(zip_error)?;
    let project: Project = toml::from_str(&read_entry(&mut archive, PROJECT)?)
        .map_err(|error| format!("project.toml: {error}"))?;
    upgrade(project.format_version)?;
    let scene: SceneFile = serde_json::from_str(&read_entry(&mut archive, SCENE)?)
        .map_err(|error| format!("scene.json: {error}"))?;

    let camera = scene
        .scene
        .nodes
        .get(&scene.scene.camera)
        .ok_or("scene.json: the camera node is missing")?;
    if !matches!(camera.kind, NodeKind::Camera(_)) {
        return Err("scene.json: the camera id points at something that is not a camera".into());
    }
    for channel in &scene.channels {
        let node = scene.scene.nodes.get(&channel.target.node).ok_or_else(|| {
            format!(
                "scene.json: channel target {} is missing",
                channel.target.node.0
            )
        })?;
        if !props_of(&node.kind).contains(&channel.target.prop) {
            return Err(format!(
                "scene.json: channel {} on node {} is not a property that node has",
                channel.target.prop as u8, channel.target.node.0
            ));
        }
    }

    let mut doc = Document {
        scene: scene.scene,
        channels: Vec::new(),
    };
    doc.sync_channels();
    for channel in scene.channels {
        let Some(ci) = doc.channel_index(channel.target.node, channel.target.prop) else {
            continue;
        };
        let mut keys = channel.keys;
        keys.sort_by(|a, b| a.t.total_cmp(&b.t));
        doc.channels[ci].keys = keys;
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::editor::Editor;
    use cinematix_anim::{Interp, Key, Value};
    use cinematix_scene::NodeId;
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT: AtomicU32 = AtomicU32::new(0);

    fn temp_path() -> std::path::PathBuf {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("cinematix-film-{}-{n}.cmx", std::process::id()))
    }

    fn animated() -> Document {
        let mut doc = Document::new();
        let ci = doc
            .channel_index(NodeId(1), cinematix_anim::Prop::Translation)
            .unwrap();
        doc.channels[ci].keys = vec![
            Key {
                t: 0.0,
                v: Value::Vec3([1.0, 64.0, 2.0]),
                interp: Interp::Smooth,
            },
            Key {
                t: 2.0,
                v: Value::Vec3([5.0, 70.0, -4.0]),
                interp: Interp::Linear,
            },
        ];
        let fi = doc
            .channel_index(NodeId(1), cinematix_anim::Prop::Fov)
            .unwrap();
        doc.channels[fi].keys = vec![Key {
            t: 1.0,
            v: Value::F32(35.0),
            interp: Interp::Step,
        }];
        let li = doc
            .channel_index(NodeId(2), cinematix_anim::Prop::Intensity)
            .unwrap();
        doc.channels[li].keys = vec![Key {
            t: 0.5,
            v: Value::F32(2.5),
            interp: Interp::Smooth,
        }];
        doc
    }

    #[test]
    fn a_film_round_trips_through_the_zip() {
        let path = temp_path();
        let doc = animated();
        save(&path, &doc, Stamp::default()).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.channels.len(), doc.channels.len());
        assert_eq!(loaded.key_count(), 4);
        let mut a = Editor::new();
        a.doc = doc;
        let mut b = Editor::new();
        b.doc = loaded;
        for t in [0.0, 0.5, 1.0, 1.5, 2.0, 3.0] {
            assert_eq!(
                a.doc.channels[0].eval(t).map(|v| format!("{v:?}")),
                b.doc.channels[0].eval(t).map(|v| format!("{v:?}")),
                "t={t}"
            );
        }
        assert!((b.preview_fov() - 35.0).abs() < 0.01);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_version_one_film_loads_and_saves_as_version_two() {
        let path = temp_path();
        let doc = animated();
        // Write the file the version 1 build wrote: same entries, format_version 1.
        let file = File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file(PROJECT, options).unwrap();
        zip.write_all(
            b"format_version = 1\napp = \"cinematix\"\napp_version = \"0.1.0\"\nsaved_unix = 0\n",
        )
        .unwrap();
        zip.start_file(SCENE, options).unwrap();
        let scene = SceneFile {
            scene: doc.scene.clone(),
            channels: doc.channels.clone(),
        };
        zip.write_all(&serde_json::to_vec(&scene).unwrap()).unwrap();
        zip.finish().unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.key_count(), 4);

        save(&path, &loaded, Stamp::default()).unwrap();
        let mut archive = ZipArchive::new(File::open(&path).unwrap()).unwrap();
        let project = read_entry(&mut archive, PROJECT).unwrap();
        assert!(project.contains("format_version = 2"), "{project}");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_newer_format_is_refused_with_its_version() {
        let path = temp_path();
        let file = File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        zip.start_file(PROJECT, options).unwrap();
        zip.write_all(
            b"format_version = 99\napp = \"cinematix\"\napp_version = \"x\"\nsaved_unix = 0\n",
        )
        .unwrap();
        zip.start_file(SCENE, options).unwrap();
        zip.write_all(b"{}").unwrap();
        zip.finish().unwrap();
        let error = load(&path).unwrap_err();
        assert!(error.contains("99") && error.contains("newer"), "{error}");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_scene_with_no_camera_is_refused() {
        let path = temp_path();
        let mut doc = Document::new();
        doc.scene.camera = NodeId(2);
        save(&path, &doc, Stamp::default()).unwrap();
        let error = load(&path).unwrap_err();
        assert!(error.contains("not a camera"), "{error}");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_channel_for_a_missing_node_is_refused() {
        let path = temp_path();
        let mut doc = Document::new();
        let ci = doc
            .channel_index(NodeId(1), cinematix_anim::Prop::Fov)
            .unwrap();
        doc.channels[ci].keys = vec![Key {
            t: 0.0,
            v: Value::F32(70.0),
            interp: Interp::Smooth,
        }];
        doc.channels[ci].target.node = NodeId(404);
        save(&path, &doc, Stamp::default()).unwrap();
        let error = load(&path).unwrap_err();
        assert!(error.contains("404"), "{error}");
        std::fs::remove_file(&path).ok();
    }
}
