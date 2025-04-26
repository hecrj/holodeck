use crate::binder;
use crate::collection::{self, Collection};
use crate::icon;
use crate::pokebase::Database;
use crate::widget::logo;

use iced::widget::{
    bottom_center, button, center, column, container, horizontal_space, row, stack, text,
    text_input,
};
use iced::{Center, Element, Fill, Task};

pub struct Welcome {
    state: State,
}

#[derive(Debug, Clone)]
pub enum Message {
    CollectionsListed(Result<Vec<Collection>, anywho::Error>),
    Select(Collection),
    New,
    NameChanged(String),
    Create(collection::Name),
    CollectionCreated(Result<Collection, anywho::Error>),
}

pub enum State {
    Loading,
    Selection {
        collections: Vec<Collection>,
    },
    Creation {
        name: String,
        collections: Vec<Collection>,
    },
}

pub enum Action {
    None,
    Run(Task<Message>),
    Select(Collection),
}

impl Welcome {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                state: State::Loading,
            },
            Task::perform(Collection::list(), Message::CollectionsListed),
        )
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::CollectionsListed(Ok(collections)) => {
                if collections.is_empty() {
                    self.state = State::Creation {
                        name: "Red".to_owned(),
                        collections,
                    };
                } else {
                    self.state = State::Selection { collections };
                }

                Action::None
            }
            Message::Select(collection) => Action::Select(collection),
            Message::Create(name) => Action::Run(Task::perform(
                Collection::create(name),
                Message::CollectionCreated,
            )),
            Message::New => {
                let State::Selection { collections } = &self.state else {
                    return Action::None;
                };

                self.state = State::Creation {
                    name: String::new(),
                    collections: collections.clone(),
                };

                Action::None
            }
            Message::NameChanged(new_name) => {
                if let State::Creation { name, .. } = &mut self.state {
                    *name = new_name;
                }

                Action::None
            }
            Message::CollectionCreated(Ok(_collection)) => {
                self.state = State::Loading;

                Action::Run(Task::perform(
                    Collection::list(),
                    Message::CollectionsListed,
                ))
            }
            Message::CollectionsListed(Err(error)) | Message::CollectionCreated(Err(error)) => {
                log::error!("{error}");

                Action::None
            }
        }
    }

    pub fn view(&self, database: &Database) -> Element<Message> {
        let content: Element<_> = match &self.state {
            State::Loading => text("Loading...").into(),
            State::Selection { collections } => column![
                column(
                    collections
                        .iter()
                        .map(|collection| card(collection, database)),
                )
                .spacing(10),
                button(
                    row![icon::add().size(14), text("New Profile").size(14)]
                        .spacing(10)
                        .align_y(Center)
                )
                .on_press(Message::New),
            ]
            .spacing(20)
            .align_x(Center)
            .into(),
            State::Creation { name, collections } => {
                let welcome = container(text(
                    "Hello there! Welcome to the world of Pokémon!\n\
                        First, what is your name?",
                ))
                .width(Fill)
                .padding(10)
                .style(container::bordered_box);

                let name_input = text_input("Name", name)
                    .on_input(Message::NameChanged)
                    .padding(10);

                let name = collection::Name::parse(name);

                let submit = button(if collections.is_empty() {
                    "Start"
                } else {
                    "Create"
                })
                .padding([10, 20])
                .on_press_maybe(name.map(Message::Create));

                column![
                    welcome,
                    column![text("Your name"), row![name_input, submit].spacing(10)].spacing(10)
                ]
                .spacing(30)
                .into()
            }
        };

        stack![
            center(
                column![logo(40), content]
                    .spacing(20)
                    .align_x(Center)
                    .max_width(480),
            )
            .padding(20),
            legal_disclaimer()
        ]
        .into()
    }
}

fn card<'a>(collection: &'a Collection, database: &Database) -> Element<'a, Message> {
    let name = text(collection.name.as_str()).size(20);

    let total_cards = collection.total_cards();
    let unique_cards = collection.unique_cards();
    let total_pokemon = collection.total_pokemon(database);

    let stat = |stat| text(stat).size(14);

    let progress = binder::Mode::GottaCatchEmAll.progress(collection, database);

    let badge = stat(
        match progress as u32 {
            0..10 => "Beginner",
            10..20 => "Intermediate",
            20..40 => "Advanced",
            40..60 => "Pokéxpert",
            70..80 => "Master",
            80..100 => "Elite Four",
            100 => "Champion",
            _ => "Unknown",
        }
        .to_owned(),
    );

    let stats = row![
        stat(format!("{total_pokemon} Pokémon")),
        stat(format!("{unique_cards} unique")),
        stat(format!(
            "{total_cards} card{}",
            if total_cards == 1 { "" } else { "s" }
        )),
    ]
    .spacing(20);

    button(
        container(
            column![
                row![name, horizontal_space(), stats]
                    .spacing(20)
                    .align_y(Center),
                row![
                    badge,
                    horizontal_space(),
                    stat(format!("{progress:.1}% completed"))
                ]
                .spacing(20)
                .align_y(Center),
            ]
            .spacing(10),
        )
        .padding(20)
        .style(container::bordered_box)
        .width(Fill),
    )
    .on_press_with(|| Message::Select(collection.clone()))
    .padding(0)
    .style(button::text)
    .into()
}

fn legal_disclaimer<'a>() -> Element<'a, Message> {
    bottom_center(
        text(
            "This application is not affiliated with, endorsed, sponsored, or \
            approved by Nintendo, Game Freak, The Pokémon Company, or any other \
            official TCG publisher.\n\
            All trademarks and copyrighted materials are the property of their \
            respective owners.",
        )
        .center()
        .width(Fill)
        .size(8),
    )
    .padding(10)
    .into()
}
