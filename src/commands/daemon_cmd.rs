use super::{update_cmd::Command as UpdateCommand, CliCommand, GlobalOptions};
use crate::commands::update_cmd::UpdateRequestBuilder;
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

        let request = UpdateRequestBuilder::new(config).build().await?;
        let mut update_interval = tokio::time::interval(request.config.auto_update.timeout);

        log::warn!("Starting daemon...");
        loop {
            tokio::select! {
                () = shutdown.handle() => {
                        log::warn!("Got shutdown signal. Exiting...");
                        break;
                    },
                _ = update_interval.tick() => {
                    if let Err(error) = update_cmd.perform_update(&request).await {
                        log::error!("Error when performing update command: {error:?}");
                    }
                }
            }
        }

        Ok(())
    }
}
