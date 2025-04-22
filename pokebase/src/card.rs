use crate::locale;
use crate::pokemon;
use crate::set;

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: Id,
    pub set: set::Id,
    pub name: locale::Map,
    pub types: BTreeSet<Type>,
    pub rarity: Rarity,
    pub variants: Variants,
    pub illustrator: Option<String>,
    pub pokedex: Vec<pokemon::Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Id(pub(crate) String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub enum Rarity {
    #[default]
    None,
    Common,
    Uncommon,
    Rare,
    HoloRare,
    HoloRareLvx,
    HoloRareV,
    HoloRareVmax,
    HoloRareVstar,
    ShinyRare,
    ShinyRareV,
    ShinyRareVmax,
    DoubleRare,
    AceSpecRare,
    AmazingRare,
    RadiantRare,
    RarePrime,
    Legend,
    ClassicCollection,
    UltraRare,
    ShinyUltraRare,
    SecretRare,
    FullArtTrainer,
    IllustrationRare,
    SpecialIllustrationRare,
    HyperRare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Type {
    Grass,
    Fire,
    Water,
    Lightning,
    Psychic,
    Fighting,
    Darkness,
    Metal,
    Fairy,
    Dragon,
    Colorless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variants {
    pub first_edition: bool,
    pub holo: bool,
    pub normal: bool,
    pub reverse: bool,
    pub w_promo: bool,
}
