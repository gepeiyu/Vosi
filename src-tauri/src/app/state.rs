use crate::config::AppConfig;
use crate::log::Logger;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

pub struct AppState {
    config: Arc<RwLock<AppConfig>>,
    pipeline_started: AtomicBool,
    logger: RwLock<Option<Arc<Logger>>>,
    bundled: RwLock<Option<PathBuf>>,
    dev_models: RwLock<Option<PathBuf>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            pipeline_started: AtomicBool::new(false),
            logger: RwLock::new(None),
            bundled: RwLock::new(None),
            dev_models: RwLock::new(None),
        }
    }

    pub fn init_runtime(
        &self,
        logger: Arc<Logger>,
        bundled: PathBuf,
        dev_models: Option<PathBuf>,
    ) {
        *self.logger.write().expect("logger lock") = Some(logger);
        *self.bundled.write().expect("bundled lock") = Some(bundled);
        *self.dev_models.write().expect("dev_models lock") = dev_models;
    }

    pub fn voice_ready(&self) -> bool {
        self.pipeline_started.load(Ordering::Relaxed)
    }

    pub fn mark_pipeline_started(&self) {
        self.pipeline_started.store(true, Ordering::Relaxed);
    }

    pub fn logger(&self) -> Option<Arc<Logger>> {
        self.logger.read().expect("logger lock").clone()
    }

    pub fn bundled(&self) -> Option<PathBuf> {
        self.bundled.read().expect("bundled lock").clone()
    }

    pub fn dev_models(&self) -> Option<PathBuf> {
        self.dev_models.read().expect("dev_models lock").clone()
    }

    pub fn get_config(&self) -> AppConfig {
        self.config.read().expect("config lock").clone()
    }

    pub fn set_config(&self, cfg: AppConfig) -> Result<(), String> {
        crate::config::save(&cfg).map_err(|e| e.to_string())?;
        *self.config.write().expect("config lock") = cfg;
        Ok(())
    }
}
