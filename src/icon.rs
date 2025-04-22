// Generated automatically by iced_fontello at build time.
// Do not edit manually. Source: ../fonts/pokedeck-icons.toml
// dce49ae0854975679d4010321d53ce15aa1641e4825a7e982c8ece591504f1f3
use iced::widget::{text, Text};
use iced::Font;

pub const FONT: &[u8] = include_bytes!("../fonts/pokedeck-icons.ttf");

pub fn binder<'a>() -> Text<'a> {
    icon("\u{1F4D6}")
}

pub fn book<'a>() -> Text<'a> {
    icon("\u{1F4D5}")
}

pub fn browse<'a>() -> Text<'a> {
    icon("\u{1F50D}")
}

fn icon(codepoint: &str) -> Text<'_> {
    text(codepoint).font(Font::with_name("pokedeck-icons"))
}
