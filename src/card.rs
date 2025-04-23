pub use crate::pokebase::card::{Card, Id};

use crate::pokebase::set;
use crate::pokebase::{Database, Locale};

use iced::futures::TryFutureExt;

use bytes::Bytes;
use std::env;
use std::fmt;
use std::io;
use std::path::PathBuf;
use tokio::fs;
use tokio::task;

#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Bytes,
}

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("rgba", &self.rgba.len())
            .finish()
    }
}

impl Image {
    pub fn fetch<'a>(
        card: &Card,
        database: &Database,
    ) -> impl Future<Output = Result<Image, anywho::Error>> + 'a {
        let database = database.clone();
        let card = card.clone();

        async move {
            let cache = cache_dir().join(format!("{id}.png", id = card.id.as_str()));

            let client = reqwest::Client::new();

            let fetch_from_cache = async {
                let bytes = fs::read(&cache).await?;

                Ok(Bytes::from(bytes))
            };

            let fetch_from_pokemontcg = async {
                if !card.name.contains_key("en") {
                    return Err(Error::LocaleNotAvailable.into());
                }

                // PokemonTCG does not pad with leading 0s
                let set = {
                    let prefix: String = card
                        .set
                        .as_str()
                        .chars()
                        .take_while(|c| !c.is_digit(10))
                        .collect();

                    let number = &card.set.as_str()[prefix.len()..];

                    format!(
                        "{}{}",
                        prefix.replace(".", "pt"),
                        number.trim_start_matches('0').replace(".", "pt")
                    )
                };

                let number = card
                    .id
                    .as_str()
                    .rsplit("-")
                    .next()
                    .unwrap_or(card.id.as_str())
                    .trim_start_matches('0');

                let url = format!("https://images.pokemontcg.io/{set}/{number}_hires.png");

                log::info!("Downloading image: {url}");

                let request = client.get(url);

                let request = if let Ok(api_key) = env::var("POKEMONTCG_API_KEY") {
                    request.header("X-Api-Key", api_key)
                } else {
                    request
                };

                Ok::<_, anywho::Error>(request.send().await?.error_for_status()?.bytes().await?)
            };

            let fetch_from_tcgdex = async {
                let Some(set) = database.sets.get(&card.set) else {
                    return Err(Error::SetNotFound(card.set.clone()).into());
                };

                let locale = if card.name.contains_key("en") {
                    "en" // TODO
                } else if card.name.contains_key("ja") {
                    "ja"
                } else {
                    card.name.keys().next().map(Locale::as_str).unwrap_or("en")
                };

                let url = format!(
                    "https://assets.tcgdex.net/{locale}/{series}/{set}/{number}/high.png",
                    series = set.series.as_str(),
                    set = card.set.as_str(),
                    number = card
                        .id
                        .as_str()
                        .rsplit("-")
                        .next()
                        .unwrap_or(card.id.as_str())
                );

                log::info!("Downloading image: {url}");

                let request = client.get(url);

                Ok::<_, anywho::Error>(request.send().await?.error_for_status()?.bytes().await?)
            };

            // Rationale on the order of image fetching:
            // 1. Cache - fast and avoids rate limits in the long term.
            // 2. PokemonTCG - Highest quality, but English only. 20,000 requests/day with an API key.
            // 3. TCGDex - Lower quality, but supports multiple locales. Rate limiting unknown (?).
            let fetch = fetch_from_cache
                .or_else(|_: anywho::Error| fetch_from_pokemontcg)
                .or_else(|error| {
                    log::warn!("{error}");

                    fetch_from_tcgdex
                });

            let bytes = fetch.await?;

            if !fs::try_exists(&cache).await.unwrap_or_default() {
                let _ = fs::create_dir_all(cache.parent().unwrap_or(&cache)).await;
                let _ = fs::write(cache, &bytes).await;
            }

            // Decode image as RGBA in a background blocking thread
            task::spawn_blocking(move || {
                let image = image::ImageReader::new(io::Cursor::new(bytes))
                    .with_guessed_format()?
                    .decode()?
                    .to_rgba8();

                Ok(Image {
                    width: image.width(),
                    height: image.height(),
                    rgba: Bytes::from(image.into_raw()),
                })
            })
            .await?
        }
    }
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_default()
        .join("pokedeck")
        .join("cards")
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("set not found: {0:?}")]
    SetNotFound(set::Id),
    #[error("locale not available")]
    LocaleNotAvailable,
}
