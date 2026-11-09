//! Desktop-only settings that the tray menu toggles: close to tray, launch at
//! login and start hidden at login. Stored as JSON in the app config dir.

use std::{fs, path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_autostart::ManagerExt;

/// Passed by the login item so a launch at login can be told apart.
pub const AUTOSTART_ARG: &str = "--autostart";

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Closing the window hides it instead of quitting.
    pub close_to_tray: bool,
    /// When launched at login, stay in the tray instead of opening the window.
    pub start_hidden: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            start_hidden: false,
        }
    }
}

fn path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join("desktop-settings.json"))
}

/// Load the settings from disk into app state.
pub fn init<R: Runtime>(app: &AppHandle<R>) {
    let settings: Settings = path(app)
        .and_then(|p| fs::read(p).ok())
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default();
    app.manage(Mutex::new(settings));
}

pub fn get<R: Runtime>(app: &AppHandle<R>) -> Settings {
    app.state::<Mutex<Settings>>().lock().unwrap().clone()
}

/// Change the settings and write them to disk.
pub fn update<R: Runtime>(app: &AppHandle<R>, change: impl FnOnce(&mut Settings)) {
    let state = app.state::<Mutex<Settings>>();
    let mut settings = state.lock().unwrap();
    change(&mut settings);
    let Some(path) = path(app) else { return };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(data) = serde_json::to_vec_pretty(&*settings) {
        let _ = fs::write(path, data);
    }
}

/// Whether the window should stay hidden on this launch.
pub fn launch_hidden<R: Runtime>(app: &AppHandle<R>) -> bool {
    get(app).start_hidden && std::env::args().any(|arg| arg == AUTOSTART_ARG)
}

/// The login item would point at the binary inside the sandbox, so launch at
/// login is not offered in Flatpak.
pub fn autostart_supported() -> bool {
    !(cfg!(target_os = "linux") && std::path::Path::new("/.flatpak-info").exists())
}

pub fn autostart_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    autostart_supported() && app.autolaunch().is_enabled().unwrap_or(false)
}

/// Turn launch at login on or off. Returns the resulting state.
pub fn set_autostart<R: Runtime>(app: &AppHandle<R>, enable: bool) -> bool {
    let autolaunch = app.autolaunch();
    let _ = if enable {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    autostart_enabled(app)
}
