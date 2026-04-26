use crate::session;
use crate::{Card, Database, Error, Locale, Result};

use bytes::Bytes;
use serde::Deserialize;

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

    pub async fn download_image(&self, card: &Card, database: &Database) -> Result<Bytes> {
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

        Ok(session::retry(2, || async {
            self.client
                .get(&url)
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await
        })
        .await?)
    }

    pub async fn fetch_pricing(&self, card: &Card) -> Result<Pricing> {
        let locale = if card.name.has_english() {
            "en"
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
            "https://api.tcgdex.net/v2/{locale}/cards/{set}-{number}",
            set = card.set.as_str(),
            number = card
                .id
                .as_str()
                .rsplit("-")
                .next()
                .unwrap_or(card.id.as_str())
        );

        #[derive(Deserialize)]
        struct Response {
            pricing: Pricing,
        }

        let response: Response = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(response.pricing)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Pricing {
    #[serde(default)]
    pub tcgplayer: Option<tcgplayer::Pricing>,
    #[serde(default)]
    pub cardmarket: Option<cardmarket::Pricing>,
}

impl Default for Tcgdex {
    fn default() -> Self {
        Self::new()
    }
}
pub mod tcgplayer {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Pricing {
        #[serde(default)]
        pub normal: Option<Spread>,
        #[serde(default)]
        pub holofoil: Option<Spread>,
        #[serde(default)]
        pub reverse_holofoil: Option<Spread>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Spread {
        #[serde(rename = "lowPrice")]
        pub low: f64,
        #[serde(rename = "midPrice")]
        pub mid: f64,
        #[serde(rename = "highPrice")]
        pub high: f64,
        #[serde(rename = "marketPrice")]
        pub market: f64,
    }
}

pub mod cardmarket {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Pricing {
        #[serde(default)]
        pub low: f64,
        #[serde(default)]
        pub trend: f64,
        #[serde(default)]
        pub avg1: f64,
        #[serde(default)]
        pub avg7: f64,
        #[serde(default)]
        pub avg30: f64,

        #[serde(default)]
        pub low_holo: f64,
        #[serde(default)]
        pub trend_holo: f64,
        #[serde(default)]
        pub avg1_holo: f64,
        #[serde(default)]
        pub avg7_holo: f64,
        #[serde(default)]
        pub avg30_holo: f64,
    }
}
