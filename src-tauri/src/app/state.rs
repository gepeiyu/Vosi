use crate::config::AppConfig;
use crate::log::Logger;
use crate::permissions::SetupPhase;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

pub struct AppState {
    config: Arc<RwLock<AppConfig>>,
    pipeline_started: AtomicBool,
    pipeline_spawned: AtomicBool,
    setup_phase: RwLock<SetupPhase>,
    setup_message: RwLock<Option<String>>,
    logger: RwLock<Option<Arc<Logger>>>,
    bundled: RwLock<Option<PathBuf>>,
    dev_models: RwLock<Option<PathBuf>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            pipeline_started: AtomicBool::new(false),
            pipeline_spawned: AtomicBool::new(false),
            setup_phase: RwLock::new(SetupPhase::WaitingPermissions),
            setup_message: RwLock::new(None),
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

    pub fn pipeline_spawned(&self) -> bool {
        self.pipeline_spawned.load(Ordering::Relaxed)
    }

    pub fn mark_pipeline_spawned(&self) {
        self.pipeline_spawned.store(true, Ordering::Relaxed);
    }

    pub fn clear_pipeline_spawned(&self) {
        self.pipeline_spawned.store(false, Ordering::Relaxed);
    }

    pub fn mark_pipeline_started(&self) {
        self.pipeline_started.store(true, Ordering::Relaxed);
        self.set_setup(SetupPhase::Ready, None);
    }

    pub fn setup_phase(&self) -> SetupPhase {
        self.setup_phase.read().expect("setup_phase lock").clone()
    }

    pub fn setup_message(&self) -> Option<String> {
        self.setup_message.read().expect("setup_message lock").clone()
    }

    pub fn set_setup(&self, phase: SetupPhase, message: Option<String>) {
        *self.setup_phase.write().expect("setup_phase lock") = phase;
        *self.setup_message.write().expect("setup_message lock") = message;
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
