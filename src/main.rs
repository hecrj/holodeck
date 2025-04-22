use pokebase;

mod collection;
mod screen;

use crate::pokebase::Database;
use crate::screen::Screen;
use crate::screen::welcome;

use iced::widget::{center, text};
use iced::{Element, Font, Task, Theme};

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application(Pokedeck::new, Pokedeck::update, Pokedeck::view)
        .theme(Pokedeck::theme)
        .run()
}

struct Pokedeck {
    state: State,
}

enum State {
    Loading,
    Ready { database: Database, screen: Screen },
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<Database, anywho::Error>),
    Welcome(welcome::Message),
}

impl Pokedeck {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                state: State::Loading,
            },
            Task::perform(Database::load(), Message::Loaded),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(Ok(database)) => {
                let (welcome, task) = screen::Welcome::new(&database);

                self.state = State::Ready {
                    database,
                    screen: Screen::Welcome(welcome),
                };

                task.map(Message::Welcome)
            }
            Message::Loaded(Err(error)) => {
                log::error!("{error}");

                Task::none()
            }
            Message::Welcome(message) => {
                let State::Ready {
                    database,
                    screen: Screen::Welcome(welcome),
                } = &mut self.state
                else {
                    return Task::none();
                };

                welcome.update(message, database).map(Message::Welcome)
            }
        }
    }

    fn view(&self) -> Element<Message> {
        match &self.state {
            State::Loading => center(text("Loading...").font(Font::MONOSPACE)).into(),
            State::Ready { database, screen } => match screen {
                Screen::Welcome(welcome) => welcome.view(database).map(Message::Welcome),
            },
        }
    }

    fn theme(&self) -> Theme {
        Theme::CatppuccinMocha
    }
}
