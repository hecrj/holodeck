use std::sync::Arc;

use std::borrow::Borrow;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Map<K, V>(Arc<Inner<K, V>>);

#[derive(Debug)]
struct Inner<K, V> {
    entries: BTreeMap<K, usize>,
    values: Arc<[V]>,
}

impl<K, V> Map<K, V> {
    pub fn new(values: impl Into<Arc<[V]>>, to_key: impl Fn(&V) -> K) -> Self
    where
        K: Ord,
    {
        let values = values.into();

        Self(Arc::new(Inner {
            entries: BTreeMap::from_iter(
                values
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (to_key(value), i)),
            ),
            values,
        }))
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        Some(&self.0.values[*self.0.entries.get(key)?])
    }

    pub fn len(&self) -> usize {
        self.0.values.len()
    }

    pub fn values(&self) -> &[V] {
        &self.0.values
    }
}

impl<K, V> Clone for Map<K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
