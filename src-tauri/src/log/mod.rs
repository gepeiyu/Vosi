use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MAX_LOG_BYTES: u64 = 1_048_576;

pub fn log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vosi")
        .join("logs")
}

pub struct Logger {
    path: PathBuf,
    lock: Mutex<()>,
}

impl Logger {
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self {
            path: root.join("vosi.log"),
            lock: Mutex::new(()),
        })
    }

    pub fn default_logger() -> std::io::Result<Self> {
        Self::new(log_dir())
    }

    pub fn info(&self, msg: &str) {
        self.write("INFO", msg);
    }

    pub fn error(&self, msg: &str) {
        self.write("ERROR", msg);
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, level: &str, msg: &str) {
        let _guard = self.lock.lock().expect("logger lock");
        self.rotate_if_needed();
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(file, "[{level}] {msg}");
        }
    }

    fn rotate_if_needed(&self) {
        let Ok(meta) = fs::metadata(&self.path) else {
            return;
        };
        if meta.len() <= MAX_LOG_BYTES {
            return;
        }
        let backup = self.path.with_extension("log.1");
        let _ = fs::remove_file(&backup);
        let _ = fs::rename(&self.path, backup);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logger_writes_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let logger = Logger::new(dir.path().to_path_buf()).unwrap();
        logger.info("test event");
        let content = fs::read_to_string(logger.path()).unwrap();
        assert!(content.contains("[INFO] test event"));
    }
}
