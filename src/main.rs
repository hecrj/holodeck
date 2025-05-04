use pokebase;

mod binder;
mod card;
mod collection;
mod icon;
mod screen;
mod widget;

use crate::binder::Binder;
use crate::card::pricing::{self, Pricing};
use crate::collection::Collection;
use crate::pokebase::{Database, Result, Session};
use crate::screen::Screen;
use crate::screen::binders;
use crate::screen::welcome;
use crate::widget::logo;

use iced::time::Instant;
use iced::widget::{button, center, column, container, row, text};
use iced::{Center, Element, Fill, Font, Subscription, Task, Theme};

use std::env;

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application::timed(
        Holodeck::new,
        Holodeck::update,
        Holodeck::subscription,
        Holodeck::view,
    )
    .theme(Holodeck::theme)
    .font(icon::FONT)
    .default_font(Font::MONOSPACE)
    .window_size((1700.0, 950.0))
    .run()
}

struct Holodeck {
    state: State,
    now: Instant,
}

enum State {
    Loading,
    Ready {
        database: Database,
        session: Session,
        screen: Screen,
        prices: pricing::Map,
    },
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<(Database, pricing::Map), anywho::Error>),
    Welcome(welcome::Message),
    Binders(binders::Message),
    OpenBinders,
    Browse,
    PricingUpdated((card::Id, Pricing)),
}

impl Holodeck {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                state: State::Loading,
                now: Instant::now(),
            },
            Task::perform(
                async {
                    let database = Database::load().await?;
                    let prices = Pricing::list().await?;

                    Ok((database, prices))
                },
                Message::Loaded,
            ),
        )
    }

    fn update(&mut self, message: Message, now: Instant) -> Task<Message> {
        self.now = now;

        match message {
            Message::Loaded(Ok((database, prices))) => {
                let (welcome, task) = screen::Welcome::new();

                let session = Session::new(env::var("POKEMONTCG_API_KEY").ok()); // TODO: Configuration

                let price_updates = Task::run(
                    Pricing::subscribe(&database, &session),
                    Message::PricingUpdated,
                );

                self.state = State::Ready {
                    database,
                    session,
                    screen: Screen::Welcome(welcome),
                    prices,
                };

                Task::batch([task.map(Message::Welcome), price_updates])
            }
            Message::Welcome(message) => {
                let State::Ready {
                    screen,
                    database,
                    prices,
                    session,
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                let Screen::Welcome(welcome) = screen else {
                    return Task::none();
                };

                match welcome.update(message, database, prices, session, self.now) {
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
                    prices,
                    session,
                    screen:
                        Screen::Collecting {
                            collection,
                            screen: screen::Collecting::Binders(binders),
                        },
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                binders
                    .update(message, collection, database, prices, session, self.now)
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
            Message::PricingUpdated((card, pricing)) => {
                let State::Ready { prices, .. } = &mut self.state else {
                    return Task::none();
                };

                let _ = prices.insert(card, pricing);

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
                database,
                screen,
                prices,
                ..
            } => match screen {
                Screen::Welcome(welcome) => welcome
                    .view(database, prices, self.now)
                    .map(Message::Welcome),
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
                        screen::Collecting::Binders(binders) => binders
                            .view(collection, database, prices, self.now)
                            .map(Message::Binders),
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
            Screen::Welcome(welcome) => welcome.subscription(self.now).map(Message::Welcome),
            Screen::Collecting { screen, .. } => match screen {
                screen::Collecting::Binders(binders) => {
                    binders.subscription(self.now).map(Message::Binders)
                }
            },
        }
    }

    fn theme(&self) -> Theme {
        Theme::CatppuccinMocha
    }
}
