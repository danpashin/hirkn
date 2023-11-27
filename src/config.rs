use crate::source::{Source, IP};
use anyhow::Result;
use either::Either;
use serde::Deserialize;
use std::collections::HashSet;
use std::{fs::File, path::Path};
use url::Url;

#[derive(Deserialize)]
#[serde(default)]
pub(crate) struct Config {
    log_level: log::Level,

    pub(crate) table_name: String,

    pub(crate) sources: Vec<Source>,

    #[serde(with = "either::serde_untagged_optional")]
    pub(crate) excluded_ips: Option<Either<Url, HashSet<IP>>>,

    pub(crate) split_by_chunks: Option<usize>,

    pub(crate) update_schedule: Option<String>,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: log::Level::Info,
            table_name: "fw4".to_string(),
            sources: vec![],
            excluded_ips: None,
            split_by_chunks: None,
            update_schedule: None,
        }
    }
}
