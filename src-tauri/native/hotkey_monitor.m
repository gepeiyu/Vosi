#import <AppKit/AppKit.h>
#include <dispatch/dispatch.h>
#include <stdbool.h>
#include <stdint.h>

typedef void (*VosiHotkeyFn)(int event_type);

static id s_monitor = nil;
static bool s_held = false;
static uint16_t s_keycode = 54;
static NSEventModifierFlags s_flag = NSEventModifierFlagCommand;
static VosiHotkeyFn s_callback = NULL;

static NSEventModifierFlags flag_for_keycode(uint16_t keycode) {
    switch (keycode) {
        case 58:
        case 61:
            return NSEventModifierFlagOption;
        case 59:
        case 62:
            return NSEventModifierFlagControl;
        case 56:
        case 60:
            return NSEventModifierFlagShift;
        case 54:
        case 55:
        default:
            return NSEventModifierFlagCommand;
    }
}

void vosi_hotkey_set_keycode(uint16_t keycode) {
    s_keycode = keycode;
    s_flag = flag_for_keycode(keycode);
}

void vosi_hotkey_stop(void) {
    if (s_monitor) {
        [NSEvent removeMonitor:s_monitor];
        s_monitor = nil;
    }
    s_held = false;
}

bool vosi_hotkey_start(VosiHotkeyFn callback) {
    if (![NSThread isMainThread]) {
        __block bool result = false;
        dispatch_sync(dispatch_get_main_queue(), ^{
            result = vosi_hotkey_start(callback);
        });
        return result;
    }

    vosi_hotkey_stop();
    if (!callback) {
        return false;
    }
    s_callback = callback;

    s_monitor = [NSEvent addGlobalMonitorForEventsMatchingMask:NSEventMaskFlagsChanged
                                                       handler:^(NSEvent *event) {
        if (event.keyCode != s_keycode) {
            return;
        }
        bool down = (event.modifierFlags & s_flag) != 0;
        if (down && !s_held) {
            s_held = true;
            s_callback(0);
        } else if (!down && s_held) {
            s_held = false;
            s_callback(1);
        }
    }];
    return s_monitor != nil;
}
