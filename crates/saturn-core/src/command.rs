use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandRequest {
    pub id: String,
    #[serde(default)]
    pub params: Value,
}

impl CommandRequest {
    pub fn new(id: impl Into<String>, params: Value) -> Self {
        Self {
            id: id.into(),
            params,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub menu: &'static [&'static str],
    pub shortcut: Option<&'static str>,
    pub mutating: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandStatus {
    pub enabled: bool,
    pub reason: Option<String>,
}

pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: "project.new",
        label: "New Project",
        menu: &["File"],
        shortcut: Some("Ctrl+N"),
        mutating: true,
    },
    CommandSpec {
        id: "file.open",
        label: "Open Project",
        menu: &["File"],
        shortcut: Some("Ctrl+O"),
        mutating: true,
    },
    CommandSpec {
        id: "file.save",
        label: "Save Project",
        menu: &["File"],
        shortcut: Some("Ctrl+S"),
        mutating: false,
    },
    CommandSpec {
        id: "project.add_media",
        label: "Import Media",
        menu: &["File", "Import Media"],
        shortcut: Some("Ctrl+I"),
        mutating: true,
    },
    CommandSpec {
        id: "project.update_media",
        label: "Update Media Metadata",
        menu: &[],
        shortcut: None,
        mutating: true,
    },
    CommandSpec {
        id: "edit.undo",
        label: "Undo",
        menu: &["Edit"],
        shortcut: Some("Ctrl+Z"),
        mutating: true,
    },
    CommandSpec {
        id: "edit.redo",
        label: "Redo",
        menu: &["Edit"],
        shortcut: Some("Ctrl+Shift+Z"),
        mutating: true,
    },
    CommandSpec {
        id: "project.inspect",
        label: "Inspect Project",
        menu: &[],
        shortcut: None,
        mutating: false,
    },
    CommandSpec {
        id: "project.status",
        label: "Project Status",
        menu: &[],
        shortcut: None,
        mutating: false,
    },
    CommandSpec {
        id: "media.list",
        label: "List Media",
        menu: &[],
        shortcut: None,
        mutating: false,
    },
    CommandSpec {
        id: "timeline.add_clip",
        label: "Add Clip to Timeline",
        menu: &["Timeline"],
        shortcut: None,
        mutating: true,
    },
    CommandSpec {
        id: "timeline.remove_clip",
        label: "Remove Clip",
        menu: &["Timeline"],
        shortcut: None,
        mutating: true,
    },
    CommandSpec {
        id: "timeline.move_clip",
        label: "Move Clip",
        menu: &["Timeline"],
        shortcut: None,
        mutating: true,
    },
    CommandSpec {
        id: "clip.set_color",
        label: "Set Clip Color",
        menu: &["Color"],
        shortcut: None,
        mutating: true,
    },
    CommandSpec {
        id: "track.set_gain_db",
        label: "Set Track Gain",
        menu: &["Audio"],
        shortcut: None,
        mutating: true,
    },
    CommandSpec {
        id: "clip.set_keyframe",
        label: "Set Keyframe",
        menu: &["Keyframes"],
        shortcut: None,
        mutating: true,
    },
    CommandSpec {
        id: "clip.remove_keyframe",
        label: "Remove Keyframe",
        menu: &["Keyframes"],
        shortcut: None,
        mutating: true,
    },
];

pub fn command_catalogue() -> &'static [CommandSpec] {
    COMMANDS
}

pub fn status(enabled: bool, reason: impl Into<String>) -> CommandStatus {
    CommandStatus {
        enabled,
        reason: (!enabled).then(|| reason.into()),
    }
}
