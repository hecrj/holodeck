use serde::{Deserialize, Serialize};

use std::borrow::Borrow;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Locale(pub(crate) String);

pub type Map<T = String> = BTreeMap<Locale, T>;

impl Borrow<str> for Locale {
    fn borrow(&self) -> &str {
        &self.0
    }
}
