pub mod pricing;

pub use crate::pokebase::card::{Card, Id, Search, search};
pub use pricing::Pricing;

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
    pub const WIDTH: u32 = 734;
    pub const HEIGHT: u32 = 1024;

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
                let mut image = image::ImageReader::new(io::Cursor::new(bytes))
                    .with_guessed_format()?
                    .decode()?
                    .to_rgba8();

                if !has_rounded_corners(&image) {
                    log::warn!("Card without rounded corners: {id}", id = card.id.as_str());
                    round_corners(&mut image);
                }

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

fn has_rounded_corners(rgba: &image::RgbaImage) -> bool {
    rgba.get_pixel(0, 0).0[3] == 0
}

// Courtesy of ChatGPT
fn round_corners(rgba: &mut image::RgbaImage) {
    let (width, height) = rgba.dimensions();

    let radius = (width as f32 * 0.05) as u32;
    let radius_sq = radius * radius;
    let aa_span = radius / 4;

    for y in 0..height {
        for x in 0..width {
            let dist_x = if x < radius {
                radius - x
            } else if x >= width - radius {
                x - (width - radius - 1)
            } else {
                0
            };

            let dist_y = if y < radius {
                radius - y
            } else if y >= height - radius {
                y - (height - radius - 1)
            } else {
                0
            };

            let dist_sq = dist_x * dist_x + dist_y * dist_y;

            if dist_sq > radius_sq {
                let dist = (dist_sq as f32).sqrt();

                if dist <= (radius + aa_span) as f32 {
                    let alpha_scale =
                        1.0 - (dist_sq - radius_sq) as f32 / (aa_span * aa_span) as f32;

                    let pixel = rgba.get_pixel_mut(x, y);
                    pixel.0[3] = (pixel.0[3] as f32 * alpha_scale) as u8;
                } else {
                    let pixel = rgba.get_pixel_mut(x, y);
                    pixel.0 = [0; 4];
                }
            }
        }
    }
}
