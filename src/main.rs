use iced::widget::{center, text};
use iced::{Element, Font, Task, Theme};

pub fn main() -> iced::Result {
    iced::application(Pokedeck::new, Pokedeck::update, Pokedeck::view)
        .theme(Pokedeck::theme)
        .run()
}

struct Pokedeck {}

#[derive(Debug, Clone)]
enum Message {}

impl Pokedeck {
    fn new() -> (Self, Task<Message>) {
        (Self {}, Task::none())
    }

    fn update(&mut self, message: Message) {
        match message {}
    }

    fn view(&self) -> Element<Message> {
        center(text("Pokédeck").font(Font::MONOSPACE).size(30)).into()
    }

    fn theme(&self) -> Theme {
        Theme::CatppuccinMocha
    }
}
