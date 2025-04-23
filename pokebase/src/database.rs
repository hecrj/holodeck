use crate::card;
use crate::locale;
use crate::pokemon;
use crate::series;
use crate::set;
use crate::{Card, Locale, Map, Pokemon, Series, Set};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct Database {
    pub pokemon: Map<pokemon::Id, Pokemon>,
    pub series: Map<series::Id, Series>,
    pub sets: Map<set::Id, Set>,
    pub cards: Map<card::Id, Card>,
}

impl Database {
    #[cfg(feature = "static")]
    pub async fn load() -> Result<Self, anywho::Error> {
        let pokemon = load_pokemon()?;
        let series: Vec<Series> = ron::de::from_str(include_str!("../data/series.ron"))?;
        let sets: Vec<Set> = ron::de::from_str(include_str!("../data/sets.ron"))?;
        let cards: Vec<Card> = ron::de::from_str(include_str!("../data/cards.ron"))?;

        Ok(Self {
            pokemon: Map::new(pokemon, |pokemon| pokemon.id.clone()),
            series: Map::new(series, |series| series.id.clone()),
            sets: Map::new(sets, |set| set.id.clone()),
            cards: Map::new(cards, |card| card.id.clone()),
        })
    }

    #[cfg(not(feature = "static"))]
    pub async fn load() -> Result<Self, anywho::Error> {
        use std::path::PathBuf;
        use tokio::fs;
        use tokio::task;

        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");

        let pokemon = task::spawn_blocking(load_pokemon).await??;

        let series: Vec<Series> = {
            let contents = fs::read_to_string(data.join("series.ron")).await?;
            task::spawn_blocking(move || ron::de::from_str(&contents)).await??
        };

        let sets: Vec<Set> = {
            let contents = fs::read_to_string(data.join("sets.ron")).await?;
            task::spawn_blocking(move || ron::de::from_str(&contents)).await??
        };

        let cards: Vec<Card> = {
            let contents = fs::read_to_string(data.join("cards.ron")).await?;
            task::spawn_blocking(move || ron::de::from_str(&contents)).await??
        };

        Ok(Self {
            pokemon: Map::new(pokemon, |pokemon| pokemon.id.clone()),
            series: Map::new(series, |series| series.id.clone()),
            sets: Map::new(sets, |set| set.id.clone()),
            cards: Map::new(cards, |card| card.id.clone()),
        })
    }

    pub fn generate(data: impl AsRef<Path>) -> Result<Self, anywho::Error> {
        use std::fs::{self, File};
        use std::io::BufReader;

        let pokemon = load_pokemon()?;

        let mut series: BTreeMap<String, Series> = BTreeMap::new();
        let mut sets: BTreeMap<String, Set> = BTreeMap::new();
        let mut cards: BTreeMap<String, Card> = BTreeMap::new();

        let entries = fs::read_dir(&data)?;

        for entry in entries {
            let entry = entry?;

            if !entry.metadata()?.is_dir() {
                continue;
            }

            let locale = Locale(entry.file_name().to_string_lossy().to_string());
            dbg!(&locale);

            // Series
            #[derive(Serialize, Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct LocalizedSeries {
                id: String,
                name: String,
                release_date: String,
            }

            let localized_series_list: Vec<LocalizedSeries> = {
                let file = BufReader::new(File::open(entry.path().join("series.json"))?);
                serde_json::from_reader(file)?
            };

            for localized_series in localized_series_list {
                let series = series
                    .entry(localized_series.id.clone())
                    .or_insert_with(|| Series {
                        id: series::Id(localized_series.id),
                        name: BTreeMap::new(),
                        release_date: localized_series.release_date,
                    });

                series.name.insert(locale.clone(), localized_series.name);
            }

            // Sets
            #[derive(Serialize, Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct LocalizedSet {
                id: String,
                name: String,
                serie: Serie,
                release_date: String,
                card_count: CardCount,
            }

            #[derive(Serialize, Deserialize)]
            struct Serie {
                id: String,
            }

            #[derive(Serialize, Deserialize)]
            struct CardCount {
                total: usize,
            }

            let localized_sets: Vec<LocalizedSet> = {
                let file = BufReader::new(File::open(entry.path().join("sets.json"))?);
                serde_json::from_reader(file)?
            };

            for localized_set in localized_sets {
                let set = sets.entry(localized_set.id.clone()).or_insert_with(|| Set {
                    id: set::Id(localized_set.id),
                    name: BTreeMap::new(),
                    series: series::Id(localized_set.serie.id),
                    release_date: localized_set.release_date,
                    total_cards: localized_set.card_count.total,
                });

                set.name.insert(locale.clone(), localized_set.name);
            }

            // Cards
            #[derive(Serialize, Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct LocalizedCard {
                id: String,
                name: String,
                set: CardSet,
                #[serde(default)]
                rarity: Option<String>,
                #[serde(default)]
                types: Vec<String>,
                variants: LocalizedVariants,
                #[serde(default)]
                illustrator: Option<String>,
                #[serde(default)]
                dex_id: Vec<pokemon::Id>,
            }

            #[derive(Serialize, Deserialize)]
            struct CardSet {
                id: String,
            }

            #[derive(Serialize, Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct LocalizedVariants {
                first_edition: bool,
                holo: bool,
                normal: bool,
                reverse: bool,
                w_promo: bool,
            }

            let localized_cards: Vec<LocalizedCard> = {
                let file = BufReader::new(File::open(entry.path().join("cards.json"))?);
                serde_json::from_reader(file)?
            };

            for localized_card in localized_cards {
                let card = cards
                    .entry(localized_card.id.clone())
                    .or_insert_with(|| Card {
                        id: card::Id(localized_card.id),
                        name: BTreeMap::new(),
                        set: set::Id(localized_card.set.id),
                        types: BTreeSet::new(),
                        rarity: card::Rarity::None,
                        variants: card::Variants {
                            first_edition: localized_card.variants.first_edition,
                            holo: localized_card.variants.holo,
                            normal: localized_card.variants.normal,
                            reverse: localized_card.variants.reverse,
                            w_promo: localized_card.variants.w_promo,
                        },
                        illustrator: localized_card.illustrator,
                        pokedex: localized_card.dex_id,
                    });

                // Fill in Pokedex entries
                if locale.0 == "en" && card.pokedex.is_empty() {
                    for pokemon in &pokemon {
                        if localized_card.name.contains(pokemon.name()) {
                            card.pokedex = vec![pokemon.id];
                            break;
                        }
                    }
                }

                card.name.insert(locale.clone(), localized_card.name);
                card.rarity = card.rarity.max(
                    localized_card
                        .rarity
                        .and_then(|rarity| parse_rarity(rarity).ok())
                        .unwrap_or_default(),
                );

                for type_ in localized_card.types {
                    if let Ok(type_) = parse_type(type_) {
                        card.types.insert(type_);
                    }
                }
            }
        }

        let mut cards: Vec<_> = cards.into_values().collect();
        cards.sort_by_key(|card| {
            sets.get(&card.set.0)
                .map(|set| {
                    format!(
                        "{release_date}-{:0>5}",
                        card.id.0.split("-").last().unwrap_or_default(),
                        release_date = set.release_date,
                    )
                })
                .unwrap_or_default()
        });

        let mut series: Vec<_> = series.into_values().collect();
        series.sort_by(|a, b| a.release_date.cmp(&b.release_date));

        let mut sets: Vec<_> = sets.into_values().collect();
        sets.sort_by(|a, b| a.release_date.cmp(&b.release_date));

        Ok(Self {
            pokemon: Map::new(pokemon, |pokemon| pokemon.id.clone()),
            series: Map::new(series, |series| series.id.clone()),
            sets: Map::new(sets, |set| set.id.clone()),
            cards: Map::new(cards, |card| card.id.clone()),
        })
    }

    pub fn search_cards<'a>(&self, query: &str) -> impl Future<Output = Search<Card>> + 'a {
        use tokio::task;

        let database = self.clone();
        let query = query.to_lowercase();

        async move {
            let mut matches = Vec::new();

            for card in database.cards.values() {
                if card
                    .name
                    .values()
                    .any(|name| name.as_str().to_lowercase().contains(&query))
                {
                    matches.push(card.clone());
                }

                // Avoid blocking
                task::yield_now().await;
            }

            Search {
                matches: Arc::from(matches),
            }
        }
    }
}

fn parse_type(type_: String) -> Result<card::Type, String> {
    Ok(match type_.as_str() {
        "Grass" => card::Type::Grass,
        "Fire" => card::Type::Fire,
        "Water" => card::Type::Water,
        "Lightning" => card::Type::Lightning,
        "Psychic" => card::Type::Psychic,
        "Fighting" => card::Type::Fighting,
        "Darkness" => card::Type::Darkness,
        "Metal" => card::Type::Metal,
        "Fairy" => card::Type::Fairy,
        "Dragon" => card::Type::Dragon,
        "Colorless" => card::Type::Colorless,
        _ => {
            dbg!(&type_);

            Err(format!("invalid type: {type_}"))?
        }
    })
}

fn parse_rarity(rarity: String) -> Result<card::Rarity, String> {
    Ok(match rarity.as_str() {
        "None" => card::Rarity::None,
        "Common" | "One Diamond" => card::Rarity::Common,
        "Uncommon" | "Two Diamond" => card::Rarity::Uncommon,
        "Rare" | "Three Diamond" => card::Rarity::Rare,
        "Holo Rare" | "Rare Holo" => card::Rarity::HoloRare,
        "Rare Holo LV.X" => card::Rarity::HoloRareLvx,
        "Holo Rare V" => card::Rarity::HoloRareV,
        "Holo Rare VMAX" => card::Rarity::HoloRareVmax,
        "Holo Rare VSTAR" => card::Rarity::HoloRareVstar,
        "Shiny rare" | "One Shiny" => card::Rarity::ShinyRare,
        "Shiny rare V" => card::Rarity::ShinyRareV,
        "Shiny rare VMAX" => card::Rarity::ShinyRareVmax,
        "Double rare" => card::Rarity::DoubleRare,
        "ACE SPEC Rare" => card::Rarity::AceSpecRare,
        "Amazing Rare" => card::Rarity::AmazingRare,
        "Radiant Rare" => card::Rarity::RadiantRare,
        "Rare PRIME" => card::Rarity::RarePrime,
        "LEGEND" => card::Rarity::Legend,
        "Classic Collection" => card::Rarity::ClassicCollection,
        "Ultra Rare" | "Four Diamond" => card::Rarity::UltraRare,
        "Shiny Ultra Rare" | "Two Shiny" => card::Rarity::ShinyUltraRare,
        "Secret Rare" => card::Rarity::SecretRare,
        "Full Art Trainer" => card::Rarity::FullArtTrainer,
        "Illustration rare" | "One Star" => card::Rarity::IllustrationRare,
        "Special illustration rare" | "Two Star" | "Three Star" => {
            card::Rarity::SpecialIllustrationRare
        }
        "Hyper rare" | "Crown" => card::Rarity::HyperRare,
        _ => {
            dbg!(&rarity);

            Err(format!("invalid rarity: {rarity}"))?
        }
    })
}

fn load_pokemon() -> Result<Vec<Pokemon>, anywho::Error> {
    let pokemon: Vec<String> = ron::from_str(include_str!("../data/pokemon.ron"))?;

    Ok(pokemon
        .into_iter()
        .enumerate()
        .map(|(i, name)| Pokemon {
            id: pokemon::Id(i as u32 + 1),
            name: locale::Map::from_iter([(Locale("en".to_owned()), name)]),
        })
        .collect())
}

impl fmt::Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Database")
            .field("pokemon", &self.pokemon.len())
            .field("series", &self.series.len())
            .field("sets", &self.sets.len())
            .field("cards", &self.cards.len())
            .finish()
    }
}

pub struct Search<T> {
    pub matches: Arc<[T]>,
}

impl<T> Clone for Search<T> {
    fn clone(&self) -> Self {
        Self {
            matches: self.matches.clone(),
        }
    }
}

impl<T> fmt::Debug for Search<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Search")
            .field("matches", &self.matches.len())
            .finish()
    }
}
