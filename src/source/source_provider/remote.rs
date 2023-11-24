use super::IPParsable;
use chrono::{DateTime, NaiveDateTime};
use reqwest::Client;
use std::time::Duration;
use url::Url;

pub(crate) struct IPRemoteSource {
    url: Url,
    client: Client,
}

impl IPRemoteSource {
    pub(crate) fn new(url: Url) -> Self {
        let client = Client::new();
        Self { url, client }
    }
}

#[async_trait]
impl IPParsable for IPRemoteSource {
    type Error = anyhow::Error;

    async fn modified(&self) -> Result<Option<Duration>, Self::Error> {
        let response = self.client.head(self.url.clone()).send().await?;
        let response = response.error_for_status()?;

        let modified = response
            .headers()
            .get("Last-Modified")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| DateTime::parse_from_rfc2822(value).ok())
            .map(|value| value.naive_utc())
            .map(|value| value.signed_duration_since(NaiveDateTime::UNIX_EPOCH))
            .and_then(|value| value.to_std().ok());

        Ok(modified)
    }

    async fn fetch_raw(&self) -> Result<String, Self::Error> {
        let response = self.client.get(self.url.clone()).send().await?;
        let response = response.error_for_status()?;

        Ok(response.text().await?)
    }
}
