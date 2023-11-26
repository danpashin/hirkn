use super::IP;
use std::{collections::HashSet, time::Duration};

#[derive(Debug)]
pub(crate) struct FetchInfo {
    pub(crate) addresses: HashSet<IP>,
    pub(crate) modified: Duration,
}

#[derive(Debug)]
pub(crate) enum FetchStatus {
    NotModified,
    Success(FetchInfo),
}

impl FetchStatus {
    pub(crate) fn unwrap(self) -> FetchInfo {
        match self {
            status @ Self::NotModified => panic!("Unwrap called at {status:?}"),
            Self::Success(info) => info,
        }
    }
}
