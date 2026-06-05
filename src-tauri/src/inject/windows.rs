use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

use super::{InjectMethod, TextInjector};

pub struct WinInjector;

impl TextInjector for WinInjector {
    fn inject(&self, text: &str, method: InjectMethod) -> Result<(), String> {
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        match method {
            InjectMethod::Type => {
                enigo.text(text).map_err(|e| e.to_string())?;
                Ok(())
            }
            InjectMethod::Paste => {
                arboard::Clipboard::new()
                    .map_err(|e| e.to_string())?
                    .set_text(text)
                    .map_err(|e| e.to_string())?;
                thread::sleep(Duration::from_millis(50));
                enigo.key(Key::Control, Direction::Press).map_err(|e| e.to_string())?;
                enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| e.to_string())?;
                enigo.key(Key::Control, Direction::Release).map_err(|e| e.to_string())?;
                Ok(())
            }
        }
    }
}
