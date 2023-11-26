mod update_request;

use self::update_request::UpdateRequest;
pub(crate) use self::update_request::UpdateRequestBuilder;
use super::{CliCommand, GlobalOptions};
use crate::nf_helpers::NfSet;
use anyhow::Result;

#[derive(clap::Parser)]
pub(crate) struct Command {
    #[clap(flatten)]
    global_options: GlobalOptions,
}

impl Command {
    pub(crate) fn new(options: GlobalOptions) -> Self {
        Self {
            global_options: options,
        }
    }

    pub(crate) async fn perform_update(&self, request: &UpdateRequest) -> Result<()> {
        let chunk_size = request.chunk_size();
        log::info!("Using chunks of {chunk_size} elements for apply operations");

        for source in &request.config.sources {
            let nfset = NfSet::with_template(
                &source.set_name,
                &request.config.table_name,
                source.set_template.clone(),
            );

            let cache = request.sources_cache();
            let excluded = request.excluded_ips();
            let entries = source.download_list(cache, excluded).await?;
            if !entries.is_empty() {
                nfset.flush()?;
                nfset.load_entries(entries, chunk_size)?;
            }
        }

        log::info!("Successfully updated all sources!");

        Ok(())
    }
}

#[async_trait]
impl CliCommand for Command {
    async fn run(&self) -> Result<()> {
        let config = self.global_options.parse_config()?;
        let request = UpdateRequestBuilder::new(config).build().await?;

        self.perform_update(&request).await?;

        Ok(())
    }
}
