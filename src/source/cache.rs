use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use url::Url;

#[derive(Clone)]
pub(crate) struct Cache {
    states: Arc<RwLock<HashMap<Url, Duration>>>,
}

impl Cache {
    pub(crate) fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(crate) async fn get(&self, url: &Url) -> Option<Duration> {
        let states = self.states.read().await;
        states.get(url).map(ToOwned::to_owned)
    }

    pub(crate) async fn set(&self, url: &Url, timestamp: Duration) {
        let mut states = self.states.write().await;
        states.insert(url.clone(), timestamp);
    }
}
