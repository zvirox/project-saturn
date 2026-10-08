use crate::time::{FrameRate, Tick};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const PROJECT_FORMAT: &str = "project-saturn.project";
pub const PROJECT_SCHEMA_VERSION: u32 = 3;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectDocument {
    pub format: String,
    pub schema_version: u32,
    pub generator: String,
    pub project: Project,
    #[serde(default)]
    pub view: ProjectView,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub settings: ProjectSettings,
    pub media: Vec<MediaAsset>,
    pub sequence: Sequence,
    pub next_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub width: u32,
    pub height: u32,
    pub frame_rate: FrameRate,
    pub audio_sample_rate: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Video,
    Audio,
    Image,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: u64,
    pub name: String,
    /// Source media stays external; project files store its path, not its contents.
    pub path: PathBuf,
    pub kind: MediaKind,
    #[serde(default)]
    pub duration: Option<Tick>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sequence {
    pub name: String,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub kind: TrackKind,
    pub name: String,
    #[serde(default)]
    pub gain_db: f64,
    pub clips: Vec<Clip>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub media_id: u64,
    pub timeline_start: Tick,
    pub source_in: Tick,
    pub duration: Tick,
    #[serde(default)]
    pub color: ColorAdjustments,
    #[serde(default)]
    pub keyframes: Vec<Keyframe>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorAdjustments {
    pub exposure: f64,
    pub contrast: f64,
    pub saturation: f64,
}

impl Default for ColorAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum KeyframeProperty {
    PositionX,
    PositionY,
    Scale,
    Rotation,
    Opacity,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keyframe {
    pub property: KeyframeProperty,
    /// Time relative to the clip's source in point.
    pub time: Tick,
    pub value: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProjectView {
    #[serde(default)]
    pub active_sequence: Option<String>,
    #[serde(default)]
    pub timeline_zoom: f64,
    #[serde(default)]
    pub timeline_scroll: f64,
}

impl Default for ProjectDocument {
    fn default() -> Self {
        Self::new("Untitled project")
    }
}

impl ProjectDocument {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            format: PROJECT_FORMAT.to_owned(),
            schema_version: PROJECT_SCHEMA_VERSION,
            generator: format!("Project Saturn {}", env!("CARGO_PKG_VERSION")),
            project: Project {
                name: name.into(),
                settings: ProjectSettings {
                    width: 1920,
                    height: 1080,
                    frame_rate: FrameRate::FPS_30,
                    audio_sample_rate: 48_000,
                },
                media: Vec::new(),
                sequence: Sequence {
                    name: "Sequence 1".into(),
                    tracks: vec![
                        Track {
                            id: 1,
                            kind: TrackKind::Video,
                            name: "V1".into(),
                            gain_db: 0.0,
                            clips: Vec::new(),
                        },
                        Track {
                            id: 2,
                            kind: TrackKind::Audio,
                            name: "A1".into(),
                            gain_db: 0.0,
                            clips: Vec::new(),
                        },
                    ],
                },
                next_id: 3,
            },
            view: ProjectView {
                active_sequence: Some("Sequence 1".into()),
                timeline_zoom: 1.0,
                timeline_scroll: 0.0,
            },
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format != PROJECT_FORMAT {
            return Err(format!("Unsupported project format `{}`", self.format));
        }
        if self.project.name.trim().is_empty() {
            return Err("Project name cannot be empty".into());
        }
        if self.project.settings.width == 0 || self.project.settings.height == 0 {
            return Err("Project dimensions must be greater than zero".into());
        }
        if self.project.settings.audio_sample_rate == 0
            || self.project.settings.frame_rate.frame_ticks().is_none()
        {
            return Err("Project uses an unsupported frame or audio rate".into());
        }
        let mut ids = std::collections::HashSet::new();
        for item in &self.project.media {
            if !ids.insert(item.id) {
                return Err(format!("Duplicate media id {}", item.id));
            }
        }
        for track in &self.project.sequence.tracks {
            if !track.gain_db.is_finite() || !(-60.0..=12.0).contains(&track.gain_db) {
                return Err(format!("Track {} has invalid gain", track.name));
            }
            for clip in &track.clips {
                if clip.duration.0 <= 0
                    || !clip.color.exposure.is_finite()
                    || !(-5.0..=5.0).contains(&clip.color.exposure)
                    || !clip.color.contrast.is_finite()
                    || !(0.0..=2.0).contains(&clip.color.contrast)
                    || !clip.color.saturation.is_finite()
                    || !(0.0..=2.0).contains(&clip.color.saturation)
                {
                    return Err("Project contains invalid clip properties".into());
                }
                for keyframe in &clip.keyframes {
                    let value_in_range = match &keyframe.property {
                        KeyframeProperty::PositionX | KeyframeProperty::PositionY => {
                            (-100_000.0..=100_000.0).contains(&keyframe.value)
                        }
                        KeyframeProperty::Scale => (0.01..=100.0).contains(&keyframe.value),
                        KeyframeProperty::Rotation => {
                            (-36_000.0..=36_000.0).contains(&keyframe.value)
                        }
                        KeyframeProperty::Opacity => (0.0..=1.0).contains(&keyframe.value),
                    };
                    if keyframe.time.0 < 0
                        || keyframe.time.0 > clip.duration.0
                        || !keyframe.value.is_finite()
                        || !value_in_range
                    {
                        return Err("Project contains an invalid keyframe".into());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn save_atomic(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|e| format!("Could not create project folder: {e}"))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| "Project path has no file name".to_string())?;
        let suffix = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary_path = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            suffix
        ));
        let serialized = serde_json::to_vec_pretty(self)
            .map_err(|e| format!("Could not serialize project: {e}"))?;

        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
                .map_err(|e| format!("Could not create temporary project file: {e}"))?;
            file.write_all(&serialized)
                .map_err(|e| format!("Could not write project file: {e}"))?;
            file.write_all(b"\n")
                .map_err(|e| format!("Could not finish project file: {e}"))?;
            file.sync_all()
                .map_err(|e| format!("Could not flush project file: {e}"))?;
            drop(file);
            fs::rename(&temporary_path, path)
                .map_err(|e| format!("Could not replace project file: {e}"))?;
            if let Ok(directory) = File::open(parent) {
                let _ = directory.sync_all();
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| format!("Could not open project: {e}"))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| format!("Could not read project: {e}"))?;
        let mut value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Project file is not valid JSON: {e}"))?;
        migrate_project(&mut value)?;
        let document: Self = serde_json::from_value(value)
            .map_err(|e| format!("Project file has an invalid structure: {e}"))?;
        document.validate()?;
        Ok(document)
    }
}

fn migrate_project(value: &mut serde_json::Value) -> Result<(), String> {
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "Project file has no schema_version".to_string())? as u32;
    if value.get("format").and_then(serde_json::Value::as_str) != Some(PROJECT_FORMAT) {
        return Err("This is not a Project Saturn project file".into());
    }
    match version {
        PROJECT_SCHEMA_VERSION => Ok(()),
        1 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?;
            object.insert("schema_version".into(), 2.into());
            object.entry("view").or_insert_with(|| {
                serde_json::json!({
                    "active_sequence": "Sequence 1",
                    "timeline_zoom": 1.0,
                    "timeline_scroll": 0.0
                })
            });
            migrate_project(value)
        }
        2 => {
            value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?
                .insert("schema_version".into(), PROJECT_SCHEMA_VERSION.into());
            Ok(())
        }
        unsupported => Err(format!(
            "Project schema version {unsupported} is not supported"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_project_migrates_to_current_schema() {
        let mut legacy = serde_json::to_value(ProjectDocument::new("Legacy")).unwrap();
        legacy["schema_version"] = 1.into();
        legacy.as_object_mut().unwrap().remove("view");
        migrate_project(&mut legacy).unwrap();
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        assert_eq!(loaded.view.active_sequence.as_deref(), Some("Sequence 1"));
    }

    #[test]
    fn v2_project_migrates_with_default_edit_properties() {
        let mut legacy = serde_json::to_value(ProjectDocument::new("V2 project")).unwrap();
        legacy["schema_version"] = 2.into();
        for track in legacy["project"]["sequence"]["tracks"]
            .as_array_mut()
            .unwrap()
        {
            track.as_object_mut().unwrap().remove("gain_db");
        }
        legacy["project"]["sequence"]["tracks"][0]["clips"] = serde_json::json!([
            {"media_id": 3, "timeline_start": 0, "source_in": 0, "duration": 100}
        ]);
        migrate_project(&mut legacy).unwrap();
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        assert_eq!(loaded.project.sequence.tracks[0].gain_db, 0.0);
        let clip = &loaded.project.sequence.tracks[0].clips[0];
        assert_eq!(clip.color.exposure, 0.0);
        assert_eq!(clip.color.contrast, 1.0);
        assert!(clip.keyframes.is_empty());
    }

    #[test]
    fn project_saves_atomically_and_keeps_external_media_paths() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("cut.saturn");
        let mut doc = ProjectDocument::new("Cut");
        doc.project.media.push(MediaAsset {
            id: 4,
            name: "shot.mov".into(),
            path: PathBuf::from("/media/shot.mov"),
            kind: MediaKind::Video,
            duration: Some(Tick(42)),
            width: Some(1920),
            height: Some(1080),
        });
        doc.save_atomic(&path).unwrap();
        let loaded = ProjectDocument::load(&path).unwrap();
        assert_eq!(
            loaded.project.media[0].path,
            PathBuf::from("/media/shot.mov")
        );
        assert_eq!(loaded.project.media[0].duration, Some(Tick(42)));
    }
}
