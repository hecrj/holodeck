use crate::pokebase::Database;
use crate::pokebase::card;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub name: Name,
    pub cards: BTreeMap<card::Id, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Name(String);

impl Name {
    pub fn parse(name: &str) -> Option<Self> {
        if name.is_empty() {
            return None;
        }

        Some(Name(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Collection {
    pub async fn create(name: Name) -> Result<Self, anywho::Error> {
        let collection = Self {
            name,
            cards: BTreeMap::new(),
        };

        let _ = collection.save().await;

        Ok(collection)
    }

    pub async fn list() -> Result<Vec<Self>, anywho::Error> {
        if !fs::try_exists(collections_path()).await? {
            return Ok(Vec::new());
        }

        Ok(ron::from_str(
            &fs::read_to_string(collections_path()).await?,
        )?)
    }

    pub fn add(&mut self, card: card::Id) {
        let amount = self.cards.entry(card).or_default();

        *amount += 1;
    }

    pub fn save<'a>(&self) -> impl Future<Output = Result<(), anywho::Error>> + 'a {
        let collection = self.clone();

        async move {
            let mut collections = Self::list().await?;

            if let Some(old) = collections
                .iter_mut()
                .find(|candidate| candidate.name == collection.name)
            {
                *old = collection;
            } else {
                collections.push(collection);
            }

            fs::create_dir_all(data_dir()).await?;

            fs::write(
                collections_path(),
                ron::ser::to_string_pretty(&collections, ron::ser::PrettyConfig::default())?,
            )
            .await?;

            Ok(())
        }
    }

    pub fn unique_cards(&self) -> usize {
        self.cards.len()
    }

    pub fn total_cards(&self) -> usize {
        self.cards.values().sum()
    }

    pub fn total_pokemon(&self, database: &Database) -> usize {
        let pokemon = BTreeSet::from_iter(
            self.cards
                .keys()
                .filter_map(|card| database.cards.get(card))
                .filter_map(|card| card.pokedex.first()),
        );

        pokemon.len()
    }
}

fn collections_path() -> PathBuf {
    data_dir().join("collections.ron")
}

fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_default().join("holobyte")
}
