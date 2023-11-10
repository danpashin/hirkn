use crate::source::Source;
use anyhow::Result;
use serde::Deserialize;
use std::{fs::File, path::Path, time::Duration};

#[derive(Deserialize)]
#[serde(default)]
pub(crate) struct AutoUpdateConfig {
    pub(crate) enabled: bool,

    #[serde(with = "humantime_serde")]
    pub(crate) timeout: Duration,
}

#[derive(Deserialize)]
#[serde(default)]
pub(crate) struct Config {
    log_level: log::Level,

    pub(crate) table_name: String,

    pub(crate) sources: Vec<Source>,

    pub(crate) single_run_append_max: Option<usize>,

    pub(crate) auto_update: AutoUpdateConfig,
}

impl Config {
    pub(crate) fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        Ok(serde_yaml::from_reader(file)?)
    }

    pub(crate) fn init_logger(&self) {
        simple_logger::init_with_level(self.log_level).expect("Cannot init logger");
    }
}

impl Default for AutoUpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: Duration::from_secs(12 * 60 * 60),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: log::Level::Info,
            table_name: "fw4".to_string(),
            sources: vec![],
            single_run_append_max: None,
            auto_update: AutoUpdateConfig::default(),
        }
    }
}
