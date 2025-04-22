use pokebase;

mod collection;
mod icon;
mod screen;
mod widget;

use crate::collection::Collection;
use crate::pokebase::Database;
use crate::screen::Screen;
use crate::screen::binder;
use crate::screen::welcome;
use crate::widget::logo;

use iced::widget::{button, center, column, container, row, text};
use iced::{Center, Element, Fill, Font, Subscription, Task, Theme};

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application(Pokedeck::new, Pokedeck::update, Pokedeck::view)
        .subscription(Pokedeck::subscription)
        .theme(Pokedeck::theme)
        .font(icon::FONT)
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
    Binder(binder::Message),
    OpenBinder,
    Browse,
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
            Message::Welcome(message) => {
                let State::Ready { database, screen } = &mut self.state else {
                    return Task::none();
                };

                let Screen::Welcome(welcome) = screen else {
                    return Task::none();
                };

                match welcome.update(message, database) {
                    welcome::Action::None => Task::none(),
                    welcome::Action::Run(task) => task.map(Message::Welcome),
                    welcome::Action::Select(collection) => {
                        let binder = screen::Binder::new();

                        *screen = Screen::Collecting {
                            collection,
                            screen: screen::Collecting::Binder(binder),
                        };

                        Task::none()
                    }
                }
            }
            Message::Binder(message) => {
                let State::Ready {
                    screen:
                        Screen::Collecting {
                            collection,
                            screen: screen::Collecting::Binder(binder),
                        },
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                binder.update(message, collection).map(Message::Binder)
            }
            Message::OpenBinder => {
                let State::Ready {
                    screen: Screen::Collecting { screen, .. },
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                let binder = screen::Binder::new();
                *screen = screen::Collecting::Binder(binder);

                Task::none()
            }
            Message::Browse => {
                // TODO
                Task::none()
            }
            Message::Loaded(Err(error)) => {
                log::error!("{error}");

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        match &self.state {
            State::Loading => center(text("Loading...").font(Font::MONOSPACE)).into(),
            State::Ready { database, screen } => match screen {
                Screen::Welcome(welcome) => welcome.view(database).map(Message::Welcome),
                Screen::Collecting { collection, screen } => {
                    let tabs = [
                        (
                            "Binder",
                            icon::binder(),
                            Message::OpenBinder,
                            matches!(screen, screen::Collecting::Binder(_),),
                        ),
                        ("Browse", icon::browse(), Message::Browse, false), // TODO
                    ]
                    .into_iter()
                    .map(|(label, icon, on_click, is_active)| {
                        button(
                            row![icon.size(14), text(label).size(14).font(Font::MONOSPACE)]
                                .spacing(10)
                                .align_y(Center),
                        )
                        .style(move |theme, status| {
                            if is_active {
                                let palette = theme.extended_palette();

                                button::Style {
                                    background: Some(palette.background.base.color.into()),
                                    text_color: palette.background.base.text,
                                    ..button::text(theme, status)
                                }
                            } else {
                                button::text(theme, status)
                            }
                        })
                        .padding([8, 15])
                        .on_press(on_click)
                        .into()
                    });

                    let navbar = container(
                        row![logo(14), row(tabs)]
                            .spacing(10)
                            .width(Fill)
                            .align_y(Center),
                    )
                    .padding([0, 10])
                    .style(container::dark);

                    let screen = match screen {
                        screen::Collecting::Binder(binder) => {
                            binder.view(collection, database).map(Message::Binder)
                        }
                    };

                    column![container(screen).height(Fill).padding(10), navbar].into()
                }
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let State::Ready { screen, .. } = &self.state else {
            return Subscription::none();
        };

        match screen {
            Screen::Welcome(_) => Subscription::none(),
            Screen::Collecting { screen, .. } => match screen {
                screen::Collecting::Binder(binder) => binder.subscription().map(Message::Binder),
            },
        }
    }

    fn theme(&self) -> Theme {
        Theme::CatppuccinMocha
    }
}
