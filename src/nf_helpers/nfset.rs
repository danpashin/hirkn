use crate::source::SetTemplate;
use anyhow::Result;
use itertools::Itertools;
use nftables::{
    batch::Batch,
    expr::Expression,
    helper::apply_ruleset,
    schema::{self, FlushObject, NfCmd, NfListObject},
};

pub(crate) struct NfSet {
    inner: schema::Set,
}

impl NfSet {
    pub(crate) fn with_template(
        name: impl Into<String>,
        table_name: impl Into<String>,
        template: SetTemplate,
    ) -> Self {
        let inner = schema::Set {
            family: template.family,
            table: table_name.into(),
            name: name.into(),
            handle: None,
            set_type: template.set_type,
            policy: template.policy,
            flags: template.flags,
            elem: None,
            timeout: template.timeout,
            gc_interval: template.gc_interval,
            size: None,
        };

        Self { inner }
    }

    pub(crate) fn flush(&self) -> Result<()> {
        let mut batch = Batch::new();

        let object = FlushObject::Set(self.inner.clone());
        batch.add_cmd(NfCmd::Flush(object));

        let nftables = batch.to_nftables();
        apply_ruleset(&nftables, None, None)?;

        Ok(())
    }

    pub(crate) fn load_entries(&self, entries: Vec<Expression>, chunk_size: usize) -> Result<()> {
        log::info!(
            "Downloaded {} elements for {} set. Applying...",
            entries.len(),
            self.inner.name
        );

        let chunked: Vec<Vec<_>> = entries
            .into_iter()
            .chunks(chunk_size)
            .into_iter()
            .map(Iterator::collect)
            .collect();

        for chunk in chunked {
            let mut set = self.inner.clone();
            set.elem = Some(chunk);

            let mut batch = Batch::new();
            batch.add(NfListObject::Set(set));

            let nftables = batch.to_nftables();
            apply_ruleset(&nftables, None, None)?;
        }

        Ok(())
    }
}
