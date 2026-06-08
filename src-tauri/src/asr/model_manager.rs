use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModelPaths {
    pub sense_voice_dir: PathBuf,
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
            sense_voice_dir: base.join("sense-voice"),
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

    pub fn sense_voice_ready(base: &Path) -> bool {
        let dir = base.join("sense-voice");
        let has_model = ["model.int8.onnx", "model.onnx"]
            .iter()
            .any(|name| dir.join(name).exists());
        has_model && dir.join("tokens.txt").exists()
    }

    /// True when Application Support is missing sense-voice but the bundle can supply it.
    pub fn needs_install(models_root: &Path, bundled: &Path) -> bool {
        let dest = models_root.join("models");
        !Self::sense_voice_ready(&dest) && Self::sense_voice_ready(bundled)
    }

    pub fn ensure_installed(
        &self,
        bundled: &Path,
        dev_fallback: Option<&Path>,
    ) -> std::io::Result<ModelPaths> {
        let dest = self.models_dir();
        std::fs::create_dir_all(&dest)?;

        if !Self::sense_voice_ready(&dest) {
            if Self::sense_voice_ready(bundled) {
                copy_dir_all(&bundled.join("sense-voice"), &dest.join("sense-voice"))?;
            } else if let Some(dev) = dev_fallback {
                if Self::sense_voice_ready(dev) {
                    copy_dir_all(&dev.join("sense-voice"), &dest.join("sense-voice"))?;
                }
            }
        }

        if !dest.join("vad/model.onnx").exists() && bundled.join("vad/model.onnx").exists() {
            copy_dir_all(&bundled.join("vad"), &dest.join("vad"))?;
        }

        if !Self::sense_voice_ready(&dest) {
            copy_dir_all(bundled, &dest)?;
            if !Self::sense_voice_ready(&dest) {
                if let Some(dev) = dev_fallback {
                    copy_dir_all(dev, &dest)?;
                }
            }
        }

        remove_legacy_paraformer(&dest)?;
        Ok(self.resolve_paths())
    }
}

fn remove_legacy_paraformer(dest: &Path) -> std::io::Result<()> {
    let legacy = dest.join("paraformer-zh");
    if legacy.exists() {
        std::fs::remove_dir_all(&legacy)?;
    }
    Ok(())
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
        assert!(paths.sense_voice_dir.ends_with("sense-voice"));
        assert!(paths.vad_model.ends_with("vad/model.onnx"));
    }

    #[test]
    fn ensure_installed_copies_sense_voice_and_removes_legacy_paraformer() {
        let bundled = tempfile::tempdir().unwrap();
        let sv = bundled.path().join("sense-voice");
        std::fs::create_dir_all(&sv).unwrap();
        std::fs::write(sv.join("model.int8.onnx"), b"x").unwrap();
        std::fs::write(sv.join("tokens.txt"), b"t").unwrap();

        let dest_root = tempfile::tempdir().unwrap();
        let pf = dest_root.path().join("models/paraformer-zh");
        std::fs::create_dir_all(&pf).unwrap();
        std::fs::write(pf.join("model.int8.onnx"), b"old").unwrap();

        let mgr = ModelManager::new(dest_root.path().to_path_buf());
        mgr.ensure_installed(bundled.path(), None).unwrap();

        let models = dest_root.path().join("models");
        assert!(ModelManager::sense_voice_ready(&models));
        assert!(!models.join("paraformer-zh").exists());
    }
}
