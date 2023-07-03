use super::{update_cmd::Command as UpdateCommand, CliCommand, GlobalOptions};
use crate::source::SourcesCache;
use anyhow::Result;

#[derive(clap::Parser)]
pub(crate) struct Command {
    #[clap(flatten)]
    global_options: GlobalOptions,
}

#[async_trait]
impl CliCommand for Command {
    async fn run(&self) -> Result<()> {
        let config = self.global_options.parse_config()?;

        let shutdown = tokio_shutdown::Shutdown::new()?;
        let update_cmd = UpdateCommand::new(self.global_options.clone());

        let cache = SourcesCache::new();
        let mut update_interval = tokio::time::interval(config.auto_update.timeout.into());

        eprintln!("Starting daemon...");
        loop {
            tokio::select! {
                _ = shutdown.handle() => {
                        eprintln!("Got shutdown signal. Exiting...");
                        break;
                    },
                _ = update_interval.tick() => {
                    if let Err(error) = update_cmd.perform_update(&config, cache.clone()).await {
                        eprintln!("Error when performing update command: {error:?}");
                    }
                }
            }
        }

        Ok(())
    }
}
