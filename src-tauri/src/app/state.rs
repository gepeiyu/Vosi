use crate::config::AppConfig;
use std::sync::{Arc, RwLock};

pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
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
