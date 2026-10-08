use saturn_core::{CommandRequest, EditorEngine, command_catalogue};
use serde_json::{Value, json};
use std::io::{self, BufRead};
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("saturn-cli: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        return usage();
    };
    let mut engine = EditorEngine::default();
    match command.as_str() {
        "commands" => {
            for spec in command_catalogue() {
                println!(
                    "{}\t{}\t{}",
                    spec.id,
                    spec.label,
                    spec.shortcut.unwrap_or("")
                );
            }
        }
        "control" => run_control()?,
        "new" => {
            let path = required(&mut args, "project path")?;
            let name = args.next().unwrap_or_else(|| "Untitled project".into());
            execute(&mut engine, "project.new", json!({"name": name}))?;
            execute(&mut engine, "file.save", json!({"path": path}))?;
            println!("Created {}", path.display());
        }
        "inspect" | "list-media" => {
            let path = required(&mut args, "project path")?;
            execute(&mut engine, "file.open", json!({"path": path}))?;
            let id = if command == "inspect" {
                "project.inspect"
            } else {
                "media.list"
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&execute(&mut engine, id, Value::Null)?).unwrap()
            );
        }
        "add-media" => {
            let project_path = required(&mut args, "project path")?;
            let media_path = required(&mut args, "media path")?;
            execute(&mut engine, "file.open", json!({"path": project_path}))?;
            let name = media_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let kind = media_kind(&media_path);
            execute(
                &mut engine,
                "project.add_media",
                json!({
                    "name": name,
                    "path": media_path,
                    "kind": kind
                }),
            )?;
            execute(&mut engine, "file.save", json!({"path": project_path}))?;
            println!("Added media to {}", project_path.display());
        }
        _ => return usage(),
    }
    Ok(())
}

/// JSON Lines control interface: one CommandRequest per input line, one result per output line.
fn run_control() -> Result<(), String> {
    let stdin = io::stdin();
    let mut engine = EditorEngine::default();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| format!("could not read control input: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<CommandRequest>(&line) {
            Ok(request) => match engine.execute(request) {
                Ok(result) => json!({"ok": true, "result": result}),
                Err(error) => json!({"ok": false, "error": error.to_string()}),
            },
            Err(error) => json!({
                "ok": false,
                "error": format!("invalid command request: {error}")
            }),
        };
        println!(
            "{}",
            serde_json::to_string(&response).map_err(|error| error.to_string())?
        );
    }
    Ok(())
}

fn execute(engine: &mut EditorEngine, id: &str, params: Value) -> Result<Value, String> {
    engine
        .execute(CommandRequest::new(id, params))
        .map_err(|e| e.to_string())
}

fn required(args: &mut impl Iterator<Item = String>, label: &str) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {label}"))
}

fn media_kind(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" => "image",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" => "audio",
        "mp4" | "mkv" | "mov" | "webm" | "avi" | "m4v" => "video",
        _ => "unknown",
    }
}

fn usage<T>() -> Result<T, String> {
    Err("usage: saturn-cli commands | control | new <project.saturn> [name] | inspect <project.saturn> | list-media <project.saturn> | add-media <project.saturn> <media-path>".into())
}
