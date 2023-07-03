use super::{CliCommand, GlobalOptions};
use crate::{config::Config, source::SourcesCache};
use anyhow::Result;
use nftables::{batch::Batch, helper::apply_ruleset, schema::NfListObject};

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
        eprintln!("Using chunks of {chunk_size} elements for apply operations");

        for source in &config.sources {
            let elements = source.download_list(sources_cache.clone()).await?;
            eprintln!(
                "Downloaded {} elements for {} set. Applying...",
                elements.len(),
                source.set_name
            );

            for chunk in elements.chunks(chunk_size) {
                let mut set = source.create_set(&config.table_name);
                set.elem = Some(chunk.to_vec());

                let mut batch = Batch::new();
                batch.add(NfListObject::Set(set));

                let nftables = batch.to_nftables();
                apply_ruleset(&nftables, None, None)?;
            }
        }

        eprintln!("Successfully updated all sources!");

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
