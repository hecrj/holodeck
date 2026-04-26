// Generated automatically by iced_fontello at build time.
// Do not edit manually. Source: ../fonts/holodeck-icons.toml
// 541400ead5ce4e69cc5b905704f5203ea60ad9a65aaf233d754fd69d90fd5661
use iced::widget::{Text, text};

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

pub fn camera<'a>() -> Text<'a> {
    icon("\u{1F4F7}")
}

pub fn cancel<'a>() -> Text<'a> {
    icon("\u{2715}")
}

fn icon(codepoint: &str) -> Text<'_> {
    text(codepoint).font("holodeck-icons")
}
