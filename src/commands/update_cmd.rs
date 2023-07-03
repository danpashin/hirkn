use crate::commands::{CliCommand, GlobalOptions};
use anyhow::Result;
use nftables::{
    batch::Batch,
    helper::apply_ruleset,
    schema::{NfListObject, Set},
};

#[derive(clap::Parser)]
pub(crate) struct Command {
    #[clap(flatten)]
    global_options: GlobalOptions,
}

#[async_trait]
impl CliCommand for Command {
    async fn run(&self) -> Result<()> {
        let config = self.global_options.parse_config()?;
        let chunk_size = config.single_run_append_max.unwrap_or(usize::MAX);
        eprintln!("Using chunks of {chunk_size} elements for apply operations");

        for source in config.sources {
            let elements = source.download_set().await?;
            eprintln!(
                "Downloaded {} elements for {} set. Applying...",
                elements.len(),
                source.set_name
            );

            let set = Set {
                family: source.set_template.family,
                table: config.table_name.clone(),
                name: source.set_name,
                handle: None,
                set_type: source.set_template.set_type,
                policy: source.set_template.policy,
                flags: source.set_template.flags,
                elem: None,
                timeout: source.set_template.timeout,
                gc_interval: source.set_template.gc_interval,
                size: None,
            };

            for chunk in elements.chunks(chunk_size) {
                let mut set = set.clone();
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
