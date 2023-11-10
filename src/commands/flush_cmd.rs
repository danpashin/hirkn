use super::{CliCommand, GlobalOptions};
use crate::nf_helpers::NfSet;
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

        let mut success = true;

        for source in &config.sources {
            let nfset = NfSet::with_template(
                &source.set_name,
                &config.table_name,
                source.set_template.clone(),
            );

            if let Err(error) = nfset.flush() {
                success = false;
                log::error!("Error while flushing: {error:?}");
            };
        }

        if success {
            log::warn!("All sets are flushed!");
        }

        Ok(())
    }
}
