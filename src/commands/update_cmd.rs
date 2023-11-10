use super::{CliCommand, GlobalOptions};
use crate::{config::Config, nf_helpers::NfSet, source::SourcesCache};
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

    pub(crate) async fn perform_update(
        &self,
        config: &Config,
        sources_cache: SourcesCache,
    ) -> Result<()> {
        let chunk_size = config.single_run_append_max.unwrap_or(usize::MAX);
        log::info!("Using chunks of {chunk_size} elements for apply operations");

        for source in &config.sources {
            let nfset = NfSet::with_template(
                &source.set_name,
                &config.table_name,
                source.set_template.clone(),
            );

            let entries = source.download_list(sources_cache.clone()).await?;
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
        let cache = SourcesCache::new();
        self.perform_update(&config, cache).await?;

        Ok(())
    }
}
