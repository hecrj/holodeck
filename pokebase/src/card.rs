pub mod pricing;

pub use crate::core::card::*;

use crate::{Database, Error, Session};

use bytes::Bytes;
use std::fmt;
use std::sync::mpsc;
use std::sync::{Arc, LazyLock};

#[derive(Clone)]
pub struct Image {
    pub bytes: Bytes,
}

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

impl Image {
    pub async fn download(
        card: &Card,
        database: &Database,
        session: &Session,
    ) -> Result<Self, Error> {
        use futures_util::TryFutureExt;

        let download_from_pokemontcg = session.pokemon_tcg.download_image(card);
        let download_from_tcgdex = session.tcgdex.download_image(card, database);

        // Rationale on the order of image fetching:
        // 1. PokemonTCG - Highest quality, but English only. 20,000 requests/day with an API key.
        // 2. TCGdex - Lower quality, but supports multiple locales. Rate limiting unknown (?).
        let bytes = download_from_pokemontcg
            .or_else(|error| {
                log::warn!("{error}");

                download_from_tcgdex
            })
            .await?;

        Ok(Self { bytes })
    }
}

pub fn search<'a>(query: &str, database: &Database) -> impl Future<Output = Search> + 'a {
    use tokio::task;

    let query = query.to_lowercase();
    let database = database.clone();

    async move {
        let mut matches = Vec::new();

        for card in database.cards.values().iter().rev() {
            if !card.name.contains_key("en") && !card.name.contains_key("ja") {
                continue;
            }

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

        Search::new(matches)
    }
}

pub struct Search {
    matches: Arc<[Card]>,
}

impl Search {
    pub fn new(matches: impl Into<Arc<[Card]>>) -> Self {
        Self {
            matches: matches.into(),
        }
    }

    pub fn matches(&self) -> &[Card] {
        &self.matches
    }
}

impl Clone for Search {
    fn clone(&self) -> Self {
        Self {
            matches: self.matches.clone(),
        }
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        static CLEANER: LazyLock<mpsc::SyncSender<Arc<[Card]>>> = LazyLock::new(|| {
            let (sender, receiver) = mpsc::sync_channel(1);

            let _ = std::thread::spawn(move || {
                while let Ok(search) = receiver.recv() {
                    drop(search);
                }
            });

            sender
        });

        let _ = CLEANER.send(self.matches.clone());
    }
}

impl fmt::Debug for Search {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Search")
            .field("matches", &self.matches.len())
            .finish()
    }
}
