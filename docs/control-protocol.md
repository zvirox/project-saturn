# Project Saturn command interface

The desktop app and `saturn-cli` use the same command IDs and `saturn-core::EditorEngine` dispatcher. UI actions, keyboard shortcuts, CLI operations, and automation should submit a `CommandRequest` instead of changing project state directly.

## JSON Lines control mode

Run `cargo run -p saturn-cli -- control`. The process reads one JSON object per input line and writes one JSON response per line. It keeps one in-memory project session for the lifetime of the process.

Request:

```json
{"id":"project.add_media","params":{"name":"shot.mp4","path":"/media/shot.mp4","kind":"video"}}
```

Success:

```json
{"ok":true,"result":{"media_id":3}}
```

Failure:

```json
{"ok":false,"error":"Command `edit.undo` is disabled: Nothing to undo"}
```

Useful commands include `project.new`, `file.open`, `file.save`, `project.add_media`, `project.update_media`, `edit.undo`, `edit.redo`, `project.inspect`, and `media.list`. `saturn-cli commands` prints IDs, labels, and shortcuts. The core exposes `command_status()` so clients can query whether Undo and Redo are enabled and why.

## Project files

Project files use the `.saturn` extension and JSON document format `project-saturn.project`. `schema_version` is migrated on load. Media is referenced by path; the file does not copy source media. Saves write and sync a temporary sibling file before atomically renaming it into place.

## Current limits

This control interface is local stdin/stdout JSON Lines, not a network daemon or MCP server. The project model and command envelope are serializable so another adapter can call the same dispatcher later. Media discovery currently happens in the GTK app; CLI media import classifies common formats by extension.
