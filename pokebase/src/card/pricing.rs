pub use crate::session::pokemon_tcg::cardmarket;
pub use crate::session::pokemon_tcg::tcgplayer;

use crate::{Card, Result, Session};

use std::time::SystemTime;

#[derive(Debug, Clone, Copy)]
pub struct Pricing {
    pub tcgplayer: tcgplayer::Pricing,
    pub cardmarket: cardmarket::Pricing,
    pub updated_at: SystemTime,
}

impl Pricing {
    pub async fn fetch(card: &Card, session: &Session) -> Result<Self> {
        let pricing = session.pokemon_tcg.fetch_pricing(card).await?;

        Ok(Self {
            tcgplayer: pricing.tcgplayer,
            cardmarket: pricing.cardmarket,
            updated_at: SystemTime::now(),
        })
    }
}

impl Default for Pricing {
    fn default() -> Self {
        Self {
            tcgplayer: tcgplayer::Pricing::default(),
            cardmarket: cardmarket::Pricing::default(),
            updated_at: SystemTime::now(),
        }
    }
}
