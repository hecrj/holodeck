pub use crate::session::pokemon_tcg::cardmarket;
pub use crate::session::pokemon_tcg::tcgplayer;

use crate::{Card, Result, Session};

#[derive(Debug, Clone)]
pub struct Pricing {
    pub tcgplayer: tcgplayer::Pricing,
    pub cardmarket: cardmarket::Pricing,
}

impl Pricing {
    pub async fn fetch(card: &Card, session: &Session) -> Result<Self> {
        let pricing = session.pokemon_tcg.fetch_pricing(card).await?;

        Ok(Self {
            tcgplayer: pricing.tcgplayer,
            cardmarket: pricing.cardmarket,
        })
    }
}
