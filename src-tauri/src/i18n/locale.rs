#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    Zh,
    En,
    Ja,
}

impl Locale {
    pub fn default_locale() -> Self {
        Self::Zh
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "en" => Self::En,
            "ja" => Self::Ja,
            _ => Self::Zh,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
            Self::Ja => "ja",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_locale_falls_back_to_zh() {
        assert_eq!(Locale::parse("fr"), Locale::Zh);
    }

    #[test]
    fn default_is_zh() {
        assert_eq!(Locale::default_locale(), Locale::Zh);
    }
}
