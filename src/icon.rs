// Generated automatically by iced_fontello at build time.
// Do not edit manually. Source: ../fonts/pokedeck-icons.toml
// ab5c74f9a440949c310f422142625f565eb1379992ad189733a4c3e8bcf5e446
use iced::widget::{text, Text};
use iced::Font;

pub const FONT: &[u8] = include_bytes!("../fonts/pokedeck-icons.ttf");

pub fn add<'a>() -> Text<'a> {
    icon("\u{2B}")
}

pub fn binder<'a>() -> Text<'a> {
    icon("\u{1F4D6}")
}

pub fn book<'a>() -> Text<'a> {
    icon("\u{1F4D5}")
}

pub fn browse<'a>() -> Text<'a> {
    icon("\u{1F50D}")
}

pub fn cancel<'a>() -> Text<'a> {
    icon("\u{2715}")
}

fn icon(codepoint: &str) -> Text<'_> {
    text(codepoint).font(Font::with_name("pokedeck-icons"))
}
