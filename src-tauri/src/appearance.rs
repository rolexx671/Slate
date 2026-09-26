//! Per-application appearance; never changes the macOS appearance preference.
use tauri::{AppHandle, Manager, Theme};

fn path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_data_dir().map(|dir| dir.join("appearance"))
        .map_err(|e| e.to_string())
}

fn theme(value: &str) -> Result<Option<Theme>, String> {
    match value {
        "dark" => Ok(Some(Theme::Dark)),
        "light" => Ok(Some(Theme::Light)),
        "system" => Ok(None),
        _ => Err("Неизвестная тема оформления.".into()),
    }
}

pub fn restore(app: &AppHandle) {
    let preference = path(app).ok().and_then(|p| std::fs::read_to_string(p).ok());
    app.set_theme(theme(preference.as_deref().unwrap_or("dark")).unwrap_or(Some(Theme::Dark)));
}

#[tauri::command]
pub fn set_native_theme(app: AppHandle, theme: String) -> Result<(), String> {
    let selected = self::theme(&theme)?;
    let path = path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, theme).map_err(|e| e.to_string())?;
    app.set_theme(selected);
    Ok(())
}
