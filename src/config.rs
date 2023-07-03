use crate::{relative_time::RelativeTime, source::Source};
use anyhow::Result;
use serde::Deserialize;
use std::{fs::File, path::Path};

#[derive(Deserialize)]
#[serde(default)]
pub(crate) struct AutoUpdateConfig {
    pub(crate) enabled: bool,
    pub(crate) timeout: RelativeTime,
}

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) table_name: String,

    #[serde(default)]
    pub(crate) sources: Vec<Source>,

    pub(crate) single_run_append_max: Option<usize>,

    #[serde(default)]
    pub(crate) auto_update: AutoUpdateConfig,
}

impl Config {
    pub(crate) fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        Ok(serde_yaml::from_reader(file)?)
    }
}

impl Default for AutoUpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: RelativeTime::Hours(12),
        }
    }
}
