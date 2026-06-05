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
