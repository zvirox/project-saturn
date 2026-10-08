use saturn_core::{CommandRequest, EditorEngine};
use serde_json::Value;
use std::sync::Mutex;
use tauri::State;

#[derive(Default)]
struct AppState(Mutex<EditorEngine>);

#[tauri::command]
fn execute_command(state: State<'_, AppState>, request: CommandRequest) -> Result<Value, String> {
    state
        .0
        .lock()
        .map_err(|_| "Editor state is unavailable".to_string())?
        .execute(request)
        .map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![execute_command])
        .run(tauri::generate_context!())
        .expect("failed to run Project Saturn Tauri app");
}
