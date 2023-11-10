mod cache;
mod ip;

pub(crate) use self::{cache::Cache as SourcesCache, ip::IP};
use anyhow::Result;
use nftables::{
    expr::{Expression, NamedExpression, Prefix},
    schema, types,
};
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
    let entries = if url.scheme() == "file" {
        download_imp::local(url, sources_cache).await?
    } else {
        download_imp::remote(url, sources_cache).await?
    };

    // Optimization step
    // Ignore empty lines and comments, verify entry is IP or subnet
    let parsed_entries: Vec<_> = entries
        .lines()
        .filter(|entry| !(entry.is_empty() || entry.starts_with('#')))
        .filter_map(|entry| entry.parse::<IP>().ok())
        .collect();

    drop(entries);

    // Collect all ip's and networks back to strings
    Ok(parsed_entries
        .into_iter()
        .map(|entry| match entry {
            IP::Plain(ip) => Expression::String(ip.to_string()),
            IP::Prefixed(ip_net) => {
                let prefix = Prefix {
                    addr: Box::new(Expression::String(ip_net.network().to_string())),
                    len: u32::from(ip_net.prefix_len()),
                };

                Expression::Named(NamedExpression::Prefix(prefix))
            }
        })
        .collect())
}

mod download_imp {
    use super::SourcesCache;
    use anyhow::Result;
    use std::time::SystemTime;
    use url::Url;

    pub(super) async fn local(url: Url, sources_cache: SourcesCache) -> Result<String> {
        let path = url.path();

        let modified_since = std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|modified_since| modified_since.as_secs());

        if let Some(modified_since) = modified_since {
            let cache_is_newer = sources_cache
                .get(&url)
                .await
                .map_or(false, |value| value >= modified_since);

            if cache_is_newer {
                return Ok(String::new());
            }

            sources_cache.set(&url, modified_since).await;
        }

        Ok(std::fs::read_to_string(url.path())?)
    }

    pub(super) async fn remote(url: Url, sources_cache: SourcesCache) -> Result<String> {
        let client = reqwest::Client::new();

        let response = client.head(url.clone()).send().await?;
        let response = response.error_for_status()?;

        #[allow(clippy::cast_sign_loss)]
        let modified_since = response
            .headers()
            .get("Last-Modified")
            .and_then(|value| value.to_str().ok())
            .and_then(|time_string| chrono::DateTime::parse_from_rfc2822(time_string).ok())
            .map(|value| value.timestamp() as u64);

        if let Some(modified_since) = modified_since {
            let cache_is_newer = sources_cache
                .get(&url)
                .await
                .map_or(false, |value| value >= modified_since);

            if cache_is_newer {
                return Ok(String::new());
            }

            sources_cache.set(&url, modified_since).await;
        }

        let response = client.get(url.clone()).send().await?;
        let response = response.error_for_status()?;

        Ok(response.text().await?)
    }
}
