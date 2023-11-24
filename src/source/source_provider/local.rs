use super::IPParsable;
use std::time::Duration;
use std::{
    fs::{File, Metadata},
    io::Read,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub(crate) struct IPLocalSource {
    path: PathBuf,
    metadata: Metadata,
}

impl IPLocalSource {
    pub(crate) fn new(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let metadata = path.metadata()?;

        Ok(Self { path, metadata })
    }
}

#[async_trait]
impl IPParsable for IPLocalSource {
    type Error = anyhow::Error;

    async fn modified(&self) -> Result<Option<Duration>, Self::Error> {
        let modified = self.metadata.modified()?;
        let modified = modified.duration_since(SystemTime::UNIX_EPOCH)?;
        Ok(Some(modified))
    }

    async fn fetch_raw(&self) -> Result<String, Self::Error> {
        let buffer_size = self.metadata.len();
        let mut buffer = String::with_capacity(buffer_size.try_into()?);

        let mut file = File::open(&self.path)?;
        file.read_to_string(&mut buffer)?;

        Ok(buffer)
    }
}
