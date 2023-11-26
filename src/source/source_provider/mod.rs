mod info;
mod ip;
mod local;
mod remote;

pub(crate) use self::{
    info::{FetchInfo, FetchStatus},
    ip::IP,
};
use self::{local::IPLocalSource, remote::IPRemoteSource};
use std::time::{Duration, SystemTime};
use url::Url;

#[async_trait]
pub(crate) trait IPParsable {
    type Error;

    async fn modified(&self) -> Result<Option<Duration>, Self::Error>;

    async fn fetch_raw(&self) -> Result<String, Self::Error>;

    async fn fetch(&self, not_older_than: Option<Duration>) -> Result<FetchStatus, Self::Error> {
        let modified = self.modified().await?.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
        });

        if let Some(not_older_than) = not_older_than {
            if not_older_than >= modified {
                return Ok(FetchStatus::NotModified);
            }
        }

        let raw_list = self.fetch_raw().await?;

        let addresses = raw_list
            .lines()
            .filter(|entry| !(entry.is_empty() || entry.starts_with('#')))
            .filter_map(|entry| entry.parse::<IP>().ok())
            .collect();

        Ok(FetchStatus::Success(FetchInfo {
            addresses,
            modified,
        }))
    }
}

pub(crate) enum SourceProvider {
    Local(IPLocalSource),
    Remote(IPRemoteSource),
}

impl SourceProvider {
    pub(crate) fn new(url: Url) -> anyhow::Result<Self> {
        if url.scheme() == "file" {
            Ok(Self::Local(IPLocalSource::new(url.path())?))
        } else {
            Ok(Self::Remote(IPRemoteSource::new(url)))
        }
    }
}

#[async_trait]
impl IPParsable for SourceProvider {
    type Error = anyhow::Error;

    async fn modified(&self) -> Result<Option<Duration>, Self::Error> {
        match self {
            Self::Local(parser) => parser.modified().await,
            Self::Remote(parser) => parser.modified().await,
        }
    }

    async fn fetch_raw(&self) -> Result<String, Self::Error> {
        match self {
            Self::Local(parser) => parser.fetch_raw().await,
            Self::Remote(parser) => parser.fetch_raw().await,
        }
    }
}
