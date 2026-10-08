use gstreamer_pbutils::Discoverer;
use saturn_core::{CommandRequest, CommandStatus, EditorEngine, MediaKind, ProjectDocument, Tick, command_catalogue};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State, ipc::Channel};

#[derive(Clone, Default)]
struct AppState(Arc<Mutex<EditorEngine>>);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderProgressEvent {
    fraction_percent: u8,
    phase: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenverseAsset {
    id: String,
    title: String,
    media_url: String,
    source_url: String,
    thumbnail_url: Option<String>,
    creator: Option<String>,
    license: Option<String>,
    license_url: Option<String>,
}

#[derive(Deserialize)]
struct OpenverseResponse {
    #[serde(default)]
    results: Vec<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryMetadata {
    saved_at: u64,
    project_name: String,
    source_path: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MediaAvailability {
    media_id: u64,
    missing: bool,
}

fn recovery_paths(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let directory = app.path().app_data_dir()
        .map_err(|error| format!("Could not locate the app data folder: {error}"))?
        .join("recovery");
    Ok((directory.join("autosave.saturn"), directory.join("autosave.json")))
}

fn remove_recovery_files(app: &AppHandle) {
    if let Ok((project, metadata)) = recovery_paths(app) {
        let _ = std::fs::remove_file(project);
        let _ = std::fs::remove_file(metadata);
    }
}

fn save_recovery_metadata(path: &Path, metadata: &RecoveryMetadata) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(metadata).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|error| format!("Could not write recovery details: {error}"))?;
    std::fs::rename(&temporary, path).map_err(|error| format!("Could not finalize recovery details: {error}"))
}

fn autosave_once(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    if !engine.is_dirty() { return Ok(()); }
    let document = engine.document();
    let (project_path, metadata_path) = recovery_paths(app)?;
    let directory = project_path.parent().ok_or_else(|| "Recovery folder has no parent".to_string())?;
    std::fs::create_dir_all(directory).map_err(|error| format!("Could not create recovery folder: {error}"))?;
    document.save_atomic(&project_path)?;
    let metadata = RecoveryMetadata {
        saved_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        project_name: document.project.name.clone(),
        source_path: engine.current_path().map(|path| path.to_string_lossy().into_owned()),
    };
    save_recovery_metadata(&metadata_path, &metadata)
}

fn start_autosave(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            if let Err(error) = autosave_once(&app, &state) {
                eprintln!("Autosave failed: {error}");
            }
        }
    });
}

#[tauri::command]
fn execute_command(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CommandRequest,
) -> Result<Value, String> {
    let command_id = request.id.clone();
    let result = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?
        .execute(request)
        .map_err(|error| error.to_string())?;

    if matches!(command_id.as_str(), "file.open" | "project.new" | "file.save") {
        remove_recovery_files(&app);
    }
    let mutating = command_catalogue().iter().find(|command| command.id == command_id)
        .is_some_and(|command| command.mutating);
    if mutating && !matches!(command_id.as_str(), "file.open" | "file.save") {
        let is_dirty = state.0.lock().map_err(|_| "Editor state is unavailable")?.is_dirty();
        if is_dirty {
            let autosave_app = app.clone();
            let autosave_state = AppState(Arc::clone(&state.0));
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(error) = autosave_once(&autosave_app, &autosave_state) {
                    eprintln!("Autosave failed: {error}");
                }
            });
        } else {
            remove_recovery_files(&app);
        }
    }
    if matches!(command_id.as_str(), "file.open" | "project.add_media" | "project.relink_media") {
        let state = state.0.lock().map_err(|_| "Editor state is unavailable")?;
        for media in &state.document().project.media {
            let _ = app.asset_protocol_scope().allow_file(&media.path);
        }
    }
    Ok(result)
}

#[tauri::command]
fn recovery_status(app: AppHandle) -> Result<Value, String> {
    let (project_path, metadata_path) = recovery_paths(&app)?;
    if !project_path.is_file() {
        return Ok(json!({"available": false}));
    }
    let document = ProjectDocument::load(&project_path)?;
    let metadata = std::fs::read(&metadata_path).ok()
        .and_then(|bytes| serde_json::from_slice::<RecoveryMetadata>(&bytes).ok());
    let modified = std::fs::metadata(&project_path).ok().and_then(|item| item.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map(|duration| duration.as_secs());
    Ok(json!({
        "available": true,
        "projectName": metadata.as_ref().map(|item| item.project_name.clone()).unwrap_or(document.project.name),
        "savedAt": metadata.map(|item| item.saved_at).or(modified).unwrap_or_default(),
    }))
}

#[tauri::command]
fn restore_autosave(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let (project_path, metadata_path) = recovery_paths(&app)?;
    let document = ProjectDocument::load(&project_path)?;
    let metadata = std::fs::read(metadata_path).ok()
        .and_then(|bytes| serde_json::from_slice::<RecoveryMetadata>(&bytes).ok());
    let source_path = metadata.and_then(|item| item.source_path).map(PathBuf::from);
    state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?
        .restore_recovery(document, source_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn discard_autosave(app: AppHandle) {
    remove_recovery_files(&app);
}

#[tauri::command]
fn check_media_availability(state: State<'_, AppState>) -> Result<Vec<MediaAvailability>, String> {
    let engine = state.0.lock().map_err(|_| "Editor state is unavailable".to_string())?;
    Ok(engine.document().project.media.iter().map(|media| MediaAvailability {
        media_id: media.id,
        missing: !media.path.is_file(),
    }).collect())
}

#[tauri::command]
fn command_status(state: State<'_, AppState>, id: String) -> Result<CommandStatus, String> {
    state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?
        .command_status(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_command_catalogue() -> Vec<saturn_core::CommandSpec> {
    command_catalogue().to_vec()
}

#[tauri::command]
async fn import_media(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Value, String> {
    let engine = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        gstreamer::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
        let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(30))
            .map_err(|error| format!("Could not create media scanner: {error}"))?;
        let mut discovered = Vec::new();

        for selected_path in paths {
            let path = PathBuf::from(&selected_path);
            if !path.is_file() {
                continue;
            }
            let name = path
                .file_name()
                .map(|part| part.to_string_lossy().into_owned())
                .unwrap_or_else(|| selected_path.clone());

            let metadata = gstreamer::glib::filename_to_uri(&path, None)
                .ok()
                .and_then(|uri| discoverer.discover_uri(&uri).ok())
                .map(|info| {
                    let videos = info.video_streams();
                    let audios = info.audio_streams();
                    let kind = if let Some(video) = videos.first() {
                        if video.is_image() {
                            MediaKind::Image
                        } else {
                            MediaKind::Video
                        }
                    } else if !audios.is_empty() {
                        MediaKind::Audio
                    } else {
                        MediaKind::Unknown
                    };
                    let duration = info
                        .duration()
                        .and_then(|duration| Tick::from_nanoseconds(duration.nseconds()));
                    let width = videos.first().map(|video| video.width());
                    let height = videos.first().map(|video| video.height());
                    (kind, duration, width, height)
                })
                .unwrap_or((MediaKind::Unknown, None, None, None));

            discovered.push((name, path, metadata));
        }

        let mut engine_guard = engine
            .lock()
            .map_err(|_| "Editor state is unavailable".to_string())?;
        for (name, path, (kind, duration, width, height)) in discovered {
            let params = json!({
                "name": name,
                "path": path,
                "kind": kind,
                "duration": duration,
                "width": width,
                "height": height,
            });
            engine_guard
                .execute(CommandRequest::new("project.add_media", params))
                .map_err(|error| error.to_string())?;
            let _ = app.asset_protocol_scope().allow_file(&path);
        }

        let status = engine_guard
            .execute(CommandRequest::new("project.status", Value::Null))
            .map_err(|error| error.to_string())?;
        drop(engine_guard);
        if let Err(error) = autosave_once(&app, &AppState(Arc::clone(&engine))) {
            eprintln!("Autosave after media import failed: {error}");
        }
        Ok(status)
    })
    .await
    .map_err(|error| format!("Media import task failed: {error}"))?
}

#[tauri::command]
async fn start_export(
    state: State<'_, AppState>,
    output_path: String,
    use_mark_range: bool,
    on_progress: Channel<RenderProgressEvent>,
) -> Result<(), String> {
    let document = state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?
        .document()
        .clone();
    let range = if use_mark_range {
        match (document.project.sequence.mark_in, document.project.sequence.mark_out) {
            (Some(start), Some(end)) => Some(saturn_render::RenderRange { start, end }),
            _ => return Err("Set both In and Out points before exporting the marked range".into()),
        }
    } else {
        None
    };
    tauri::async_runtime::spawn_blocking(move || {
        saturn_render::render_project_range(&document, Path::new(&output_path), range, |progress| {
            let _ = on_progress.send(RenderProgressEvent {
                fraction_percent: progress.fraction_percent,
                phase: progress.phase.to_string(),
            });
        })
    })
    .await
    .map_err(|error| format!("Render task failed: {error}"))?
}

#[tauri::command]
async fn search_openverse(
    query: String,
    media_type: String,
) -> Result<Vec<OpenverseAsset>, String> {
    let query = query.trim().to_owned();
    if query.is_empty() || query.chars().count() > 200 {
        return Err("Enter a search phrase up to 200 characters".into());
    }
    let endpoint = match media_type.as_str() {
        "images" => "https://api.openverse.org/v1/images/",
        "audio" => "https://api.openverse.org/v1/audio/",
        _ => return Err("Unsupported asset type".into()),
    };
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .map_err(|error| format!("Could not create API client: {error}"))?
        .get(endpoint)
        .query(&[("q", query), ("page_size", "24".into())])
        .send()
        .await
        .map_err(|error| format!("Openverse search failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Openverse search returned an error: {error}"))?
        .json::<OpenverseResponse>()
        .await
        .map_err(|error| format!("Could not read Openverse results: {error}"))?;
    Ok(response
        .results
        .into_iter()
        .filter_map(|item| {
            let media_url = item.get("url")?.as_str()?.to_owned();
            Some(OpenverseAsset {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                title: item
                    .get("title")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Untitled media")
                    .to_owned(),
                media_url,
                source_url: item
                    .get("foreign_landing_url")
                    .or_else(|| item.get("detail_url"))?
                    .as_str()?
                    .to_owned(),
                thumbnail_url: item
                    .get("thumbnail")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                creator: item
                    .get("creator")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                license: item
                    .get("license")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                license_url: item
                    .get("license_url")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            })
        })
        .collect())
}

#[tauri::command]
async fn download_openverse_asset(app: AppHandle, asset: OpenverseAsset) -> Result<String, String> {
    let url = reqwest::Url::parse(&asset.media_url)
        .map_err(|error| format!("Invalid media URL: {error}"))?;
    if url.scheme() != "https" {
        return Err("Only HTTPS media downloads are allowed".into());
    }
    let mut response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|error| error.to_string())?
        .get(url.clone())
        .send()
        .await
        .map_err(|error| format!("Media download failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Media provider returned an error: {error}"))?;
    if response.url().scheme() != "https" {
        return Err("Media provider redirected to a non-HTTPS URL".into());
    }
    if response
        .content_length()
        .is_some_and(|length| length > 512 * 1024 * 1024)
    {
        return Err("This asset is larger than Easy Edit Pro's 512 MiB download limit".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Media download failed: {error}"))?
    {
        if bytes.len().saturating_add(chunk.len()) > 512 * 1024 * 1024 {
            return Err("This asset is larger than Easy Edit Pro's 512 MiB download limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate the app data folder: {error}"))?
        .join("assets");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create asset folder: {error}"))?;
    let extension = url
        .path_segments()
        .and_then(|mut parts| parts.next_back())
        .and_then(|name| name.rsplit_once('.').map(|(_, ext)| ext))
        .filter(|ext| ext.len() <= 8 && ext.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("bin");
    let title = asset
        .title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let mut target = directory.join(format!("{}.{}", title.trim_matches('-'), extension));
    let mut suffix = 1;
    while target.exists() {
        target = directory.join(format!("{}-{suffix}.{extension}", title.trim_matches('-')));
        suffix += 1;
    }
    std::fs::write(&target, &bytes)
        .map_err(|error| format!("Could not save downloaded asset: {error}"))?;
    let attribution_path = target.with_extension(format!("{extension}.attribution.json"));
    let attribution = json!({
        "provider": "Openverse",
        "title": asset.title,
        "creator": asset.creator,
        "license": asset.license,
        "licenseUrl": asset.license_url,
        "sourceUrl": asset.source_url,
        "mediaUrl": asset.media_url
    });
    std::fs::write(
        attribution_path,
        serde_json::to_vec_pretty(&attribution).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("Could not save asset attribution: {error}"))?;
    let _ = app.asset_protocol_scope().allow_file(&target);
    Ok(target.to_string_lossy().into_owned())
}

#[tauri::command]
async fn ask_local_ai(prompt: String, model: String) -> Result<String, String> {
    let prompt = prompt.trim().to_owned();
    let model = model.trim().to_owned();
    if prompt.is_empty() || prompt.chars().count() > 12_000 {
        return Err("Enter a prompt up to 12,000 characters".into());
    }
    if model.is_empty() || model.chars().count() > 100 {
        return Err("Enter a local Ollama model name".into());
    }
    let response: Value = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|error| error.to_string())?
        .post("http://127.0.0.1:11434/api/chat")
        .json(&json!({"model": model, "messages": [{"role": "user", "content": prompt}], "stream": false}))
        .send()
        .await
        .map_err(|error| format!("Could not connect to local Ollama. Start Ollama and download a model first. ({error})"))?
        .error_for_status()
        .map_err(|error| format!("Ollama returned an error: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Could not read the Ollama response: {error}"))?;
    response
        .pointer("/message/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "Ollama returned no assistant text".to_string())
}

fn main() {
    let state = AppState::default();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state.clone())
        .setup(move |app| {
            start_autosave(app.handle().clone(), state.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            execute_command,
            command_status,
            get_command_catalogue,
            recovery_status,
            restore_autosave,
            discard_autosave,
            check_media_availability,
            import_media,
            start_export,
            search_openverse,
            download_openverse_asset,
            ask_local_ai
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Easy Edit Pro");
}
