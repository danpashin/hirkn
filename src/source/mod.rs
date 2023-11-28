mod cache;
mod source_provider;

use self::source_provider::FetchStatus;
pub(crate) use self::{
    cache::Cache as SourcesCache,
    source_provider::{IPParsable, SourceProvider, IP},
};
use anyhow::Result;
use nftables::{expr::Expression, schema, types};
use serde::Deserialize;
use std::{
    collections::{HashSet, LinkedList},
    sync::Arc,
};
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
    pub(crate) async fn download_list(
        &self,
        cache: SourcesCache,
        excluded: Arc<HashSet<IP>>,
    ) -> Result<Vec<Expression>> {
        if self.urls.len() == 1 {
            self.download_single_list(cache, excluded).await
        } else {
            self.download_multiple_lists(cache, excluded).await
        }
    }

    async fn download_single_list(
        &self,
        cache: SourcesCache,
        excluded: Arc<HashSet<IP>>,
    ) -> Result<Vec<Expression>> {
        // url will always exist at this moment
        // so it's safe
        let first_url = &self.urls[0];

        let mut entries = download_ips_list(first_url.clone(), cache, excluded).await?;

        if self.entries_limit != 0 && entries.len() > self.entries_limit {
            log::warn!(
                "Source {} exceeds maximum ({}) number of entries. Got {}. Truncating...",
                first_url,
                self.entries_limit,
                entries.len()
            );
            entries.truncate(self.entries_limit);
        }

        Ok(entries)
    }

    async fn download_multiple_lists(
        &self,
        cache: SourcesCache,
        excluded: Arc<HashSet<IP>>,
    ) -> Result<Vec<Expression>> {
        let mut active_downloads = JoinSet::new();
        for url in &self.urls {
            active_downloads.spawn(download_ips_list(
                url.clone(),
                cache.clone(),
                excluded.clone(),
            ));
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

async fn download_ips_list(
    url: Url,
    sources_cache: SourcesCache,
    excluded: Arc<HashSet<IP>>,
) -> Result<Vec<Expression>> {
    fn perform_filtering(mut original: HashSet<IP>, to_exclude: &HashSet<IP>) -> HashSet<IP> {
        // HashSet has O(1)~ complexity for remove operation
        // So we can remove items without any extra allocations

        // Firstly, remove raw entries presented in list
        //
        // For 1.1.1.1 this will remove corresponding 1.1.1.1 entry.
        // For 192.168.0.0/24 - corresponding 192.168.0.0/24 entry.
        for excluded_ip in to_exclude {
            original.remove(excluded_ip);
        }

        // Shrink original as retain method below has O(capacity) complexity
        original.shrink_to_fit();

        // Then remove *any* address or subnet that is contained in to_exclude list
        // For 1.1.1.1/24 this will remove all 1.1.1.1, 1.1.1.2 ... 1.1.1.255 entries
        //
        // This algo has O(original * subnets) complexity.
        // I don't know how to optimize this further yet.
        let subnets: Vec<_> = to_exclude.iter().filter_map(IP::as_network).collect();
        if !subnets.is_empty() {
            original.retain(|ip| {
                !subnets.iter().any(|subnet| match ip {
                    IP::Single(ip) => subnet.contains(ip),
                    IP::Network(ip) => subnet.contains(ip),
                })
            });
        }

        original
    }

    let not_older_than = sources_cache.get(&url).await;
    let provider = SourceProvider::new(url.clone())?;

    let FetchStatus::Success(info) = provider.fetch(not_older_than).await? else {
        return Ok(vec![]);
    };

    sources_cache.set(&url, info.modified).await;

    let filtered_ips = perform_filtering(info.addresses, &excluded);

    Ok(filtered_ips.into_iter().map(Into::into).collect())
}

#[cfg(test)]
mod tests {
    use super::{SetTemplate, Source, SourcesCache};
    use std::collections::HashSet;
    use std::sync::Arc;
    use url::Url;

    #[tokio::test]
    async fn download_local_and_filter() {
        let source_path = "/tmp/hirkn_download_local_and_filter.txt";
        let ips = "
        # IPv4
        192.168.0.1
        192.168.0.2
        192.168.0.3
        192.168.0.14
        192.168.0.15
        192.168.1.1

        # Subnets
        10.10.0.0/16
        10.11.0.0/16

        # IPv6
        97e6:2566:e5dd:048e:22d7:7b0f:950b:1907
        97e6:2566:e5dd:048e:97e6:2566:e5dd:048e
        ::1

        # Must be presented after filtering
        192.168.1.2
        ::2
        11.10.0.0/16
        ";

        std::fs::write(source_path, ips).unwrap();

        let set = Source {
            set_name: "test_set".to_string(),
            set_template: SetTemplate::default(),
            urls: vec![Url::from_file_path(source_path).unwrap()],
            entries_limit: 0,
        };

        let excluded = Arc::new(HashSet::from([
            // IPv4 single address and subnet
            "192.168.1.1".parse().unwrap(),
            "192.168.0.10/28".parse().unwrap(),
            // IPv6 single address and subnet
            "::1".parse().unwrap(),
            "97e6:2566:e5dd:48e::/64".parse().unwrap(),
            // Subnets
            "10.0.0.0/8".parse().unwrap(),
        ]));

        let cache = SourcesCache::default();
        let downloaded = set.download_list(cache, excluded).await.unwrap();

        std::fs::remove_file(source_path).unwrap();

        // 192.168.1.1
        // ::2
        // 11.10.0.0/16
        assert_eq!(downloaded.len(), 3);
    }
}
