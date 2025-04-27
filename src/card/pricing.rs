use crate::Collection;
use crate::card;
use crate::pokebase::card::pricing;
use crate::pokebase::{Card, Database, Session};

use futures_util::{Stream, TryFutureExt};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::task;
use tokio::time;

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub type Map = HashMap<card::Id, Pricing>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pricing {
    pub america: Variants<Dollars>,
    pub europe: Variants<Euros>,
    pub updated_at: SystemTime,
}

impl Pricing {
    pub fn fetch<'a>(
        card: &Card,
        session: &Session,
    ) -> impl Future<Output = Result<Self, anywho::Error>> + 'a {
        let card = card.clone();
        let session = session.clone();

        async move {
            #[derive(Serialize, Deserialize)]
            struct Cache {
                tcgplayer: pricing::tcgplayer::Pricing,
                cardmarket: pricing::cardmarket::Pricing,
            }

            let cache = cache_dir().join(card.id.as_str()).with_extension("ron");

            let fetch_from_cache = async {
                let pricing = fs::read_to_string(&cache).await?;

                task::spawn_blocking(move || {
                    let cache: Cache = ron::from_str(&pricing)?;

                    Ok::<_, anywho::Error>(pricing::Pricing {
                        tcgplayer: cache.tcgplayer,
                        cardmarket: cache.cardmarket,
                    })
                })
                .await?
            };

            let fetch_remotely = async move {
                let pricing = pricing::Pricing::fetch(&card, &session).await?;

                Ok::<_, anywho::Error>(pricing)
            };

            let pricing = fetch_from_cache.or_else(|_| fetch_remotely).await?;

            if !fs::try_exists(&cache).await.unwrap_or(false) {
                let _ = fs::create_dir_all(cache.parent().unwrap_or(&cache)).await;
                let _ = fs::write(
                    cache,
                    ron::ser::to_string_pretty(
                        &Cache {
                            tcgplayer: pricing.tcgplayer,
                            cardmarket: pricing.cardmarket,
                        },
                        ron::ser::PrettyConfig::default(),
                    )
                    .expect("Serialize pricing cache"),
                )
                .await;
            }

            let pricing::Pricing {
                tcgplayer,
                cardmarket,
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

            Ok(Pricing {
                america,
                europe,
                updated_at: SystemTime::now(),
            })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread<T> {
    pub low: T,
    pub high: T,
    pub average: T,
    pub market: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dollars {
    cents: u64,
}

impl Dollars {
    pub fn new(dollars: f64) -> Self {
        Self {
            cents: (dollars * 100.0).round() as u64,
        }
    }
}

impl fmt::Display for Dollars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:.2}", self.cents as f64 / 100.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Euros {
    cents: u64,
}

impl Euros {
    pub fn new(euros: f64) -> Self {
        Self {
            cents: (euros * 100.0).round() as u64,
        }
    }
}

impl fmt::Display for Euros {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}€", self.cents as f64 / 100.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub card: card::Id,
    pub pricing: Pricing,
}

impl Update {
    pub fn subscribe<'a>(database: &Database, session: &Session) -> impl Stream<Item = Self> + 'a {
        let database = database.clone();
        let session = session.clone();

        iced::stream::channel(1, async move |_sender| {
            let cache = cache_dir();
            let _ = fs::create_dir_all(&cache).await;

            let mut prices = loop {
                log::info!("Scanning prices cache...");

                let Ok(mut entries) = fs::read_dir(&cache).await else {
                    time::sleep(Duration::from_secs(60)).await;
                    continue;
                };

                let mut prices = HashMap::new();

                while let Ok(Some(entry)) = entries.next_entry().await {
                    match entry.path().extension() {
                        Some(extension) if extension == "ron" => {
                            if let Ok(updated_at) = entry
                                .metadata()
                                .await
                                .and_then(|metadata| metadata.modified())
                            {
                                if let Some(card) = entry.path().file_stem() {
                                    prices.insert(card.to_string_lossy().into_owned(), updated_at);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                break prices;
            };

            log::info!("Found {} prices in cache", prices.len());

            loop {
                const WEEK: Duration = Duration::from_secs(60 * 60 * 24 * 7);
                log::info!("Scanning for out of date prices...");

                let Ok(collections) = Collection::list().await else {
                    time::sleep(Duration::from_secs(60)).await;
                    continue;
                };

                let cards = collections
                    .into_iter()
                    .flat_map(|collection| collection.cards.into_keys());

                for card in cards {
                    match prices.get(card.as_str()) {
                        Some(updated_at) if updated_at.elapsed().unwrap_or_default() < WEEK => {
                            continue;
                        }
                        _ => {}
                    }

                    let Some(card) = database.cards.get(&card) else {
                        continue;
                    };

                    let _ = dbg!(Pricing::fetch(card, &session).await);

                    prices.insert(card.id.as_str().to_owned(), SystemTime::now());

                    time::sleep(Duration::from_secs(60)).await;
                }

                time::sleep(Duration::from_secs(60)).await;
            }
        })
    }
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_default()
        .join(env!("CARGO_PKG_NAME"))
        .join("prices")
}
