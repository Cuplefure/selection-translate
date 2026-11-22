use crate::error::{AppError, AppResult};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const KEYRING_SERVICE: &str = "selection-translate";
const KEYRING_USER: &str = "api-key";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub provider: String,
    pub endpoint: String,
    pub model: String,
    pub source_lang: String,
    pub target_lang: String,
    pub auto_copy: bool,
    pub hotkey: String,
    pub cache_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: "mock".into(),
            endpoint: "https://api-free.deepl.com/v2/translate".into(),
            model: "gpt-4o-mini".into(),
            source_lang: "auto".into(),
            target_lang: "EN".into(),
            auto_copy: false,
            hotkey: "Alt+Shift+T".into(),
            cache_enabled: true,
        }
    }
}

fn config_path(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .resolve("settings.json", tauri::path::BaseDirectory::AppConfig)
        .map_err(|e| AppError::Config(e.to_string()))
}

pub fn load(app: &AppHandle) -> AppResult<Settings> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(Settings::default());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| AppError::Config(e.to_string()))?;
    serde_json::from_str(&raw).map_err(|e| AppError::Config(e.to_string()))
}

pub fn save(app: &AppHandle, settings: &Settings) -> AppResult<()> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::Config(e.to_string()))?;
    }
    let raw = serde_json::to_string_pretty(settings).map_err(|e| AppError::Config(e.to_string()))?;
    std::fs::write(path, raw).map_err(|e| AppError::Config(e.to_string()))
}

fn entry() -> AppResult<Entry> {
    Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| AppError::Config(e.to_string()))
}

pub fn set_api_key(key: &str) -> AppResult<()> {
    let item = entry()?;
    if key.trim().is_empty() {
        match item.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Config(e.to_string())),
        }
    } else {
        item.set_password(key.trim())
            .map_err(|e| AppError::Config(e.to_string()))
    }
}

pub fn has_api_key() -> AppResult<bool> {
    matches!(entry()?.get_password(), Ok(_))
}

pub fn api_key() -> AppResult<Option<String>> {
    match entry()?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Config(e.to_string())),
    }
}
