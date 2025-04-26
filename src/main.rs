use pokebase;

mod binder;
mod card;
mod collection;
mod icon;
mod screen;
mod widget;

use crate::binder::Binder;
use crate::collection::Collection;
use crate::pokebase::{Database, Result, Session};
use crate::screen::Screen;
use crate::screen::binders;
use crate::screen::welcome;
use crate::widget::logo;

use iced::widget::{button, center, column, container, row, text};
use iced::{Center, Element, Fill, Font, Subscription, Task, Theme};
use std::env;

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application(Holofoil::new, Holofoil::update, Holofoil::view)
        .subscription(Holofoil::subscription)
        .theme(Holofoil::theme)
        .font(icon::FONT)
        .default_font(Font::MONOSPACE)
        .window_size((1700.0, 950.0))
        .run()
}

struct Holofoil {
    state: State,
}

enum State {
    Loading,
    Ready {
        database: Database,
        session: Session,
        screen: Screen,
    },
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<Database, anywho::Error>),
    Welcome(welcome::Message),
    Binders(binders::Message),
    OpenBinders,
    Browse,
}

impl Holofoil {
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
                let (welcome, task) = screen::Welcome::new();

                self.state = State::Ready {
                    database,
                    session: Session::new(env::var("POKEMONTCG_API_KEy").ok()), // TODO: Configuration
                    screen: Screen::Welcome(welcome),
                };

                task.map(Message::Welcome)
            }
            Message::Welcome(message) => {
                let State::Ready { screen, .. } = &mut self.state else {
                    return Task::none();
                };

                let Screen::Welcome(welcome) = screen else {
                    return Task::none();
                };

                match welcome.update(message) {
                    welcome::Action::None => Task::none(),
                    welcome::Action::Run(task) => task.map(Message::Welcome),
                    welcome::Action::Select(collection) => {
                        let binders = screen::Binders::new();

                        *screen = Screen::Collecting {
                            collection,
                            screen: screen::Collecting::Binders(binders),
                        };

                        Task::none()
                    }
                }
            }
            Message::Binders(message) => {
                let State::Ready {
                    database,
                    session,
                    screen:
                        Screen::Collecting {
                            collection,
                            screen: screen::Collecting::Binders(binders),
                        },
                } = &mut self.state
                else {
                    return Task::none();
                };

                binders
                    .update(message, collection, database, session)
                    .map(Message::Binders)
            }
            Message::OpenBinders => {
                let State::Ready {
                    screen: Screen::Collecting { screen, .. },
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                let binders = screen::Binders::new();
                *screen = screen::Collecting::Binders(binders);

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
            State::Loading => center(text("Loading...")).into(),
            State::Ready {
                database, screen, ..
            } => match screen {
                Screen::Welcome(welcome) => welcome.view(database).map(Message::Welcome),
                Screen::Collecting { collection, screen } => {
                    let tabs = [
                        (
                            "Binders",
                            icon::binder(),
                            Message::OpenBinders,
                            matches!(screen, screen::Collecting::Binders(_),),
                        ),
                        ("Browse", icon::browse(), Message::Browse, false), // TODO
                    ]
                    .into_iter()
                    .map(|(label, icon, on_click, is_active)| {
                        button(
                            row![icon.size(14), text(label).size(14)]
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
                        screen::Collecting::Binders(binders) => {
                            binders.view(collection, database).map(Message::Binders)
                        }
                    };

                    column![container(screen).height(Fill), navbar].into()
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
                screen::Collecting::Binders(binders) => {
                    binders.subscription().map(Message::Binders)
                }
            },
        }
    }

    fn theme(&self) -> Theme {
        Theme::CatppuccinMocha
    }
}
