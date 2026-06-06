use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModelPaths {
    pub paraformer_dir: PathBuf,
    pub vad_model: PathBuf,
    pub punc_dir: PathBuf,
}

pub struct ModelManager {
    root: PathBuf,
}

impl ModelManager {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    pub fn resolve_paths(&self) -> ModelPaths {
        let base = self.models_dir();
        ModelPaths {
            paraformer_dir: base.join("paraformer-zh"),
            vad_model: base.join("vad/model.onnx"),
            punc_dir: base.join("punctuation"),
        }
    }

    pub fn verify_file_sha256(path: &Path, expected: &str) -> bool {
        if !path.exists() || expected.is_empty() {
            return false;
        }
        let bytes = std::fs::read(path).unwrap_or_default();
        let digest = Sha256::digest(bytes);
        format!("{:x}", digest) == expected.to_lowercase()
    }

    pub fn paraformer_ready(base: &Path) -> bool {
        let dir = base.join("paraformer-zh");
        ["model.int8.onnx", "model.onnx", "model_quant.onnx"]
            .iter()
            .any(|name| dir.join(name).exists())
    }

    pub fn ensure_installed(&self, bundled: &Path, dev_fallback: Option<&Path>) -> std::io::Result<ModelPaths> {
        let dest = self.models_dir();
        if !Self::paraformer_ready(&dest) {
            std::fs::create_dir_all(&dest)?;
            copy_dir_all(bundled, &dest)?;
            if !Self::paraformer_ready(&dest) {
                if let Some(dev) = dev_fallback {
                    copy_dir_all(dev, &dest)?;
                }
            }
        }
        Ok(self.resolve_paths())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn verify_file_sha256_matches() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.bin");
        let mut f = std::fs::File::create(&file).unwrap();
        f.write_all(b"hello").unwrap();
        let expected = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(ModelManager::verify_file_sha256(&file, expected));
    }

    #[test]
    fn resolve_paths_under_root() {
        let mgr = ModelManager::new(PathBuf::from("/tmp/vosi"));
        let paths = mgr.resolve_paths();
        assert!(paths.paraformer_dir.ends_with("paraformer-zh"));
        assert!(paths.vad_model.ends_with("vad/model.onnx"));
    }
}
