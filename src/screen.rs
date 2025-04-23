pub mod binders;
pub mod welcome;

pub use binders::Binders;
pub use welcome::Welcome;

use crate::Collection;

pub enum Screen {
    Welcome(Welcome),
    Collecting {
        collection: Collection,
        screen: Collecting,
    },
}

pub enum Collecting {
    Binders(Binders),
}
