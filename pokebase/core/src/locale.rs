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

pub type Map<T = String> = BTreeMap<Locale, T>;

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
