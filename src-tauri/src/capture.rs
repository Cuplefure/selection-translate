use crate::error::{AppError, AppResult};
use crate::platform;
use crate::settings::Settings;
use arboard::Clipboard;
use std::time::Instant;

pub struct Captured {
    pub text: String,
    pub capture_ms: u128,
}

pub fn capture(settings: &Settings) -> AppResult<Captured> {
    let started = Instant::now();
    let mut clipboard = Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;

    let text = if settings.auto_copy {
        let backup = clipboard.get_text().ok();
        platform::send_ctrl_c()?;
        let selected = clipboard
            .get_text()
            .map_err(|e| AppError::Clipboard(e.to_string()))?;
        if let Some(original) = backup {
            let _ = clipboard.set_text(original);
        }
        selected
    } else {
        clipboard
            .get_text()
            .map_err(|e| AppError::Clipboard(e.to_string()))?
    };

    Ok(Captured {
        text,
        capture_ms: started.elapsed().as_millis(),
    })
}
