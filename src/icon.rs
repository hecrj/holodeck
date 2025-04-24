// Generated automatically by iced_fontello at build time.
// Do not edit manually. Source: ../fonts/holofoil-icons.toml
// f491e302ff8defe16c5f2120ee069ae2355d481ce9a376926a99704277d8a1e6
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

pub fn pointer<'a>() -> Text<'a> {
    icon("\u{F25A}")
}

fn icon(codepoint: &str) -> Text<'_> {
    text(codepoint).font(Font::with_name("holofoil-icons"))
}
