mod cache;
mod source_provider;

use self::source_provider::FetchStatus;
pub(crate) use self::{
    cache::Cache as SourcesCache,
    source_provider::{IPParsable, SourceProvider},
};
use anyhow::Result;
use nftables::{expr::Expression, schema, types};
use serde::Deserialize;
use std::collections::{HashSet, LinkedList};
use tokio::task::JoinSet;
use url::Url;

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub(crate) struct SetTemplate {
    pub(crate) family: types::NfFamily,
    #[serde(rename = "type")]
    pub(crate) set_type: schema::SetTypeValue,
    pub(crate) policy: Option<schema::SetPolicy>,
    pub(crate) flags: Option<HashSet<schema::SetFlag>>,
    pub(crate) timeout: Option<u32>,
    pub(crate) gc_interval: Option<u32>,
}

impl Default for SetTemplate {
    fn default() -> Self {
        Self {
            family: types::NfFamily::INet,
            set_type: schema::SetTypeValue::Single(schema::SetType::Ipv4Addr),
            policy: None,
            flags: Some([schema::SetFlag::Interval].into()),
            timeout: None,
            gc_interval: None,
        }
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct Source {
    pub(crate) set_name: String,
    #[serde(default)]
    pub(crate) set_template: SetTemplate,
    pub(crate) urls: Vec<Url>,
    pub(crate) entries_limit: usize,
}

impl Source {
    pub(crate) async fn download_list(&self, cache: SourcesCache) -> Result<Vec<Expression>> {
        let Some(first_url) = self.urls.first() else {
            return Ok(vec![]);
        };

        if self.urls.len() == 1 {
            let mut entries = download_ips_list(first_url.clone(), cache).await?;

            if self.entries_limit != 0 && entries.len() > self.entries_limit {
                log::warn!(
                    "Source {} exceeds maximum ({}) number of entries. Got {}. Truncating...",
                    first_url,
                    self.entries_limit,
                    entries.len()
                );
                entries.truncate(self.entries_limit);
            }

            return Ok(entries);
        }

        let mut active_downloads = JoinSet::new();
        for url in &self.urls {
            active_downloads.spawn(download_ips_list(url.clone(), cache.clone()));
        }

        let mut entries = LinkedList::new();

        while let Some(download) = active_downloads.join_next().await {
            let download = download?;
            if self.entries_limit != 0 && entries.len() >= self.entries_limit {
                log::warn!(
                    "Source exceeds total maximum ({}) number of entries. \
                    Aborting all other urls...",
                    self.entries_limit
                );

                active_downloads.abort_all();
                break;
            }

            entries.extend(download?.into_iter());
        }

        Ok(entries.into_iter().collect())
    }
}

async fn download_ips_list(url: Url, sources_cache: SourcesCache) -> Result<Vec<Expression>> {
    let not_older_than = sources_cache.get(&url).await;
    let provider = SourceProvider::new(url.clone())?;

    let FetchStatus::Success {
        addresses,
        modified,
    } = provider.fetch(not_older_than).await?
    else {
        return Ok(vec![]);
    };

    sources_cache.set(&url, modified).await;

    Ok(addresses.into_iter().map(Into::into).collect())
}
