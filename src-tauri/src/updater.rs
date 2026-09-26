// Russian builds are updated manually; never install upstream binaries.
use serde::Serialize;
use tauri::AppHandle;
#[derive(Serialize, Clone)]
pub struct UpdateInfo { pub version: String, pub current_version: String, pub notes: Option<String> }
#[tauri::command]
pub async fn check_for_update(_app: AppHandle) -> Result<Option<UpdateInfo>, String> { Ok(None) }
#[tauri::command]
pub async fn install_update(_app: AppHandle) -> Result<(), String> { Err("Slate RU обновляется вручную из вашего форка.".into()) }
