use crate::set;

use std::sync::Arc;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("request failed: {0}")]
    RequestFailed(Arc<reqwest::Error>),
    #[error("set not found: {0}")]
    SetNotFound(set::Id),
    #[error("locale is not available")]
    LocaleNotAvailable,
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::RequestFailed(Arc::new(error))
    }
}
