use std::sync::Arc;

use std::borrow::Borrow;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Map<K, V> {
    entries: BTreeMap<K, usize>,
    values: Arc<[V]>,
}

impl<K, V> Map<K, V> {
    pub fn new(values: impl Into<Arc<[V]>>, to_key: impl Fn(&V) -> K) -> Self
    where
        K: Ord,
    {
        let values = values.into();

        Self {
            entries: BTreeMap::from_iter(
                values
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (to_key(value), i)),
            ),
            values,
        }
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        Some(&self.values[*self.entries.get(key)?])
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn values(&self) -> &[V] {
        &self.values
    }
}
