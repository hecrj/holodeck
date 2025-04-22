use crate::locale;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pokemon {
    pub id: Id,
    pub name: locale::Map,
}

impl Pokemon {
    pub fn name(&self) -> &str {
        self.name
            .get("en")
            .expect("en locale must always be available")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(pub(crate) u32);
