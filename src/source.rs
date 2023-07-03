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

#[derive(Deserialize, Debug)]
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
    pub(crate) async fn download_set(&self) -> Result<Vec<Expression>> {
        let Some(first_url) = self.urls.first() else {
            return Ok(vec![])
        };

        if self.urls.len() > 1 {
            let mut active_downloads = JoinSet::new();
            for url in &self.urls {
                active_downloads.spawn(download_ips(url.clone()));
            }

            let mut results = vec![];
            while let Some(download) = active_downloads.join_next().await {
                let download = download?;
                results.extend(download?.into_iter());
            }

            Ok(results)
        } else {
            download_ips(first_url.clone()).await
        }
    }
}

async fn download_ips(url: Url) -> Result<Vec<Expression>> {
    let sources_string = if url.scheme() == "file" {
        std::fs::read_to_string(url.path())?
    } else {
        let response = reqwest::get(url.clone()).await?;
        response.text().await?
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
