use crate::Collection;
use crate::card;
use crate::pokebase::card::pricing;
use crate::pokebase::{Card, Database, Error, Session};

use anywho::anywho;
use futures_util::{SinkExt, Stream, TryFutureExt};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::task;
use tokio::time;

use std::collections::HashMap;
use std::fmt;
use std::ops;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Clone, Default)]
pub struct Map(HashMap<card::Id, Pricing>);

impl Map {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: &card::Id) -> Option<Pricing> {
        self.0.get(id).copied()
    }

    pub fn contains(&self, id: &card::Id) -> bool {
        self.0.contains_key(id)
    }

    pub fn insert(&mut self, id: card::Id, pricing: Pricing) -> Option<Pricing> {
        self.0.insert(id, pricing)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn total_value(&self, collection: &Collection) -> Value {
        let prices = collection
            .cards
            .iter()
            .filter_map(|(card, amount)| Some((self.get(card)?, *amount as u64)));

        let america = prices
            .clone()
            .filter_map(|(pricing, amount)| Some(pricing.america.spread()?.average * amount))
            .fold(Dollars::ZERO, ops::Add::add);

        let europe = prices
            .filter_map(|(pricing, amount)| Some(pricing.europe.spread()?.average * amount))
            .fold(Euros::ZERO, ops::Add::add);

        Value { america, europe }
    }

    pub fn most_expensive<'a>(
        &self,
        collection: &Collection,
        database: &'a Database,
    ) -> impl Iterator<Item = &'a Card> {
        let mut cards: Vec<_> = collection
            .cards
            .keys()
            .filter_map(|card| database.cards.get(card))
            .collect();

        cards.sort_by(|a, b| {
            let price_a = self
                .get(&a.id)
                .and_then(|pricing| pricing.america.spread())
                .unwrap_or_default()
                .average;

            let price_b = self
                .get(&b.id)
                .and_then(|pricing| pricing.america.spread())
                .unwrap_or_default()
                .average;

            price_a.cmp(&price_b).reverse()
        });
        cards.into_iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Value {
    pub america: Dollars,
    pub europe: Euros,
}

impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Map")
            .field("prices", &self.0.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pricing {
    pub america: Variants<Dollars>,
    pub europe: Variants<Euros>,
    pub updated_at: SystemTime,
}

#[derive(Serialize, Deserialize)]
struct Cache {
    tcgplayer: pricing::tcgplayer::Pricing,
    cardmarket: pricing::cardmarket::Pricing,
    updated_at: SystemTime,
}

impl Pricing {
    pub async fn list() -> Result<Map, anywho::Error> {
        let collections = Collection::list().await?;

        let mut prices = Map::new();

        let cards = collections
            .into_iter()
            .flat_map(|collection| collection.cards.into_keys());

        for card in cards {
            if let Ok(pricing) = Self::fetch_cache(&card).await {
                prices.insert(card, pricing);
            }
        }

        Ok(prices)
    }

    pub fn fetch<'a>(
        card: &Card,
        session: &Session,
    ) -> impl Future<Output = Result<Self, anywho::Error>> + 'a {
        let card = card.clone();
        let session = session.clone();

        async move {
            let fetch_from_cache = Self::fetch_cache(&card.id);

            let fetch_remotely = async {
                let pricing = match pricing::Pricing::fetch(&card, &session).await {
                    Ok(pricing) => pricing,
                    Err(Error::LocaleNotAvailable) => {
                        log::warn!("Locale not available for {id}", id = card.id.as_str());
                        pricing::Pricing::default()
                    }
                    Err(Error::RequestFailed(error))
                        if error.status() == Some(reqwest::StatusCode::NOT_FOUND) =>
                    {
                        log::warn!("Pricing for {id} not found", id = card.id.as_str());
                        pricing::Pricing::default()
                    }
                    Err(error) => Err(error)?,
                };

                let cache = cache_dir().join(format!("{}.ron", card.id.as_str()));

                if !fs::try_exists(&cache).await.unwrap_or(false) {
                    let _ = fs::create_dir_all(cache.parent().unwrap_or(&cache)).await;
                    let _ = fs::write(
                        cache,
                        ron::ser::to_string_pretty(
                            &Cache {
                                tcgplayer: pricing.tcgplayer,
                                cardmarket: pricing.cardmarket,
                                updated_at: SystemTime::now(),
                            },
                            ron::ser::PrettyConfig::default(),
                        )
                        .expect("Serialize pricing cache"),
                    )
                    .await;
                }

                Ok(Self::from_raw(pricing))
            };

            fetch_from_cache.or_else(|_| fetch_remotely).await
        }
    }

    pub fn subscribe<'a>(
        database: &Database,
        session: &Session,
    ) -> impl Stream<Item = (card::Id, Pricing)> + 'a {
        let database = database.clone();
        let session = session.clone();

        iced::stream::channel(1, async move |mut sender| {
            let mut prices = loop {
                log::debug!("Scanning prices cache...");

                let Ok(prices) = Pricing::list().await else {
                    time::sleep(Duration::from_secs(60)).await;
                    continue;
                };

                break prices;
            };

            log::info!("Fetched {} prices from cache", prices.len());

            loop {
                log::debug!("Scanning for out of date prices...");

                let Ok(collections) = Collection::list().await else {
                    time::sleep(Duration::from_secs(60)).await;
                    continue;
                };

                let cards = collections
                    .into_iter()
                    .flat_map(|collection| collection.cards.into_keys());

                let mut i = 0;

                for card in cards {
                    if prices
                        .get(&card)
                        .is_some_and(|pricing| !is_outdated(pricing.updated_at))
                    {
                        continue;
                    }

                    let Some(card) = database.cards.get(&card) else {
                        continue;
                    };

                    if let Ok(pricing) = Pricing::fetch(card, &session).await {
                        prices.insert(card.id.clone(), pricing);
                        let _ = sender.send((card.id.clone(), pricing)).await;
                    }

                    i += 1;

                    if i % 10 == 0 {
                        time::sleep(Duration::from_secs(30)).await;
                    }
                }

                time::sleep(Duration::from_secs(30)).await;
            }
        })
    }

    async fn fetch_cache(card: &card::Id) -> Result<Self, anywho::Error> {
        let cache = cache_dir().join(format!("{}.ron", card.as_str()));

        let pricing = fs::read_to_string(&cache).await?;

        let cache: Cache = task::spawn_blocking(move || ron::from_str(&pricing)).await??;

        if is_outdated(cache.updated_at) {
            return Err(anywho!("Pricing cache for {card:?} is outdated"));
        }

        Ok(Self::from_raw(pricing::Pricing {
            tcgplayer: cache.tcgplayer,
            cardmarket: cache.cardmarket,
            updated_at: cache.updated_at,
        }))
    }

    fn from_raw(pricing: pricing::Pricing) -> Self {
        let pricing::Pricing {
            tcgplayer,
            cardmarket,
            updated_at,
        } = pricing;

        let america = {
            let spread = |spread: pricing::tcgplayer::Spread| Spread {
                low: Dollars::new(spread.low),
                high: Dollars::new(spread.high),
                average: Dollars::new(spread.mid),
                market: Dollars::new(spread.market),
            };

            Variants {
                normal: tcgplayer.prices.normal.map(spread),
                holofoil: tcgplayer.prices.holofoil.map(spread),
                reverse_holofoil: tcgplayer.prices.reverse_holofoil.map(spread),
            }
        };

        let europe = {
            let normal_or_holofoil = Spread {
                low: Euros::new(cardmarket.prices.low_price),
                high: Euros::new(
                    cardmarket
                        .prices
                        .trend_price
                        .max(cardmarket.prices.avg1)
                        .max(cardmarket.prices.avg7)
                        .max(cardmarket.prices.avg30)
                        .max(cardmarket.prices.average_sell_price),
                ),
                average: Euros::new(cardmarket.prices.average_sell_price),
                market: Euros::new(cardmarket.prices.trend_price),
            };

            let reverse_holofoil = Spread {
                low: Euros::new(cardmarket.prices.reverse_holo_low),
                high: Euros::new(
                    cardmarket
                        .prices
                        .reverse_holo_trend
                        .max(cardmarket.prices.reverse_holo_avg1)
                        .max(cardmarket.prices.reverse_holo_avg7)
                        .max(cardmarket.prices.reverse_holo_avg30)
                        .max(cardmarket.prices.reverse_holo_sell),
                ),
                average: Euros::new(cardmarket.prices.reverse_holo_sell),
                market: Euros::new(cardmarket.prices.reverse_holo_trend),
            };

            Variants {
                normal: tcgplayer
                    .prices
                    .normal
                    .is_some()
                    .then_some(normal_or_holofoil),
                holofoil: tcgplayer
                    .prices
                    .holofoil
                    .is_some()
                    .then_some(normal_or_holofoil),
                reverse_holofoil: tcgplayer
                    .prices
                    .reverse_holofoil
                    .is_some()
                    .then_some(reverse_holofoil),
            }
        };

        Self {
            america,
            europe,
            updated_at,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variants<T> {
    normal: Option<Spread<T>>,
    holofoil: Option<Spread<T>>,
    reverse_holofoil: Option<Spread<T>>,
}

impl<T> Variants<T> {
    pub fn spread(self) -> Option<Spread<T>> {
        self.normal.or(self.holofoil).or(self.reverse_holofoil)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Spread<T> {
    pub low: T,
    pub high: T,
    pub average: T,
    pub market: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Dollars {
    cents: u64,
}

impl Dollars {
    pub const ZERO: Self = Self { cents: 0 };

    pub fn new(dollars: f64) -> Self {
        Self {
            cents: (dollars * 100.0).round() as u64,
        }
    }
}

impl ops::Add for Dollars {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            cents: self.cents + rhs.cents,
        }
    }
}

impl ops::Mul<u64> for Dollars {
    type Output = Self;

    fn mul(self, rhs: u64) -> Self::Output {
        Self {
            cents: self.cents * rhs,
        }
    }
}

impl fmt::Display for Dollars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:.2}", self.cents as f64 / 100.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Euros {
    cents: u64,
}

impl Euros {
    pub const ZERO: Self = Self { cents: 0 };

    pub fn new(euros: f64) -> Self {
        Self {
            cents: (euros * 100.0).round() as u64,
        }
    }
}

impl ops::Add for Euros {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            cents: self.cents + rhs.cents,
        }
    }
}

impl ops::Mul<u64> for Euros {
    type Output = Self;

    fn mul(self, rhs: u64) -> Self::Output {
        Self {
            cents: self.cents * rhs,
        }
    }
}

impl fmt::Display for Euros {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}€", self.cents as f64 / 100.0)
    }
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_default()
        .join(env!("CARGO_PKG_NAME"))
        .join("prices")
}

fn is_outdated(updated_at: SystemTime) -> bool {
    const WEEK: Duration = Duration::from_secs(60 * 60 * 24 * 7);
    updated_at.elapsed().unwrap_or_default() > WEEK
}
