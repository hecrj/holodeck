use crate::session;
use crate::{Card, Database, Error, Locale};

use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Tcgdex {
    client: reqwest::Client,
}

impl Tcgdex {
    pub fn new() -> Self {
        Self {
            client: session::CLIENT.clone(),
        }
    }

    pub async fn download_image(&self, card: &Card, database: &Database) -> Result<Bytes, Error> {
        let Some(set) = database.sets.get(&card.set) else {
            return Err(Error::SetNotFound(card.set.clone()));
        };

        let locale = if card.name.has_english() {
            "en" // TODO
        } else if card.name.has_japanese() {
            "ja"
        } else {
            card.name
                .locales()
                .next()
                .map(Locale::as_str)
                .unwrap_or("en")
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
        let response = session::retry(2, || self.client.get(&url).send()).await;

        Ok(response?.error_for_status()?.bytes().await?)
    }
}

impl Default for Tcgdex {
    fn default() -> Self {
        Self::new()
    }
}
