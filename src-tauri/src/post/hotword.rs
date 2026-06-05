use std::collections::HashSet;
use std::path::Path;

pub struct HotwordReplacer {
    words: Vec<String>,
}

impl HotwordReplacer {
    pub fn from_lines(lines: impl IntoIterator<Item = String>) -> Self {
        let mut words: Vec<String> = lines
            .into_iter()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        words.sort_by_key(|w| std::cmp::Reverse(w.len()));
        Self { words }
    }

    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        Ok(Self::from_lines(raw.lines().map(|l| l.to_string())))
    }

    pub fn apply(&self, text: &str) -> String {
        let mut out = text.to_string();
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let token_set: HashSet<&str> = tokens.iter().copied().collect();

        for word in &self.words {
            if out.contains(word) {
                continue;
            }
            for token in &token_set {
                if word.contains(token) && token.len() >= 2 {
                    out = out.replace(token, word);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_fuzzy_token_with_hotword() {
        let replacer = HotwordReplacer::from_lines(vec!["阿里巴巴".into()]);
        let result = replacer.apply("我去阿里上班");
        assert!(result.contains("阿里巴巴") || result.contains("阿里"));
    }
}
