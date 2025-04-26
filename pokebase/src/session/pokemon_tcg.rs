use crate::session;
use crate::{Card, Error};

use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct PokemonTcg {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl PokemonTcg {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: session::CLIENT.clone(),
            api_key,
        }
    }

    pub async fn download_image<'a>(&self, card: &Card) -> Result<Bytes, Error> {
        if !card.name.contains_key("en") {
            return Err(Error::LocaleNotAvailable);
        }

        let set = set_name(card);
        let number = card_number(card);
        let url = format!("https://images.pokemontcg.io/{set}/{number}_hires.png");

        log::info!("Downloading image: {url}");
        let response = session::retry(2, || self.get(&url).send()).await;

        Ok(response?.error_for_status()?.bytes().await?)
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

fn set_name(card: &Card) -> String {
    // PokemonTCG does not pad with leading 0s
    let prefix: String = card
        .set
        .as_str()
        .chars()
        .take_while(|c| !c.is_digit(10))
        .collect();

    let number = &card.set.as_str()[prefix.len()..];

    let replacement =
        if prefix.starts_with("swsh") && number.starts_with("12") || prefix.starts_with("sv") {
            "pt"
        } else {
            ""
        };

    format!(
        "{}{}",
        prefix.replace(".", replacement),
        number.trim_start_matches('0').replace(".", replacement)
    )
}

fn card_number(card: &Card) -> &str {
    card.id
        .as_str()
        .rsplit("-")
        .next()
        .unwrap_or(card.id.as_str())
        .trim_start_matches('0')
}
