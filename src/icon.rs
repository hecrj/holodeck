// Generated automatically by iced_fontello at build time.
// Do not edit manually. Source: ../fonts/keroberos-icons.toml
// 7f4a7fa5e7e504ed3824adc13da5d30a500912f56d8e870c70b8025ce2caa0ea
use iced::Font;
use iced::widget::{Text, text};

pub const FONT: &[u8] = include_bytes!("../fonts/keroberos-icons.ttf");

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

pub fn camera<'a>() -> Text<'a> {
    icon("\u{1F4F7}")
}

pub fn cancel<'a>() -> Text<'a> {
    icon("\u{2715}")
}

fn icon(codepoint: &str) -> Text<'_> {
    text(codepoint).font(Font::with_name("keroberos-icons"))
}
