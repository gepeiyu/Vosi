use super::listener::HotkeyEvent;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, CallbackResult, EventField, CGEventFlags,
};
use std::cell::RefCell;
use std::sync::mpsc::Sender;
use std::thread;

/// macOS virtual key codes (same as rdev / HIToolbox).
fn keycodes_for_trigger(name: &str) -> Vec<i64> {
    match name {
        // Accept both command keys — users often press the left one.
        "RightCommand" | "RightMeta" | "MetaRight" | "LeftCommand" | "LeftMeta" | "MetaLeft" => {
            vec![54, 55]
        }
        "RightAlt" => vec![61],
        "LeftAlt" => vec![58],
        "RightCtrl" => vec![62],
        "LeftCtrl" => vec![59],
        "RightShift" => vec![60],
        "LeftShift" => vec![56],
        _ => vec![54, 55],
    }
}

pub fn spawn_listener(trigger_name: &str, tx: Sender<HotkeyEvent>) {
    let trigger_codes = keycodes_for_trigger(trigger_name);
    let trigger_label = trigger_name.to_string();
    let codes_log = trigger_codes.clone();
    thread::spawn(move || {
        let is_held = RefCell::new(false);
        let last_flags = RefCell::new(CGEventFlags::CGEventFlagNull);

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

                let flags = event.get_flags();
                let prev = *last_flags.borrow();
                let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                if !trigger_codes.contains(&keycode) {
                    *last_flags.borrow_mut() = flags;
                    return CallbackResult::Keep;
                }

                // Same edge detection as rdev (FlagsChanged branch in macos/common.rs).
                let is_release = flags < prev;
                *last_flags.borrow_mut() = flags;
                let mut held = is_held.borrow_mut();

                if is_release {
                    if *held {
                        *held = false;
                        let _ = tx.send(HotkeyEvent::Released);
                    }
                } else if !*held {
                    *held = true;
                    let _ = tx.send(HotkeyEvent::Pressed);
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
            "hotkey listener ready: {trigger_label} (keycodes {codes_log:?}) via FlagsChanged"
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
