use crate::binder;
use crate::card;
use crate::card::pricing;
use crate::collection::{self, Collection};
use crate::icon;
use crate::pokebase::{Database, Session};
use crate::widget::logo;

use function::Binary;
use iced::animation;
use iced::border;
use iced::gradient;
use iced::time::{Instant, seconds};
use iced::widget::{
    bottom_center, button, center, column, container, image, mouse_area, row, stack, text,
    text_input, vertical_space,
};
use iced::window;
use iced::{Animation, Center, Color, ContentFit, Degrees, Element, Fill, Subscription, Task};

use std::collections::HashMap;

pub struct Welcome {
    state: State,
}

#[derive(Debug, Clone)]
pub enum Message {
    Listed(Result<Vec<Collection>, anywho::Error>),
    ImagesLoaded(collection::Name, Vec<Result<card::Image, anywho::Error>>),
    Hovered(collection::Name, bool),
    Select(Collection),
    New,
    NameChanged(String),
    Create(collection::Name),
    Created(Result<Collection, anywho::Error>),
    Tick(Instant),
}

pub enum State {
    Loading,
    Selection {
        collections: Vec<Collection>,
        animations: HashMap<collection::Name, AnimationSet>,
        now: Instant,
    },
    Creation {
        name: String,
        collections: Vec<Collection>,
    },
}

pub struct AnimationSet {
    images: Vec<image::Handle>,
    fade_in: Animation<bool>,
    current: Animation<f32>,
    zoom: Animation<bool>,
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
            Task::perform(Collection::list(), Message::Listed),
        )
    }

    pub fn update(&mut self, message: Message, database: &Database, session: &Session) -> Action {
        match message {
            Message::Listed(Ok(collections)) => {
                if collections.is_empty() {
                    self.state = State::Creation {
                        name: "Red".to_owned(),
                        collections,
                    };

                    Action::None
                } else {
                    let load_images = Task::batch(collections.iter().map(|collection| {
                        Task::batch(
                            collection
                                .rarest_cards(database)
                                .take(8)
                                .map(|card| card::Image::fetch(card, database, session))
                                .map(Task::future),
                        )
                        .collect()
                        .map(Message::ImagesLoaded.with(collection.name.clone()))
                    }));

                    self.state = State::Selection {
                        collections,
                        animations: HashMap::new(),
                        now: Instant::now(),
                    };

                    Action::Run(load_images)
                }
            }
            Message::ImagesLoaded(collection, images) => {
                let State::Selection { animations, .. } = &mut self.state else {
                    return Action::None;
                };

                let Ok(images): Result<Vec<_>, _> = images.into_iter().collect() else {
                    return Action::None;
                };

                let images: Vec<_> = images
                    .into_iter()
                    .map(|image| image::Handle::from_rgba(image.width, image.height, image.rgba))
                    .collect();

                if images.is_empty() {
                    return Action::None;
                }

                animations.insert(
                    collection,
                    AnimationSet {
                        fade_in: Animation::new(false)
                            .duration(seconds(1))
                            .easing(animation::Easing::EaseIn)
                            .go(true),
                        current: Animation::new(0.0)
                            .duration(seconds(2))
                            .delay(seconds(2))
                            .go(1.0),
                        zoom: Animation::new(false).quick(),
                        images,
                    },
                );

                Action::None
            }
            Message::Hovered(collection, hovered) => {
                let State::Selection { animations, .. } = &mut self.state else {
                    return Action::None;
                };

                if let Some(animation) = animations.get_mut(&collection) {
                    animation.zoom.go_mut(hovered);
                }

                Action::None
            }
            Message::Select(collection) => Action::Select(collection),
            Message::Create(name) => {
                Action::Run(Task::perform(Collection::create(name), Message::Created))
            }
            Message::New => {
                let State::Selection { collections, .. } = &self.state else {
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
            Message::Created(Ok(_collection)) => {
                self.state = State::Loading;

                Action::Run(Task::perform(Collection::list(), Message::Listed))
            }
            Message::Listed(Err(error)) | Message::Created(Err(error)) => {
                log::error!("{error}");

                Action::None
            }
            Message::Tick(instant) => {
                let State::Selection {
                    now, animations, ..
                } = &mut self.state
                else {
                    return Action::None;
                };

                *now = instant;

                for animation in animations.values_mut() {
                    if !animation.current.is_animating(*now) {
                        animation.current.go_mut(animation.current.value() + 1.0);
                    }
                }

                Action::None
            }
        }
    }

    pub fn view(&self, database: &Database, prices: &pricing::Map) -> Element<Message> {
        let content: Element<_> = match &self.state {
            State::Loading => text("Loading...").height(512).center().into(),
            State::Selection {
                collections,
                animations,
                now,
            } => column![
                column(collections.iter().map(|collection| card(
                    collection,
                    animations.get(&collection.name),
                    database,
                    prices,
                    *now,
                )))
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
            center(column![logo(50), content].spacing(20).align_x(Center)).padding(20),
            legal_disclaimer()
        ]
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match &self.state {
            State::Selection {
                animations, now, ..
            } if animations.values().any(|animation| {
                animation.fade_in.is_animating(*now)
                    || animation.current.is_animating(*now)
                    || animation.zoom.is_animating(*now)
            }) =>
            {
                window::frames().map(Message::Tick)
            }
            _ => Subscription::none(),
        }
    }
}

fn card<'a>(
    collection: &'a Collection,
    animations: Option<&'a AnimationSet>,
    database: &Database,
    prices: &pricing::Map,
    now: Instant,
) -> Element<'a, Message> {
    let name = text(collection.name.as_str()).size(25);

    let total_cards = collection.total_cards();
    let unique_cards = collection.unique_cards();
    let total_pokemon = collection.total_pokemon(database);
    let total_value = prices.total_value(collection);

    let stat = |stat| text(stat).size(14);

    let progress = binder::Mode::GottaCatchEmAll.progress(collection, database);

    let badge = stat(format!(
        "{level} ({progress:.0}%)",
        level = match progress as u32 {
            0..10 => "Beginner",
            10..20 => "Intermediate",
            20..40 => "Advanced",
            40..60 => "Pokéxpert",
            70..80 => "Master",
            80..100 => "Elite Four",
            100 => "Champion",
            _ => "Unknown",
        }
    ));

    let stats = row![
        stat(format!("{total_pokemon} Pokémon")),
        stat(format!("{unique_cards} unique")),
        stat(format!(
            "{total_cards} card{}",
            if total_cards == 1 { "" } else { "s" }
        )),
    ]
    .spacing(20);

    let content = column![
        name,
        badge,
        vertical_space(),
        row![
            stat(format!("{}", total_value.america)),
            stat(format!("{}", total_value.europe)),
        ]
        .spacing(20),
        stats,
    ]
    .width(Fill)
    .spacing(10)
    .align_x(Center);

    let content: Element<_> = if let Some(animations) = animations {
        let current = animations.current.interpolate_with(|value| value, now) + 1.0;
        let fade_in = animations.fade_in.interpolate(0.0, 1.0, now);
        let _zoom = animations.zoom.interpolate(1.0, 1.2, now);

        stack![
            container(
                image(&animations.images[(current as usize - 1) % animations.images.len()])
                    .content_fit(ContentFit::Cover)
                    .opacity(fade_in * (1.0 - current.fract()))
            )
            .padding(1),
            container(
                image(&animations.images[current as usize % animations.images.len()])
                    .content_fit(ContentFit::Cover)
                    .opacity(fade_in * current.fract())
            )
            .padding(1),
            mouse_area(container(content).padding(20).style(move |_theme| {
                container::Style::default()
                    .background(
                        gradient::Linear::new(Degrees(180.0))
                            .add_stop(0.05, Color::BLACK.scale_alpha(0.98))
                            .add_stop(0.5, Color::TRANSPARENT)
                            .add_stop(0.95, Color::BLACK.scale_alpha(0.98)),
                    )
                    .border(border::rounded(12.0))
            }))
            .on_enter(Message::Hovered(collection.name.clone(), true))
            .on_exit(Message::Hovered(collection.name.clone(), false))
        ]
        .into()
    } else {
        container(content).padding(20).style(container::dark).into()
    };

    button(content)
        .width(367)
        .height(512)
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
