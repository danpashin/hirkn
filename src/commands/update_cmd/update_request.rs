use crate::{
    config::Config,
    source::{IPParsable, SourceProvider, SourcesCache, IP},
};
use anyhow::Result;
use either::Either;
use std::{collections::HashSet, sync::Arc};

pub(crate) struct UpdateRequestBuilder {
    config: Config,
    sources_cache: Option<SourcesCache>,
    excluded_ips: Option<HashSet<IP>>,
}

pub(crate) struct UpdateRequest {
    pub(crate) config: Config,
    excluded_ips: Arc<HashSet<IP>>,
    sources_cache: SourcesCache,
}

impl UpdateRequest {
    pub(crate) fn chunk_size(&self) -> usize {
        self.config.single_run_append_max.unwrap_or(usize::MAX)
    }

    pub(crate) fn excluded_ips(&self) -> Arc<HashSet<IP>> {
        self.excluded_ips.clone()
    }

    pub(crate) fn sources_cache(&self) -> SourcesCache {
        self.sources_cache.clone()
    }
}

#[allow(unused)]
impl UpdateRequestBuilder {
    pub(crate) fn new(config: Config) -> UpdateRequestBuilder {
        UpdateRequestBuilder {
            config,
            sources_cache: None,
            excluded_ips: None,
        }
    }

    pub(crate) fn set_sources_cache(mut self, sources_cache: SourcesCache) -> Self {
        self.sources_cache = Some(sources_cache);
        self
    }

    pub(crate) fn set_excluded_ips(mut self, excluded_ips: HashSet<IP>) -> Self {
        self.excluded_ips = Some(excluded_ips);
        self
    }

    pub(crate) async fn build(mut self) -> Result<UpdateRequest> {
        let excluded_ips = match self.config.excluded_ips.take() {
            Some(Either::Left(url)) => {
                let provider = SourceProvider::new(url)?;

                // unwrap is safe here 'cause result will never equal to NotModified
                // since there's no cache
                provider.fetch(None).await?.unwrap().addresses
            }
            Some(Either::Right(excluded)) => excluded,
            None => HashSet::new(),
        };

        Ok(UpdateRequest {
            config: self.config,
            sources_cache: self.sources_cache.unwrap_or_default(),
            excluded_ips: Arc::new(excluded_ips),
        })
    }
}
