use serde::{Deserialize, Serialize};

use std::borrow::{Borrow, Cow};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Locale(pub(crate) Cow<'static, str>);

impl Locale {
    pub const JA: Self = Self::from_str("ja");
    pub const EN: Self = Self::from_str("en");
    pub const ES: Self = Self::from_str("es");

    pub const fn from_str(locale: &'static str) -> Self {
        Self(Cow::Borrowed(locale))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Locale {
    fn from(locale: String) -> Self {
        Self(Cow::Owned(locale))
    }
}

impl PartialEq<&str> for Locale {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for Locale {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
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
        self.has(&Locale::EN) || self.has(&Locale::JA)
    }

    pub fn has(&self, locale: &Locale) -> bool {
        self.0.contains_key(locale)
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
