pub mod welcome;

pub use welcome::Welcome;

pub enum Screen {
    Welcome(Welcome),
}
