mod update_cmd;

use crate::config::Config;
use anyhow::Result;
use std::path::{Path, PathBuf};

static DEFAULT_CONFIG_PATH: &str = concat!("/etc/", env!("CARGO_PKG_NAME"), "/config.yaml");
static EXAMPLE_CONFIG_PATH: &str = concat!("/etc/", env!("CARGO_PKG_NAME"), "/config.yaml.example");

#[async_trait]
pub(crate) trait CliCommand {
    async fn run(&self) -> Result<()>;
}

#[derive(clap::Parser)]
struct GlobalOptions {
    #[arg(
        long,
        short,
        value_parser,
        default_value = DEFAULT_CONFIG_PATH
    )]
    #[arg(help_heading = "GLOBAL OPTIONS", global = true)]
    config: PathBuf,
}

impl GlobalOptions {
    pub(crate) fn parse_config(&self) -> Result<Config> {
        let path = if self.config.exists() {
            self.config.as_path()
        } else {
            std::fs::copy(EXAMPLE_CONFIG_PATH, DEFAULT_CONFIG_PATH)?;
            Path::new(DEFAULT_CONFIG_PATH)
        };

        Config::from_file(path)
    }
}

#[derive(clap::Parser)]
#[clap(version)]
pub(crate) enum Command {
    #[clap(disable_version_flag = true)]
    Update(update_cmd::Command),
}
