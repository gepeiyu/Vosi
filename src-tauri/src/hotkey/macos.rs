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

pub fn spawn_listener(trigger_name: &str, tx: Sender<HotkeyEvent>) {
    let trigger_code = keycode_from_name(trigger_name);
    thread::spawn(move || {
        let pressed_at = RefCell::new(None::<Instant>);
        let event_tap = match CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::KeyDown, CGEventType::KeyUp],
            move |_proxy, event_type, event: &CGEvent| {
                if matches!(
                    event_type,
                    CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
                ) {
                    return CallbackResult::Keep;
                }

                let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                match event_type {
                    CGEventType::KeyDown if keycode == trigger_code => {
                        *pressed_at.borrow_mut() = Some(Instant::now());
                        let _ = tx.send(HotkeyEvent::Pressed);
                    }
                    CGEventType::KeyUp if keycode == trigger_code => {
                        if let Some(t0) = pressed_at.borrow_mut().take() {
                            if t0.elapsed().as_millis() >= 300 {
                                let _ = tx.send(HotkeyEvent::Released);
                            }
                        }
                    }
                    _ => {}
                }
                CallbackResult::Keep
            },
        ) {
            Ok(tap) => tap,
            Err(()) => {
                eprintln!(
                    "hotkey listener: failed to create CGEventTap — grant Accessibility permission"
                );
                return;
            }
        };

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
