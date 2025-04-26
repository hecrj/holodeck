use std::fmt;
use std::sync::Arc;

pub struct Search<T> {
    pub matches: Arc<[T]>,
}

impl<T> Clone for Search<T> {
    fn clone(&self) -> Self {
        Self {
            matches: self.matches.clone(),
        }
    }
}

impl<T> fmt::Debug for Search<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Search")
            .field("matches", &self.matches.len())
            .finish()
    }
}
