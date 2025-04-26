use pokebase_core as core;

pub use crate::core::locale;
pub use crate::core::pokemon;
pub use crate::core::series;
pub use crate::core::set;
pub use crate::core::{Database, Map, Search};

pub use locale::Locale;
pub use pokemon::Pokemon;
pub use series::Series;
pub use set::Set;

pub mod card;
pub mod session;

mod error;

pub use card::Card;
pub use error::Error;
pub use session::Session;

pub type Result<T, E = Error> = std::result::Result<T, E>;
