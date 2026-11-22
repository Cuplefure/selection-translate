//! The only platform-sensitive module. Swap this implementation if your target
//! OS requires native accessibility APIs instead of synthetic key presses.

use crate::error::{AppError, AppResult};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::time::Duration;

pub fn send_ctrl_c() -> AppResult<()> {
    let mut keyboard = Enigo::new(&Settings::default())
        .map_err(|e| AppError::Platform(e.to_string()))?;
    keyboard
        .key(Key::Control, Direction::Press)
        .map_err(|e| AppError::Platform(e.to_string()))?;
    keyboard
        .key(Key::C, Direction::Click)
        .map_err(|e| AppError::Platform(e.to_string()))?;
    keyboard
        .key(Key::Control, Direction::Release)
        .map_err(|e| AppError::Platform(e.to_string()))?;
    std::thread::sleep(Duration::from_millis(40));
    Ok(())
}
