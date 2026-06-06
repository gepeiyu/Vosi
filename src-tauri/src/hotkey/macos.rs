use super::listener::HotkeyEvent;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, CallbackResult, EventField,
};
use std::cell::RefCell;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Instant;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
}

const HID_SYSTEM_STATE: i32 = 1;

pub fn spawn_listener(trigger_name: &str, tx: Sender<HotkeyEvent>) {
    let trigger_code = keycode_from_name(trigger_name);
    let trigger_label = trigger_name.to_string();
    thread::spawn(move || {
        let pressed_at = RefCell::new(None::<Instant>);
        let is_held = RefCell::new(false);
        let event_tap = match CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::FlagsChanged],
            move |_proxy, event_type, event: &CGEvent| {
                if matches!(
                    event_type,
                    CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
                ) {
                    return CallbackResult::Keep;
                }

                let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                if keycode != trigger_code {
                    return CallbackResult::Keep;
                }

                let down = is_key_pressed(trigger_code);
                let mut held = is_held.borrow_mut();
                if down && !*held {
                    *held = true;
                    *pressed_at.borrow_mut() = Some(Instant::now());
                    let _ = tx.send(HotkeyEvent::Pressed);
                } else if !down && *held {
                    *held = false;
                    if let Some(t0) = pressed_at.borrow_mut().take() {
                        if t0.elapsed().as_millis() >= 300 {
                            let _ = tx.send(HotkeyEvent::Released);
                        }
                    }
                }
                CallbackResult::Keep
            },
        ) {
            Ok(tap) => tap,
            Err(()) => {
                eprintln!(
                    "hotkey listener: failed to create CGEventTap — add tauri-app to \
                     System Settings → Privacy & Security → Accessibility"
                );
                return;
            }
        };

        eprintln!(
            "hotkey listener ready: {trigger_label} (keycode {trigger_code:#x}) via FlagsChanged"
        );

        event_tap.enable();
        unsafe {
            let source = event_tap
                .mach_port()
                .create_runloop_source(0)
                .expect("runloop source");
            CFRunLoop::get_current().add_source(&source, kCFRunLoopCommonModes);
            CFRunLoop::run_current();
        }
    });
}

fn is_key_pressed(keycode: i64) -> bool {
    unsafe { CGEventSourceKeyState(HID_SYSTEM_STATE, keycode as u16) }
}

fn keycode_from_name(name: &str) -> i64 {
    match name {
        "RightCommand" | "RightMeta" | "MetaRight" => 0x36,
        "LeftCommand" | "LeftMeta" | "MetaLeft" => 0x37,
        "RightAlt" => 0x3D,
        "LeftAlt" => 0x3A,
        "RightCtrl" => 0x3E,
        "LeftCtrl" => 0x3B,
        "RightShift" => 0x3C,
        "LeftShift" => 0x38,
        _ => 0x36,
    }
}
