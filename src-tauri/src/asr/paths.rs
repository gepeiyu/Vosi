use std::path::{Path, PathBuf};

fn first_existing(base: &Path, names: &[&str]) -> Option<PathBuf> {
    for name in names {
        let path = base.join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Resolve SenseVoice model.int8.onnx and tokens.txt under a directory or nested subdir.
pub fn resolve_sense_voice_paths(dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    if !dir.exists() {
        return Err(format!("sense-voice model dir not found: {}", dir.display()));
    }

    let search_roots: Vec<PathBuf> = if dir.is_dir() {
        let mut roots = vec![dir.to_path_buf()];
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    roots.push(entry.path());
                }
            }
        }
        roots
    } else {
        vec![dir.to_path_buf()]
    };

    for root in search_roots {
        if let Some(model) = first_existing(
            &root,
            &["model.int8.onnx", "model.onnx"],
        ) {
            let tokens = root.join("tokens.txt");
            if tokens.exists() {
                return Ok((model, tokens));
            }
        }
    }

    Err(format!(
        "could not find sense-voice model and tokens.txt under {}",
        dir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_sense_voice_paths_finds_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("pkg");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("model.int8.onnx"), b"x").unwrap();
        fs::write(nested.join("tokens.txt"), b"t").unwrap();

        let (model, tokens) = resolve_sense_voice_paths(dir.path()).unwrap();
        assert!(model.ends_with("model.int8.onnx"));
        assert!(tokens.ends_with("tokens.txt"));
    }
}
