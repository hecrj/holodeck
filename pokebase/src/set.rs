use crate::locale;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Set {
    pub id: Id,
    pub name: locale::Map,
    pub series: Id,
    pub release_date: String,
    pub total_cards: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Id(pub(crate) String);
