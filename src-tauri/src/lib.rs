mod cache;
mod capture;
mod error;
mod platform;
mod preprocess;
mod settings;
mod translator;

use cache::Cache;
use error::{AppError, AppResult};
use serde::Serialize;
use settings::Settings;
use std::time::Instant;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder, WebviewUrl};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Timings {
    capture_ms: u128,
    translate_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    ok: bool,
    original: String,
    translated: String,
    source_lang: String,
    cached: bool,
    error: Option<String>,
    timings: Timings,
}

impl Payload {
    fn failure(message: String) -> Self {
        Self {
            ok: false,
            original: String::new(),
            translated: String::new(),
            source_lang: String::new(),
            cached: false,
            error: Some(message),
            timings: Timings { capture_ms: 0, translate_ms: 0 },
        }
    }
}

fn empty_timings() -> Timings {
    Timings { capture_ms: 0, translate_ms: 0 }
}

async fn do_translate(app: &AppHandle) -> AppResult<Payload> {
    let _ = app.emit("translate-start", ());

    let config = settings::load(app)?;
    let captured = capture::capture(&config)?;
    let prepared = preprocess::normalize(&captured.text)?;

    let cache = Cache::open(app)?;
    let key = cache::make_key(&prepared.source_lang, &config.target_lang, &prepared.text);
    let started = Instant::now();
    let api_key = settings::api_key()?;

    let mut cached = false;
    let translated = if config.cache_enabled {
        if let Some(hit) = cache.get(&key)? {
            cached = true;
            hit
        } else {
            let value = translator::translate(
                &config,
                api_key.as_deref(),
                &prepared.text,
                &prepared.source_lang,
            )
            .await?;
            cache.insert(&key, &value)?;
            value
        }
    } else {
        translator::translate(&config, api_key.as_deref(), &prepared.text, &prepared.source_lang).await?
    };

    let payload = Payload {
        ok: true,
        original: prepared.text,
        translated,
        source_lang: prepared.source_lang,
        cached,
        error: None,
        timings: Timings {
            capture_ms: captured.capture_ms,
            translate_ms: started.elapsed().as_millis(),
        },
    };
    let _ = app.emit("translate-result", &payload);
    show_card(app)?;
    Ok(payload)
}

fn show_card(app: &AppHandle) -> AppResult<()> {
    let window = app
        .get_webview_window("card")
        .ok_or_else(|| AppError::Config("卡片窗口未创建".into()))?;
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let size = monitor.size();
        let x = size.width.saturating_sub(420) as i32;
        let y = size.height.saturating_sub(340) as i32;
        let _ = window.set_position(tauri::PhysicalPosition { x, y });
    }
    window.show().map_err(|e| AppError::Config(e.to_string()))?;
    window.set_focus().map_err(|e| AppError::Config(e.to_string()))
}

fn open_settings_window(app: &AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|e| AppError::Config(e.to_string()))?;
        return window.set_focus().map_err(|e| AppError::Config(e.to_string()));
    }
    WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("index.html?view=settings".into()),
    )
    .title("设置")
    .inner_size(640.0, 560.0)
    .resizable(false)
    .build()
    .map_err(|e| AppError::Config(e.to_string()))?;
    Ok(())
}

fn register_hotkey(app: &AppHandle, hotkey: &str) -> AppResult<()> {
    app.global_shortcut()
        .register(hotkey)
        .map_err(|e| AppError::Config(format!("热键注册失败：{e}")))
}

fn setup_tray(app: &mut tauri::App) -> AppResult<()> {
    let translate = MenuItem::with_id(app, "translate", "翻译选中文字", true, None::<&str>)
        .map_err(|e| AppError::Config(e.to_string()))?;
    let settings_item =
        MenuItem::with_id(app, "settings", "设置", true, None::<&str>)
            .map_err(|e| AppError::Config(e.to_string()))?;
    let separator = PredefinedMenuItem::separator(app).map_err(|e| AppError::Config(e.to_string()))?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)
        .map_err(|e| AppError::Config(e.to_string()))?;
    let menu = Menu::with_items(app, &[&translate, &settings_item, &separator, &quit])
        .map_err(|e| AppError::Config(e.to_string()))?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
        .map_err(|e| AppError::Config(e.to_string()))?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "translate" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = do_translate(&handle).await;
                });
            }
            "settings" => {
                let _ = open_settings_window(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("card") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)
        .map_err(|e| AppError::Config(e.to_string()))?;
    Ok(())
}

#[tauri::command]
async fn translate_now(app: AppHandle) -> Result<Payload, String> {
    match do_translate(&app).await {
        Ok(payload) => Ok(payload),
        Err(error) => {
            let message = error.to_string();
            let payload = Payload::failure(message.clone());
            let _ = app.emit("translate-result", &payload);
            let _ = show_card(&app);
            Err(message)
        }
    }
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<Settings, String> {
    settings::load(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    settings::save(&app, &settings).map_err(|e| e.to_string())?;
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();
    register_hotkey(&app, &settings.hotkey).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_api_key(key: String) -> Result<(), String> {
    settings::set_api_key(&key).map_err(|e| e.to_string())
}

#[tauri::command]
fn has_api_key() -> Result<bool, String> {
    settings::has_api_key().map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_cache(app: AppHandle) -> Result<(), String> {
    let cache = Cache::open(&app).map_err(|e| e.to_string())?;
    cache.clear().map_err(|e| e.to_string())
}

#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    use arboard::Clipboard;
    Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
    open_settings_window(&app).map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = do_translate(&handle).await {
                                let _ = handle.emit("translate-result", &Payload::failure(error.to_string()));
                                let _ = show_card(&handle);
                            }
                        });
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            translate_now,
            get_settings,
            save_settings,
            set_api_key,
            has_api_key,
            clear_cache,
            copy_text,
            open_settings
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let config = settings::load(&handle)?;
            register_hotkey(&handle, &config.hotkey)?;
            setup_tray(app)?;

            WebviewWindowBuilder::new(
                &handle,
                "card",
                WebviewUrl::App("index.html".into()),
            )
            .title("划词翻译")
            .inner_size(400.0, 300.0)
            .resizable(false)
            .skip_taskbar(true)
            .visible(false)
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Selection Translate");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_payload_is_well_formed() {
        let payload = Payload::failure("boom".into());
        assert!(!payload.ok);
        assert_eq!(payload.error.as_deref(), Some("boom"));
        assert_eq!(empty_timings().capture_ms, 0);
    }
}
