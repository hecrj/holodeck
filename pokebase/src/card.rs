pub use crate::core::card::*;

use crate::{Database, Error, Search, Session};

use bytes::Bytes;
use std::fmt;
use std::sync::Arc;

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
    pub async fn download<'a>(
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

pub fn search<'a>(query: &str, database: &Database) -> impl Future<Output = Search<Card>> + 'a {
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

        Search {
            matches: Arc::from(matches),
        }
    }
}
