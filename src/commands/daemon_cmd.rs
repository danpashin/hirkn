use super::{update_cmd::Command as UpdateCommand, CliCommand, GlobalOptions};
use anyhow::Result;
use std::time::Duration;

#[derive(clap::Parser)]
pub(crate) struct Command {
    #[clap(flatten)]
    global_options: GlobalOptions,
}

#[async_trait]
impl CliCommand for Command {
    async fn run(&self) -> Result<()> {
        let config = self.global_options.parse_config()?;
        let sleep_duration: Duration = config.auto_update.timeout.into();

        let shutdown = tokio_shutdown::Shutdown::new()?;
        let update_cmd = UpdateCommand::new(self.global_options.clone());

        eprintln!("Starting daemon...");
        loop {
            tokio::select! {
                _ = shutdown.handle() => {
                        eprintln!("Got shutdown signal. Exiting...");
                        break;
                    },
                _ = tokio::time::sleep(sleep_duration) => {
                    if let Err(error) = update_cmd.perform_update(&config).await {
                        eprintln!("Error when performing update command: {error:?}");
                    }
                }
            }
        }

        Ok(())
    }
}
