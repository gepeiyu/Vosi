use crate::config::AppConfig;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

#[cfg(not(target_os = "macos"))]
use std::thread;

#[cfg(not(target_os = "macos"))]
use rdev::{listen, Event, EventType, Key};

pub enum HotkeyEvent {
    Pressed,
    Released,
}

pub fn spawn_hotkey_listener(config: Arc<RwLock<AppConfig>>, tx: Sender<HotkeyEvent>) {
    #[cfg(target_os = "macos")]
    {
        let trigger_name = config
            .read()
            .expect("config lock")
            .hotkey
            .trigger_key
            .clone();
        super::macos::spawn_listener(&trigger_name, tx);
    }

    #[cfg(not(target_os = "macos"))]
    {
        spawn_rdev_listener(config, tx);
    }
}

#[cfg(not(target_os = "macos"))]
fn spawn_rdev_listener(config: Arc<RwLock<AppConfig>>, tx: Sender<HotkeyEvent>) {
    thread::spawn(move || {
        let callback = move |event: Event| {
            let trigger = {
                let cfg = config.read().expect("config lock");
                key_from_name(&cfg.hotkey.trigger_key)
            };
            match event.event_type {
                EventType::KeyPress(key) if key == trigger => {
                    let _ = tx.send(HotkeyEvent::Pressed);
                }
                EventType::KeyRelease(key) if key == trigger => {
                    let _ = tx.send(HotkeyEvent::Released);
                }
                _ => {}
            }
        };
        if let Err(e) = listen(callback) {
            eprintln!("hotkey listener error: {e:?}");
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn key_from_name(name: &str) -> Key {
    match name {
        "RightAlt" => Key::AltGr,
        "LeftAlt" => Key::Alt,
        "RightCommand" | "RightMeta" | "MetaRight" => Key::MetaRight,
        "LeftCommand" | "LeftMeta" | "MetaLeft" => Key::MetaLeft,
        "RightCtrl" => Key::ControlRight,
        "LeftCtrl" => Key::ControlLeft,
        "RightShift" => Key::ShiftRight,
        "LeftShift" => Key::ShiftLeft,
        _ => Key::ControlRight,
    }
}
