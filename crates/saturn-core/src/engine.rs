use crate::command::{COMMANDS, CommandRequest, CommandStatus, status};
use crate::project::{
    Clip, ColorAdjustments, Keyframe, KeyframeProperty, MediaAsset, MediaKind, ProjectDocument,
    TrackKind,
};
use crate::time::{TICKS_PER_SECOND, Tick};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Debug)]
pub enum DispatchError {
    UnknownCommand(String),
    Disabled { id: String, reason: String },
    InvalidParameters(String),
    Operation(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCommand(id) => write!(f, "Unknown command `{id}`"),
            Self::Disabled { id, reason } => write!(f, "Command `{id}` is disabled: {reason}"),
            Self::InvalidParameters(error) | Self::Operation(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for DispatchError {}

#[derive(Clone)]
pub struct EditorEngine {
    document: ProjectDocument,
    current_path: Option<PathBuf>,
    undo: Vec<ProjectDocument>,
    redo: Vec<ProjectDocument>,
    saved_snapshot: String,
}

#[derive(Deserialize)]
struct NewProjectParams {
    #[serde(default = "default_project_name")]
    name: String,
}
fn default_project_name() -> String {
    "Untitled project".into()
}

#[derive(Deserialize)]
struct AddMediaParams {
    name: String,
    path: PathBuf,
    #[serde(default)]
    kind: crate::project::MediaKind,
    #[serde(default)]
    duration: Option<crate::time::Tick>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

#[derive(Deserialize)]
struct UpdateMediaParams {
    path: PathBuf,
    #[serde(default)]
    kind: crate::project::MediaKind,
    #[serde(default)]
    duration: Option<crate::time::Tick>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

#[derive(Deserialize)]
struct FilePathParams {
    path: PathBuf,
}

#[derive(Deserialize)]
struct AddClipParams {
    media_id: u64,
    #[serde(default)]
    track_id: Option<u64>,
    #[serde(default)]
    source_in: Tick,
    #[serde(default)]
    duration: Option<Tick>,
}

#[derive(Deserialize)]
struct RemoveClipParams {
    track_id: u64,
    clip_index: usize,
}

#[derive(Deserialize)]
struct MoveClipParams {
    track_id: u64,
    clip_index: usize,
    #[serde(default)]
    target_track_id: Option<u64>,
    timeline_start: Tick,
}

#[derive(Deserialize)]
struct SetColorParams {
    track_id: u64,
    clip_index: usize,
    exposure: f64,
    contrast: f64,
    saturation: f64,
}

#[derive(Deserialize)]
struct SetTrackGainParams {
    track_id: u64,
    gain_db: f64,
}

#[derive(Deserialize)]
struct SetKeyframeParams {
    track_id: u64,
    clip_index: usize,
    property: KeyframeProperty,
    time: Tick,
    value: f64,
}

#[derive(Deserialize)]
struct RemoveKeyframeParams {
    track_id: u64,
    clip_index: usize,
    property: KeyframeProperty,
    time: Tick,
}

impl Default for EditorEngine {
    fn default() -> Self {
        let document = ProjectDocument::default();
        let saved_snapshot = serde_json::to_string(&document).unwrap_or_default();
        Self {
            document,
            current_path: None,
            undo: Vec::new(),
            redo: Vec::new(),
            saved_snapshot,
        }
    }
}

impl EditorEngine {
    pub fn document(&self) -> &ProjectDocument {
        &self.document
    }
    pub fn current_path(&self) -> Option<&std::path::Path> {
        self.current_path.as_deref()
    }
    pub fn is_dirty(&self) -> bool {
        serde_json::to_string(&self.document)
            .map(|snapshot| snapshot != self.saved_snapshot)
            .unwrap_or(true)
    }

    pub fn command_status(&self, id: &str) -> Result<CommandStatus, DispatchError> {
        if !COMMANDS.iter().any(|command| command.id == id) {
            return Err(DispatchError::UnknownCommand(id.to_owned()));
        }
        let result = match id {
            "edit.undo" => status(!self.undo.is_empty(), "Nothing to undo"),
            "edit.redo" => status(!self.redo.is_empty(), "Nothing to redo"),
            _ => status(true, ""),
        };
        Ok(result)
    }

    pub fn execute(&mut self, request: CommandRequest) -> Result<Value, DispatchError> {
        let command = COMMANDS
            .iter()
            .find(|command| command.id == request.id.as_str())
            .ok_or_else(|| DispatchError::UnknownCommand(request.id.clone()))?;
        let availability = self.command_status(command.id)?;
        if !availability.enabled {
            return Err(DispatchError::Disabled {
                id: request.id,
                reason: availability.reason.unwrap_or_default(),
            });
        }

        match command.id {
            "project.new" => {
                let params: NewProjectParams = parse_params(request.params)?;
                self.checkpoint();
                self.document = ProjectDocument::new(params.name);
                self.current_path = None;
                Ok(json!({"name": self.document.project.name}))
            }
            "project.add_media" => {
                let params: AddMediaParams = parse_params(request.params)?;
                if let Some(existing) = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|item| item.path == params.path)
                {
                    return Ok(json!({"media_id": existing.id, "already_present": true}));
                }
                self.checkpoint();
                let id = self.document.project.next_id;
                self.document.project.next_id = id.saturating_add(1);
                let item = MediaAsset {
                    id,
                    name: params.name,
                    path: params.path,
                    kind: params.kind,
                    duration: params.duration,
                    width: params.width,
                    height: params.height,
                };
                self.document.project.media.push(item);
                Ok(json!({"media_id": id}))
            }
            "project.update_media" => {
                let params: UpdateMediaParams = parse_params(request.params)?;
                let Some(item) = self
                    .document
                    .project
                    .media
                    .iter_mut()
                    .find(|item| item.path == params.path)
                else {
                    return Ok(json!({"updated": false}));
                };
                item.kind = params.kind;
                item.duration = params.duration;
                item.width = params.width;
                item.height = params.height;
                Ok(json!({"updated": true, "media_id": item.id}))
            }
            "file.save" => {
                let params: FilePathParams = parse_params(request.params)?;
                self.document
                    .save_atomic(&params.path)
                    .map_err(DispatchError::Operation)?;
                self.current_path = Some(params.path.clone());
                self.saved_snapshot = serde_json::to_string(&self.document).unwrap_or_default();
                Ok(json!({"path": params.path}))
            }
            "file.open" => {
                let params: FilePathParams = parse_params(request.params)?;
                let loaded =
                    ProjectDocument::load(&params.path).map_err(DispatchError::Operation)?;
                self.document = loaded;
                self.current_path = Some(params.path.clone());
                self.undo.clear();
                self.redo.clear();
                self.saved_snapshot = serde_json::to_string(&self.document).unwrap_or_default();
                Ok(json!({"name": self.document.project.name, "path": params.path}))
            }
            "edit.undo" => {
                let previous = self.undo.pop().expect("availability checked");
                self.redo.push(self.document.clone());
                self.document = previous;
                Ok(json!({"undone": true}))
            }
            "edit.redo" => {
                let next = self.redo.pop().expect("availability checked");
                self.undo.push(self.document.clone());
                self.document = next;
                Ok(json!({"redone": true}))
            }
            "project.inspect" => serde_json::to_value(&self.document)
                .map_err(|e| DispatchError::Operation(e.to_string())),
            "project.status" => Ok(json!({
                "document": &self.document,
                "path": self.current_path.as_ref(),
                "is_dirty": self.is_dirty(),
            })),
            "media.list" => serde_json::to_value(&self.document.project.media)
                .map_err(|e| DispatchError::Operation(e.to_string())),
            "timeline.add_clip" => {
                let params: AddClipParams = parse_params(request.params)?;
                let media = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|item| item.id == params.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media item {} does not exist",
                            params.media_id
                        ))
                    })?;
                let default_kind = match &media.kind {
                    MediaKind::Audio => TrackKind::Audio,
                    _ => TrackKind::Video,
                };
                let media_duration = media.duration;
                let track_index = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|track| {
                        params
                            .track_id
                            .map(|track_id| track.id == track_id)
                            .unwrap_or_else(|| {
                                std::mem::discriminant(&track.kind)
                                    == std::mem::discriminant(&default_kind)
                            })
                    })
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("No matching timeline track exists".into())
                    })?;
                let duration = params
                    .duration
                    .or(media_duration)
                    .unwrap_or(Tick(TICKS_PER_SECOND * 5));
                if duration.0 <= 0 || params.source_in.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Clip source and duration must be positive".into(),
                    ));
                }
                if media_duration
                    .is_some_and(|length| params.source_in.0.saturating_add(duration.0) > length.0)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Clip range exceeds the media duration".into(),
                    ));
                }
                let track = &self.document.project.sequence.tracks[track_index];
                let timeline_start = track
                    .clips
                    .iter()
                    .filter_map(|clip| clip.timeline_start.0.checked_add(clip.duration.0))
                    .max()
                    .unwrap_or(0);
                self.checkpoint();
                let track = &mut self.document.project.sequence.tracks[track_index];
                track.clips.push(Clip {
                    media_id: params.media_id,
                    timeline_start: Tick(timeline_start),
                    source_in: params.source_in,
                    duration,
                    color: ColorAdjustments::default(),
                    keyframes: Vec::new(),
                });
                Ok(json!({"track_id": track.id, "clip_index": track.clips.len() - 1}))
            }
            "timeline.remove_clip" => {
                let params: RemoveClipParams = parse_params(request.params)?;
                let track_index = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|track| track.id == params.track_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Timeline track {} does not exist",
                            params.track_id
                        ))
                    })?;
                if params.clip_index
                    >= self.document.project.sequence.tracks[track_index]
                        .clips
                        .len()
                {
                    return Err(DispatchError::InvalidParameters(
                        "Timeline clip does not exist".into(),
                    ));
                }
                self.checkpoint();
                self.document.project.sequence.tracks[track_index]
                    .clips
                    .remove(params.clip_index);
                Ok(json!({"removed": true}))
            }
            "timeline.move_clip" => {
                let params: MoveClipParams = parse_params(request.params)?;
                if params.timeline_start.0 < 0 {
                    return Err(DispatchError::InvalidParameters(
                        "A clip cannot start before time zero".into(),
                    ));
                }
                let track_index = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .position(|track| track.id == params.track_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Timeline track {} does not exist",
                            params.track_id
                        ))
                    })?;
                let target_track_index = params
                    .target_track_id
                    .map(|target_id| {
                        self.document
                            .project
                            .sequence
                            .tracks
                            .iter()
                            .position(|track| track.id == target_id)
                            .ok_or_else(|| {
                                DispatchError::InvalidParameters(format!(
                                    "Timeline track {target_id} does not exist"
                                ))
                            })
                    })
                    .transpose()?
                    .unwrap_or(track_index);
                if params.clip_index
                    >= self.document.project.sequence.tracks[track_index]
                        .clips
                        .len()
                {
                    return Err(DispatchError::InvalidParameters(
                        "Timeline clip does not exist".into(),
                    ));
                }
                let media_id = self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index]
                    .media_id;
                let media = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|item| item.id == media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media item {media_id} does not exist"
                        ))
                    })?;
                let target_kind = &self.document.project.sequence.tracks[target_track_index].kind;
                let kind_matches = matches!(
                    (&media.kind, target_kind),
                    (MediaKind::Audio, TrackKind::Audio)
                        | (
                            MediaKind::Video | MediaKind::Image | MediaKind::Unknown,
                            TrackKind::Video
                        )
                );
                if !kind_matches {
                    return Err(DispatchError::InvalidParameters(
                        "Audio media must be placed on an audio track, and visual media on a video track".into(),
                    ));
                }
                self.checkpoint();
                let mut target_track_id = params.track_id;
                if track_index == target_track_index {
                    self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                        .timeline_start = params.timeline_start;
                } else {
                    let mut clip = self.document.project.sequence.tracks[track_index]
                        .clips
                        .remove(params.clip_index);
                    clip.timeline_start = params.timeline_start;
                    let target = &mut self.document.project.sequence.tracks[target_track_index];
                    target.clips.push(clip);
                    target.clips.sort_by_key(|item| item.timeline_start);
                    target_track_id = target.id;
                }
                let target = &mut self.document.project.sequence.tracks[target_track_index];
                if track_index == target_track_index {
                    target.clips.sort_by_key(|item| item.timeline_start);
                }
                let clip_index = target
                    .clips
                    .iter()
                    .position(|clip| {
                        clip.media_id == media_id && clip.timeline_start == params.timeline_start
                    })
                    .ok_or_else(|| {
                        DispatchError::Operation("Moved clip could not be found".into())
                    })?;
                Ok(json!({"moved": true, "track_id": target_track_id, "clip_index": clip_index}))
            }
            "clip.set_color" => {
                let params: SetColorParams = parse_params(request.params)?;
                if !params.exposure.is_finite()
                    || !(-5.0..=5.0).contains(&params.exposure)
                    || !params.contrast.is_finite()
                    || !(0.0..=2.0).contains(&params.contrast)
                    || !params.saturation.is_finite()
                    || !(0.0..=2.0).contains(&params.saturation)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Color values must be exposure -5..5, contrast 0..2, and saturation 0..2"
                            .into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                find_clip(&self.document, track_index, params.clip_index)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].clips[params.clip_index].color =
                    ColorAdjustments {
                        exposure: params.exposure,
                        contrast: params.contrast,
                        saturation: params.saturation,
                    };
                Ok(json!({"updated": true}))
            }
            "track.set_gain_db" => {
                let params: SetTrackGainParams = parse_params(request.params)?;
                if !params.gain_db.is_finite() || !(-60.0..=12.0).contains(&params.gain_db) {
                    return Err(DispatchError::InvalidParameters(
                        "Track gain must be between -60 and +12 dB".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].gain_db = params.gain_db;
                Ok(json!({"updated": true}))
            }
            "clip.set_keyframe" => {
                let params: SetKeyframeParams = parse_params(request.params)?;
                if !params.value.is_finite()
                    || !valid_keyframe_value(&params.property, params.value)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Keyframe value is outside the supported range".into(),
                    ));
                }
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                if params.time.0 < 0 || params.time.0 > clip.duration.0 {
                    return Err(DispatchError::InvalidParameters(
                        "Keyframe time must be within the selected clip".into(),
                    ));
                }
                self.checkpoint();
                let clip = &mut self.document.project.sequence.tracks[track_index].clips
                    [params.clip_index];
                if let Some(existing) = clip.keyframes.iter_mut().find(|keyframe| {
                    keyframe.property == params.property && keyframe.time == params.time
                }) {
                    existing.value = params.value;
                } else {
                    clip.keyframes.push(Keyframe {
                        property: params.property,
                        time: params.time,
                        value: params.value,
                    });
                    clip.keyframes.sort_by_key(|keyframe| keyframe.time);
                }
                Ok(json!({"updated": true, "keyframes": clip.keyframes.len()}))
            }
            "clip.remove_keyframe" => {
                let params: RemoveKeyframeParams = parse_params(request.params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let Some(index) = clip.keyframes.iter().position(|keyframe| {
                    keyframe.property == params.property && keyframe.time == params.time
                }) else {
                    return Ok(json!({"removed": false}));
                };
                self.checkpoint();
                self.document.project.sequence.tracks[track_index].clips[params.clip_index]
                    .keyframes
                    .remove(index);
                Ok(json!({"removed": true}))
            }
            _ => Err(DispatchError::UnknownCommand(request.id)),
        }
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.document.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
}

fn parse_params<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, DispatchError> {
    serde_json::from_value(value).map_err(|e| DispatchError::InvalidParameters(e.to_string()))
}

fn find_track_index(document: &ProjectDocument, track_id: u64) -> Result<usize, DispatchError> {
    document
        .project
        .sequence
        .tracks
        .iter()
        .position(|track| track.id == track_id)
        .ok_or_else(|| {
            DispatchError::InvalidParameters(format!("Timeline track {track_id} does not exist"))
        })
}

fn find_clip(
    document: &ProjectDocument,
    track_index: usize,
    clip_index: usize,
) -> Result<&Clip, DispatchError> {
    document.project.sequence.tracks[track_index]
        .clips
        .get(clip_index)
        .ok_or_else(|| DispatchError::InvalidParameters("Timeline clip does not exist".into()))
}

fn valid_keyframe_value(property: &KeyframeProperty, value: f64) -> bool {
    match property {
        KeyframeProperty::PositionX | KeyframeProperty::PositionY => {
            (-100_000.0..=100_000.0).contains(&value)
        }
        KeyframeProperty::Scale => (0.01..=100.0).contains(&value),
        KeyframeProperty::Rotation => (-36_000.0..=36_000.0).contains(&value),
        KeyframeProperty::Opacity => (0.0..=1.0).contains(&value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::MediaKind;

    #[test]
    fn commands_report_reasons_and_undo_redo_project_mutations() {
        let mut engine = EditorEngine::default();
        assert_eq!(
            engine
                .command_status("edit.undo")
                .unwrap()
                .reason
                .as_deref(),
            Some("Nothing to undo")
        );
        engine
            .execute(CommandRequest::new(
                "project.add_media",
                json!({
                    "name":"clip.mp4", "path":"/clips/clip.mp4", "kind": MediaKind::Video
                }),
            ))
            .unwrap();
        assert_eq!(engine.document().project.media.len(), 1);
        engine
            .execute(CommandRequest::new("edit.undo", Value::Null))
            .unwrap();
        assert!(engine.document().project.media.is_empty());
        engine
            .execute(CommandRequest::new("edit.redo", Value::Null))
            .unwrap();
        assert_eq!(engine.document().project.media[0].name, "clip.mp4");
    }
}
