#[derive(Debug, Clone, Copy)]
pub enum InjectMethod {
    Type,
    Paste,
}

pub trait TextInjector: Send {
    fn inject(&self, text: &str, method: InjectMethod) -> Result<(), String>;
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub fn default_injector() -> Box<dyn TextInjector> {
    #[cfg(target_os = "macos")]
    {
        return Box::new(macos::MacInjector);
    }
    #[cfg(target_os = "windows")]
    {
        return Box::new(windows::WinInjector);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        struct Noop;
        impl TextInjector for Noop {
            fn inject(&self, _: &str, _: InjectMethod) -> Result<(), String> {
                Err("unsupported platform".into())
            }
        }
        Box::new(Noop)
    }
}

pub fn method_from_config(name: &str) -> InjectMethod {
    match name.to_lowercase().as_str() {
        "paste" => InjectMethod::Paste,
        _ => InjectMethod::Type,
    }
}

pub struct FallbackResult {
    pub injected: bool,
    pub copied_to_clipboard: bool,
}

pub fn inject_with_fallback(
    injector: &dyn TextInjector,
    text: &str,
    method: InjectMethod,
) -> FallbackResult {
    match injector.inject(text, method) {
        Ok(()) => FallbackResult {
            injected: true,
            copied_to_clipboard: false,
        },
        Err(_) => {
            let copied_to_clipboard = arboard::Clipboard::new()
                .and_then(|mut clipboard| clipboard.set_text(text.to_owned()))
                .is_ok();
            FallbackResult {
                injected: false,
                copied_to_clipboard,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingInjector;

    impl TextInjector for FailingInjector {
        fn inject(&self, _: &str, _: InjectMethod) -> Result<(), String> {
            Err("inject failed".into())
        }
    }

    #[test]
    fn inject_with_fallback_copies_to_clipboard_on_failure() {
        let text = "你好世界";
        let result = inject_with_fallback(&FailingInjector, text, InjectMethod::Type);

        assert!(!result.injected);
        assert!(result.copied_to_clipboard);

        let clipboard_text = arboard::Clipboard::new()
            .expect("clipboard")
            .get_text()
            .expect("clipboard text");
        assert_eq!(clipboard_text, text);
    }
}
