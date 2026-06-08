use super::listener::HotkeyEvent;
use crate::permissions::microphone_macos::{hotkey_set_keycode, hotkey_start, hotkey_stop};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

static HOTKEY_TX: OnceLock<Mutex<Option<Sender<HotkeyEvent>>>> = OnceLock::new();

extern "C" fn on_hotkey_event(event_type: i32) {
    let event = if event_type == 0 {
        HotkeyEvent::Pressed
    } else {
        HotkeyEvent::Released
    };
    if let Some(tx) = HOTKEY_TX
        .get()
        .and_then(|slot| slot.lock().ok())
        .and_then(|guard| guard.as_ref().cloned())
    {
        let _ = tx.send(event);
    }
}

fn keycode_for_trigger(name: &str) -> u16 {
    match name {
        "RightCommand" | "RightMeta" | "MetaRight" => 54,
        "LeftCommand" | "LeftMeta" | "MetaLeft" => 55,
        "RightAlt" => 61,
        "LeftAlt" => 58,
        "RightCtrl" => 62,
        "LeftCtrl" => 59,
        "RightShift" => 60,
        "LeftShift" => 56,
        _ => 54,
    }
}

pub fn spawn_listener(trigger_name: &str, tx: Sender<HotkeyEvent>) {
    let trigger_label = trigger_name.to_string();
    let keycode = keycode_for_trigger(trigger_name);
    let _ = HOTKEY_TX.set(Mutex::new(Some(tx)));

    thread::spawn(move || {
        hotkey_set_keycode(keycode);
        loop {
            if hotkey_start(on_hotkey_event) {
                eprintln!(
                    "hotkey listener ready: {trigger_label} (keycode {keycode}) via NSEvent global monitor"
                );
                break;
            }
            eprintln!(
                "hotkey listener: waiting for Accessibility permission — \
                 enable Vosi in System Settings → Privacy & Security → Accessibility"
            );
            thread::sleep(Duration::from_secs(2));
        }

        loop {
            thread::sleep(Duration::from_secs(3600));
        }
    });
}

#[allow(dead_code)]
pub fn stop_listener() {
    hotkey_stop();
}
