use crate::locale;
use crate::series;

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Set {
    pub id: Id,
    pub name: locale::Map,
    pub series: series::Id,
    pub release_date: String,
    pub total_cards: usize,
}

pub type Map = crate::Map<Id, Set>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Id(pub(crate) String);

impl Id {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
