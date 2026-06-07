#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#import <AVFoundation/AVFoundation.h>
#import <dispatch/dispatch.h>

typedef enum {
    VosiMicNotDetermined = 0,
    VosiMicRestricted = 1,
    VosiMicDenied = 2,
    VosiMicAuthorized = 3,
} VosiMicStatus;

static AVAuthorizationStatus vosi_current_status(void) {
    return [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
}

static VosiMicStatus vosi_map_status(AVAuthorizationStatus status) {
    switch (status) {
        case AVAuthorizationStatusAuthorized:
            return VosiMicAuthorized;
        case AVAuthorizationStatusDenied:
            return VosiMicDenied;
        case AVAuthorizationStatusRestricted:
            return VosiMicRestricted;
        case AVAuthorizationStatusNotDetermined:
        default:
            return VosiMicNotDetermined;
    }
}

int vosi_microphone_status(void) {
    return (int)vosi_map_status(vosi_current_status());
}

bool vosi_microphone_authorized(void) {
    return vosi_current_status() == AVAuthorizationStatusAuthorized;
}

bool vosi_open_privacy_settings(const char *pane);

bool vosi_open_microphone_settings(void) {
    return vosi_open_privacy_settings("Privacy_Microphone");
}

static bool open_url(NSString *urlString) {
    NSURL *url = [NSURL URLWithString:urlString];
    if (!url) {
        return NO;
    }
    return [[NSWorkspace sharedWorkspace] openURL:url];
}

bool vosi_open_privacy_settings(const char *pane) {
    if (!pane) {
        return false;
    }
    NSString *paneName = [NSString stringWithUTF8String:pane];
    NSArray<NSString *> *urls = @[
        [NSString stringWithFormat:
            @"x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?%@",
            paneName],
        [NSString stringWithFormat:
            @"x-apple.systempreferences:com.apple.preference.security?%@",
            paneName],
    ];

    for (NSString *urlString in urls) {
        if (open_url(urlString)) {
            return true;
        }
    }
    return false;
}

/// Pump the main run loop until the TCC dialog completes.
/// Never use dispatch_semaphore_wait on the main thread here — it deadlocks
/// the permission dialog and the app never registers in System Settings.
static bool request_access_on_main(void) {
    __block BOOL granted = NO;
    __block BOOL finished = NO;

    [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio
                             completionHandler:^(BOOL g) {
                                 granted = g;
                                 finished = YES;
                             }];

    while (!finished) {
        [[NSRunLoop currentRunLoop]
            runMode:NSDefaultRunLoopMode
           beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.05]];
    }

    return granted;
}

static bool request_microphone_impl(void) {
    AVAuthorizationStatus status = vosi_current_status();
    if (status == AVAuthorizationStatusAuthorized) {
        return true;
    }
    if (status == AVAuthorizationStatusDenied
        || status == AVAuthorizationStatusRestricted) {
        return false;
    }

    return request_access_on_main();
}

bool vosi_request_microphone(void) {
    if ([NSThread isMainThread]) {
        return request_microphone_impl();
    }

    __block BOOL result = NO;
    dispatch_sync(dispatch_get_main_queue(), ^{
        result = request_microphone_impl();
    });
    return result;
}

bool vosi_prompt_microphone_denied(void) {
    if (![NSThread isMainThread]) {
        __block BOOL result = NO;
        dispatch_sync(dispatch_get_main_queue(), ^{
            result = vosi_prompt_microphone_denied();
        });
        return result;
    }

    NSAlert *alert = [[NSAlert alloc] init];
    alert.messageText = @"需要麦克风权限";
    alert.informativeText =
        @"Vosi 需要访问麦克风才能录音。请在系统设置 → 隐私与安全性 → 麦克风中开启 Vosi。";
    [alert addButtonWithTitle:@"打开系统设置"];
    [alert addButtonWithTitle:@"稍后"];
    if ([alert runModal] == NSAlertFirstButtonReturn) {
        return vosi_open_microphone_settings();
    }
    return false;
}

bool vosi_activate_app(void) {
    if (![NSThread isMainThread]) {
        __block BOOL result = NO;
        dispatch_sync(dispatch_get_main_queue(), ^{
            result = vosi_activate_app();
        });
        return result;
    }
    [NSApp activateIgnoringOtherApps:YES];
    return true;
}

bool vosi_is_accessibility_trusted(void) {
    return AXIsProcessTrusted();
}

bool vosi_request_accessibility(void) {
    if (![NSThread isMainThread]) {
        __block BOOL result = NO;
        dispatch_sync(dispatch_get_main_queue(), ^{
            result = vosi_request_accessibility();
        });
        return result;
    }

    [NSApp activateIgnoringOtherApps:YES];

    if (AXIsProcessTrusted()) {
        return true;
    }

    CFStringRef key = CFSTR("AXTrustedCheckOptionPrompt");
    CFBooleanRef value = kCFBooleanTrue;
    const void *keys[] = { key };
    const void *values[] = { value };
    CFDictionaryRef options = CFDictionaryCreate(
        kCFAllocatorDefault, keys, values, 1,
        &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    bool trusted = AXIsProcessTrustedWithOptions(options);
    CFRelease(options);
    return trusted;
}
