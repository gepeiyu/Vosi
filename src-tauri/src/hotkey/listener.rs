use rdev::{listen, Event, EventType, Key};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Instant;

pub enum HotkeyEvent {
    Pressed,
    Released,
}

pub fn spawn_hotkey_listener(trigger: Key, tx: Sender<HotkeyEvent>) {
    thread::spawn(move || {
        let mut pressed_at: Option<Instant> = None;
        let callback = move |event: Event| {
            match event.event_type {
                EventType::KeyPress(key) if key == trigger => {
                    pressed_at = Some(Instant::now());
                    let _ = tx.send(HotkeyEvent::Pressed);
                }
                EventType::KeyRelease(key) if key == trigger => {
                    if let Some(t0) = pressed_at.take() {
                        if t0.elapsed().as_millis() >= 300 {
                            let _ = tx.send(HotkeyEvent::Released);
                        }
                    }
                }
                _ => {}
            }
        };
        if let Err(e) = listen(callback) {
            eprintln!("hotkey listener error: {e:?}");
        }
    });
}

pub fn key_from_name(name: &str) -> Key {
    match name {
        "RightAlt" => Key::AltGr,
        "LeftAlt" => Key::Alt,
        "RightCtrl" => Key::ControlRight,
        "LeftCtrl" => Key::ControlLeft,
        "RightShift" => Key::ShiftRight,
        "LeftShift" => Key::ShiftLeft,
        _ => Key::AltGr,
    }
}
