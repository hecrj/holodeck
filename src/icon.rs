// Generated automatically by iced_fontello at build time.
// Do not edit manually. Source: ../fonts/holodeck-icons.toml
// df010bd4723d936cdfd888fb0750d31c4bc22724f64e5e977e9da6b1180f0e3d
use iced::widget::{text, Text};
use iced::Font;

pub const FONT: &[u8] = include_bytes!("../fonts/holodeck-icons.ttf");

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
    text(codepoint).font(Font::with_name("holodeck-icons"))
}
