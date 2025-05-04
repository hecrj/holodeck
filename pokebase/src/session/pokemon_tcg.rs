use crate::session;
use crate::{Card, Error};

use bytes::Bytes;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct PokemonTcg {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl PokemonTcg {
    pub fn new(api_key: Option<String>) -> Self {
        if let Some(api_key) = api_key.as_ref() {
            log::info!(
                "PokemonTCG session started (API key: {})",
                api_key.replace(|c: char| c.is_alphanumeric(), "*")
            )
        } else {
            log::warn!("PokemonTCG session started without an API key!")
        }

        Self {
            client: session::CLIENT.clone(),
            api_key,
        }
    }

    pub async fn download_image(&self, card: &Card) -> Result<Bytes, Error> {
        if !card.name.has_english() {
            return Err(Error::LocaleNotAvailable);
        }

        let set = set_name(card);
        let number = card_number(card);
        let url = format!("https://images.pokemontcg.io/{set}/{number}_hires.png");

        log::info!("Downloading image: {url}");
        let response = session::retry(2, || self.get(&url).send()).await?;

        Ok(response.error_for_status()?.bytes().await?)
    }

    pub async fn fetch_pricing(&self, card: &Card) -> Result<Pricing, Error> {
        if !card.name.has_english() {
            return Err(Error::LocaleNotAvailable);
        }

        let number = card_number(card);
        let set = set_name(card);

        let url = format!("https://api.pokemontcg.io/v2/cards/{set}-{number}");

        #[derive(Deserialize)]
        struct Response {
            data: Pricing,
        }

        log::info!("Fetching price: {url}");

        let response =
            session::retry(2, async || self.get(&url).send().await?.error_for_status()).await?;
        let response: Response = response.json().await?;

        Ok(response.data)
    }

    fn get(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        let request = self.client.get(url.as_ref());

        if let Some(api_key) = &self.api_key {
            request.header("X-Api-Key", api_key)
        } else {
            request
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Pricing {
    pub tcgplayer: tcgplayer::Pricing,
    pub cardmarket: cardmarket::Pricing,
}

fn set_name(card: &Card) -> String {
    // PokemonTCG does not pad with leading 0s
    let prefix: String = card
        .set
        .as_str()
        .chars()
        .take_while(|c| !c.is_ascii_digit())
        .collect();

    let number = &card.set.as_str()[prefix.len()..];

    let replacement =
        if prefix.starts_with("swsh") && number.starts_with("12") || prefix.starts_with("sv") {
            "pt"
        } else {
            ""
        };

    let mut name = format!(
        "{}{}",
        prefix.replace(".", replacement),
        number.trim_start_matches('0').replace(".", replacement)
    );

    if card_number(card).starts_with("TG") {
        name.push_str("tg");
    }

    name
}

fn card_number(card: &Card) -> &str {
    card.id
        .as_str()
        .rsplit("-")
        .next()
        .unwrap_or(card.id.as_str())
        .trim_start_matches('0')
}

pub mod tcgplayer {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
    pub struct Pricing {
        pub prices: Prices,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Prices {
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
        pub low: f64,
        pub mid: f64,
        pub high: f64,
        pub market: f64,
    }
}

pub mod cardmarket {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
    pub struct Pricing {
        pub prices: Prices,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Prices {
        #[serde(default)]
        pub average_sell_price: f64,
        #[serde(default)]
        pub low_price: f64,
        #[serde(default)]
        pub trend_price: f64,
        #[serde(default)]
        pub avg1: f64,
        #[serde(default)]
        pub avg7: f64,
        #[serde(default)]
        pub avg30: f64,
        #[serde(default)]
        pub reverse_holo_sell: f64,
        #[serde(default)]
        pub reverse_holo_low: f64,
        #[serde(default)]
        pub reverse_holo_trend: f64,
        #[serde(default)]
        pub reverse_holo_avg1: f64,
        #[serde(default)]
        pub reverse_holo_avg7: f64,
        #[serde(default)]
        pub reverse_holo_avg30: f64,
    }
}
