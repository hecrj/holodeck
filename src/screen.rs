pub mod binder;
pub mod welcome;

pub use binder::Binder;
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
    Binder(Binder),
}
