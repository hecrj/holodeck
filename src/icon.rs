// Generated automatically by iced_fontello at build time.
// Do not edit manually. Source: ../fonts/holofoil-icons.toml
// 6abf5666186eb78fa470e232f14159b1019c690230467660bf9cbd2897cabee8
use iced::widget::{text, Text};
use iced::Font;

pub const FONT: &[u8] = include_bytes!("../fonts/holofoil-icons.ttf");

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
    text(codepoint).font(Font::with_name("holofoil-icons"))
}
