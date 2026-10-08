use crate::command::{COMMANDS, CommandRequest, CommandStatus, status};
use crate::project::{MediaAsset, ProjectDocument};
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
            "media.list" => serde_json::to_value(&self.document.project.media)
                .map_err(|e| DispatchError::Operation(e.to_string())),
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
