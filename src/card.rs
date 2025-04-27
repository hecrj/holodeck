pub mod pricing;

pub use crate::pokebase::card::{Card, Id, search};

use crate::pokebase::card;
use crate::pokebase::{Database, Session};

use bytes::Bytes;
use futures_util::TryFutureExt;
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

impl Image {
    pub fn fetch<'a>(
        card: &Card,
        database: &Database,
        session: &Session,
    ) -> impl Future<Output = Result<Image, anywho::Error>> + 'a {
        let card = card.clone();
        let database = database.clone();
        let session = session.clone();

        async move {
            let cache = cache_dir().join(format!("{id}.png", id = card.id.as_str()));

            let fetch_from_cache = async {
                let bytes = fs::read(&cache).await?;

                Ok(Bytes::from(bytes))
            };

            let download_image = async {
                let image = card::Image::download(&card, &database, &session).await?;

                Ok::<_, anywho::Error>(image.bytes)
            };

            let bytes = fetch_from_cache
                .or_else(|_: anywho::Error| download_image)
                .await?;

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

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("rgba", &self.rgba.len())
            .finish()
    }
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_default()
        .join(env!("CARGO_PKG_NAME"))
        .join("cards")
}
