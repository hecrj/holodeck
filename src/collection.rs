use crate::pokebase::card;
use crate::pokebase::pokemon;
use crate::pokebase::{Card, Database, Pokemon};

use jiff::Timestamp;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use tokio::fs;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
pub struct Collection {
    pub name: Name,
    pub captures: Vec<Capture>,

    #[serde(skip)]
    amounts: BTreeMap<card::Id, Amount>,
    #[serde(skip)]
    total_pokemon: Mutex<Option<usize>>,
    #[serde(skip)]
    rarest_card_by_pokemon: Mutex<BTreeMap<pokemon::Id, Option<card::Id>>>,
}

impl Clone for Collection {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            captures: self.captures.clone(),
            amounts: self.amounts.clone(),
            total_pokemon: Mutex::new(None),
            rarest_card_by_pokemon: Mutex::new(BTreeMap::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capture {
    card: card::Id,
    variant: Variant,
    at: Timestamp,
}

impl Collection {
    pub async fn create(name: Name) -> Result<Self, anywho::Error> {
        let collection = Self {
            name,
            captures: Vec::new(),
            amounts: BTreeMap::new(),
            rarest_card_by_pokemon: Mutex::new(BTreeMap::new()),
            total_pokemon: Mutex::new(None),
        };

        let _ = collection.save().await;

        Ok(collection)
    }

    pub async fn list() -> Result<Vec<Self>, anywho::Error> {
        if !fs::try_exists(collections_path()).await? {
            return Ok(Vec::new());
        }

        let mut collections: Vec<Self> =
            ron::from_str(&fs::read_to_string(collections_path()).await?)?;

        for collection in &mut collections {
            for capture in &collection.captures {
                let amount = collection.amounts.entry(capture.card.clone()).or_default();

                match capture.variant {
                    Variant::Normal => amount.normal += 1,
                    Variant::Reverse => amount.reverse += 1,
                }
            }
        }

        Ok(collections)
    }

    pub fn add(&mut self, card: card::Id, variant: Variant) {
        self.captures.push(Capture {
            card: card.clone(),
            variant,
            at: Timestamp::now(),
        });

        let amount = self.amounts.entry(card).or_default();

        match variant {
            Variant::Normal => amount.normal += 1,
            Variant::Reverse => amount.reverse += 1,
        }

        *self.total_pokemon.lock().unwrap() = None;
        self.rarest_card_by_pokemon.lock().unwrap().clear();
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

    pub fn cards(&self) -> impl DoubleEndedIterator<Item = (&card::Id, Amount)> + Clone {
        self.amounts.iter().map(|(card, amount)| (card, *amount))
    }

    pub fn amount(&self, card: &card::Id) -> Option<&Amount> {
        self.amounts.get(card)
    }

    pub fn unique_cards(&self) -> usize {
        self.amounts.len()
    }

    pub fn total_cards(&self) -> usize {
        self.amounts.values().copied().map(Amount::total).sum()
    }

    pub fn total_pokemon(&self, database: &Database) -> usize {
        let mut total = self.total_pokemon.lock().unwrap();

        if let Some(total) = *total {
            return total;
        }

        let pokemon = BTreeSet::from_iter(
            self.amounts
                .keys()
                .filter_map(|card| database.cards.get(card))
                .filter_map(|card| card.pokedex.first()),
        );

        *total = Some(pokemon.len());
        pokemon.len()
    }

    #[allow(dead_code)]
    pub fn rarest_cards<'a>(&'a self, database: &'a Database) -> impl Iterator<Item = &'a Card> {
        let mut rares: Vec<_> = self
            .amounts
            .keys()
            .filter_map(|card| database.cards.get(card))
            .filter(|card| card.rarity >= card::Rarity::HoloRare)
            .collect();
        rares.sort_unstable_by(|a, b| a.rarity.cmp(&b.rarity).reverse());

        rares.into_iter()
    }

    pub fn rarest_card_for<'a>(
        &self,
        pokemon: &Pokemon,
        database: &'a Database,
    ) -> Option<&'a Card> {
        let mut rarest_card_by_pokemon = self.rarest_card_by_pokemon.lock().unwrap();

        if let Some(card) = rarest_card_by_pokemon.get(&pokemon.id) {
            return database.cards.get(card.as_ref()?);
        }

        let mut cards: Vec<_> = self
            .amounts
            .keys()
            .filter_map(|card| {
                let card = database.cards.get(card)?;

                if card.pokedex.contains(&pokemon.id) {
                    Some(card)
                } else {
                    None
                }
            })
            .collect();
        cards.sort_unstable_by(|a, b| a.rarity.cmp(&b.rarity).reverse());

        let card = cards.first().copied();

        rarest_card_by_pokemon.insert(pokemon.id, card.map(|card| card.id.clone()));

        card
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Amount {
    #[serde(default, skip_serializing_if = "usize::is_zero")]
    pub normal: usize,
    #[serde(default, skip_serializing_if = "usize::is_zero")]
    pub reverse: usize,
}

impl Amount {
    pub fn total(self) -> usize {
        self.normal + self.reverse
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Variant {
    Normal,
    Reverse,
}

fn collections_path() -> PathBuf {
    data_dir().join("collections.ron")
}

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_default()
        .join(env!("CARGO_PKG_NAME"))
}
