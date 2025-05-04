use serde::{Deserialize, Serialize};

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Locale(pub(crate) String);

impl Locale {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Map<T = String>(BTreeMap<Locale, T>);

impl<T> Map<T> {
    pub fn new() -> Self
    where
        T: Default,
    {
        Self::default()
    }

    pub fn insert(&mut self, locale: Locale, value: T) -> Option<T> {
        self.0.insert(locale, value)
    }

    pub fn get<Q>(&self, locale: &Q) -> Option<&T>
    where
        Locale: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.0.get(locale)
    }

    pub fn contains(&self, query: &str) -> bool
    where
        T: AsRef<str>,
    {
        self.0
            .values()
            .any(|name| name.as_ref().to_lowercase().contains(query))
    }

    pub fn is_supported(&self) -> bool {
        self.has_english() || self.has_japanese()
    }

    pub fn has_english(&self) -> bool {
        self.0.contains_key("en")
    }

    pub fn has_japanese(&self) -> bool {
        self.0.contains_key("ja")
    }

    pub fn locales(&self) -> impl Iterator<Item = &Locale> {
        self.0.keys()
    }

    pub fn as_str(&self) -> &str
    where
        T: AsRef<str>,
    {
        self.get("en")
            .or_else(|| self.get("ja"))
            .or_else(|| self.0.values().next())
            .map(AsRef::as_ref)
            .unwrap_or("Unknown")
    }
}

impl Borrow<str> for Locale {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<T> FromIterator<(Locale, T)> for Map<T> {
    fn from_iter<I: IntoIterator<Item = (Locale, T)>>(iter: I) -> Self {
        Self(BTreeMap::from_iter(iter))
    }
}
