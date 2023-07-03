use crate::source::Source;
use anyhow::Result;
use serde::Deserialize;
use std::{fs::File, path::Path};

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) table_name: String,

    #[serde(default)]
    pub(crate) sources: Vec<Source>,

    pub(crate) single_run_append_max: Option<usize>,
}

impl Config {
    pub(crate) fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        Ok(serde_yaml::from_reader(file)?)
    }
}
