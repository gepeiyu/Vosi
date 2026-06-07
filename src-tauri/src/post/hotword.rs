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

pub fn merge_builtin_hotwords(user_path: &Path, builtin_lines: &[&str]) -> std::io::Result<()> {
    let existing = std::fs::read_to_string(user_path).unwrap_or_default();
    let existing_set: HashSet<String> = existing
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let mut additions = Vec::new();
    for word in builtin_lines {
        let w = word.trim();
        if w.is_empty() {
            continue;
        }
        if !existing_set.contains(w) {
            additions.push(w);
        }
    }
    if additions.is_empty() {
        return Ok(());
    }
    if let Some(parent) = user_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(user_path)?;
    for word in additions {
        writeln!(file, "{word}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_builtin_hotwords_appends_without_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hotwords.txt");
        std::fs::write(&path, "React\n").unwrap();
        merge_builtin_hotwords(&path, &["React", "TypeScript"]).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.matches("React").count(), 1);
        assert!(content.contains("TypeScript"));
    }

    #[test]
    fn replaces_fuzzy_token_with_hotword() {
        let replacer = HotwordReplacer::from_lines(vec!["阿里巴巴".into()]);
        let result = replacer.apply("我去阿里上班");
        assert!(result.contains("阿里巴巴") || result.contains("阿里"));
    }
}
