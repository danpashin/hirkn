mod cache;

pub(crate) use self::cache::Cache as SourcesCache;
use anyhow::Result;
use ipnet::Ipv4Net;
use nftables::{
    expr::{Expression, NamedExpression, Prefix},
    schema, types,
};
use serde::Deserialize;
use std::collections::HashSet;
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
}

impl Source {
    pub(crate) async fn download_list(
        &self,
        sources_cache: SourcesCache,
    ) -> Result<Vec<Expression>> {
        let Some(first_url) = self.urls.first() else {
            return Ok(vec![])
        };

        if self.urls.len() > 1 {
            let mut active_downloads = JoinSet::new();
            for url in &self.urls {
                active_downloads.spawn(download_ips_list(url.clone(), sources_cache.clone()));
            }

            let mut results = vec![];
            while let Some(download) = active_downloads.join_next().await {
                let download = download?;
                results.extend(download?.into_iter());
            }

            Ok(results)
        } else {
            download_ips_list(first_url.clone(), sources_cache).await
        }
    }

    pub(crate) fn create_set(&self, table_name: &str) -> schema::Set {
        let template = self.set_template.clone();

        schema::Set {
            family: template.family,
            table: table_name.to_string(),
            name: self.set_name.clone(),
            handle: None,
            set_type: template.set_type,
            policy: template.policy,
            flags: template.flags,
            elem: None,
            timeout: template.timeout,
            gc_interval: template.gc_interval,
            size: None,
        }
    }
}

async fn download_ips_list(url: Url, sources_cache: SourcesCache) -> Result<Vec<Expression>> {
    let sources_string = if url.scheme() == "file" {
        download_imp::local(url, sources_cache).await?
    } else {
        download_imp::remote(url, sources_cache).await?
    };

    Ok(sources_string
        .lines()
        .filter_map(|entry| {
            if entry.is_empty() {
                return None;
            }

            if entry.contains('/') {
                let subnet: Ipv4Net = entry.parse().ok()?;
                Some(Expression::Named(NamedExpression::Prefix(Prefix {
                    addr: Box::new(Expression::String(subnet.network().to_string())),
                    len: u32::from(subnet.prefix_len()),
                })))
            } else {
                Some(Expression::String(entry.to_string()))
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
