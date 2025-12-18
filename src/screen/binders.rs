use crate::binder;
use crate::card;
use crate::card::pricing;
use crate::collection;
use crate::icon;
use crate::pokebase::{Card, Database, Session};
use crate::widget::pokeball;
use crate::{Binder, Collection};

use iced::animation;
use iced::border;
use iced::keyboard;
use iced::padding;
use iced::task;
use iced::time::{Instant, milliseconds};
use iced::widget::{
    bottom, bottom_right, button, center, center_x, center_y, column, container, float, grid,
    image, mouse_area, opaque, operation, pick_list, right, right_center, row, scrollable, sensor,
    space, stack, text, text_input, tooltip,
};
use iced::window;
use iced::{
    Animation, Bottom, Center, Color, ContentFit, Element, Fill, Shadow, Shrink, Subscription,
    Task, Theme,
};
use iced_palace::widget::typewriter;

use function::Binary;
use std::collections::HashMap;
use tokio::time;

pub struct Binders {
    binders: binder::Set,
    spread: binder::Spread,
    mode: binder::Mode,
    state: State,
    images: HashMap<card::Id, Image>,
    animations: HashMap<card::Id, AnimationSet>,
}

enum Image {
    Loading,
    Loaded(image::Handle),
    Errored,
}

enum State {
    Idle,
    Adding {
        variant: collection::Variant,
        query: String,
        search: card::Search,
        animations: HashMap<card::Id, AnimationSet>,
        search_task: Option<task::Handle>,
        price_task: Option<task::Handle>,
    },
    Scanning {
        variant: collection::Variant,
        capture: Option<image::Handle>,
        found: Option<card::Id>,
        added: Vec<card::Id>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    ModeSelected(binder::Mode),
    PreviousPage,
    NextPage,
    Add(collection::Variant),
    Scan,
    #[cfg(feature = "scanner")]
    Scanning(pokedetect::Event),
    ToggleReverseHolofoil,
    SearchChanged(String),
    SearchFinished(card::Search),
    Close,
    CardShown(card::Id, Source),
    CardHovered(card::Id, Source, bool),
    ShowCard(card::Id),
    AddCard(card::Id),
    ImageFetched(card::Id, Result<card::Image, anywho::Error>),
    PriceFetched(card::Id, Result<card::Pricing, anywho::Error>),
    CollectionSaved(Result<(), anywho::Error>),
    TabPressed {
        shift: bool,
    },
    EscapePressed,
    EnterPressed,
    Tick,
}

#[derive(Debug, Clone, Copy)]
pub enum Source {
    Binder,
    Search,
}

impl Source {
    fn zoom(self) -> f32 {
        match self {
            Source::Binder => 1.5,
            Source::Search => 1.2,
        }
    }
}

impl Binders {
    pub fn new() -> Self {
        Self {
            binders: binder::Set::default(),
            spread: binder::Spread::default(),
            mode: binder::Mode::GottaCatchEmAll,
            state: State::Idle,
            images: HashMap::new(),
            animations: HashMap::new(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        collection: &mut Collection,
        database: &Database,
        prices: &mut pricing::Map,
        session: &Session,
        now: Instant,
    ) -> Task<Message> {
        match message {
            Message::ModeSelected(mode) => {
                self.mode = mode;

                Task::none()
            }
            Message::PreviousPage => {
                let State::Idle = self.state else {
                    return Task::none();
                };

                if let Some(spread) = self.spread.decrement() {
                    self.spread = spread;
                    self.animations.clear();
                }

                Task::none()
            }
            Message::NextPage => {
                let State::Idle = self.state else {
                    return Task::none();
                };

                let total_pages = self.binders.total_pages(self.mode.total_cards(database));

                let new_spread = self
                    .spread
                    .increment()
                    .min(self.binders.spread(total_pages));

                if new_spread != self.spread {
                    self.spread = new_spread;
                    self.animations.clear();
                }

                Task::none()
            }
            Message::Add(variant) => {
                if let State::Adding { .. } = &self.state {
                    return Task::none();
                }

                let (search_cards, handle) =
                    Task::perform(card::search("", database), Message::SearchFinished).abortable();

                self.state = State::Adding {
                    variant,
                    query: String::new(),
                    search: card::Search::new([]),
                    animations: HashMap::new(),
                    search_task: Some(handle.abort_on_drop()),
                    price_task: None,
                };

                Task::batch([operation::focus("search"), search_cards])
            }
            Message::Scan => {
                self.state = State::Scanning {
                    variant: collection::Variant::Normal,
                    capture: None,
                    found: None,
                    added: Vec::new(),
                };

                Task::none()
            }
            #[cfg(feature = "scanner")]
            Message::Scanning(event) => {
                let State::Scanning { capture, found, .. } = &mut self.state else {
                    return Task::none();
                };

                match event {
                    pokedetect::Event::Captured(image) => {
                        *capture = Some(image::Handle::from_rgba(
                            image.width,
                            image.height,
                            image.rgba,
                        ));

                        Task::none()
                    }
                    pokedetect::Event::Detected { set, number } if found.is_none() => {
                        let Some(card) = database.find(&set, &number) else {
                            return Task::none();
                        };

                        *found = Some(card.id.clone());

                        let fetch_image = if self.images.contains_key(&card.id) {
                            Task::none()
                        } else {
                            let _ = self.images.insert(card.id.clone(), Image::Loading);

                            Task::perform(
                                card::Image::fetch(card, database, session),
                                Message::ImageFetched.with(card.id.clone()),
                            )
                        };

                        let fetch_price = if prices.contains(&card.id) {
                            Task::none()
                        } else {
                            Task::perform(
                                card::Pricing::fetch(card, session),
                                Message::PriceFetched.with(card.id.clone()),
                            )
                        };

                        Task::batch([fetch_image, fetch_price])
                    }
                    _ => Task::none(),
                }
            }
            Message::ToggleReverseHolofoil => {
                if let State::Adding { variant, .. } | State::Scanning { variant, .. } =
                    &mut self.state
                {
                    *variant = match variant {
                        collection::Variant::Normal => collection::Variant::Reverse,
                        collection::Variant::Reverse => collection::Variant::Normal,
                    };
                }

                Task::none()
            }
            Message::SearchChanged(new_query) => {
                let State::Adding {
                    query, search_task, ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                let (search_cards, handle) = {
                    let search = card::search(&new_query, database);

                    Task::perform(
                        async move {
                            time::sleep(milliseconds(250)).await;
                            search.await
                        },
                        Message::SearchFinished,
                    )
                    .abortable()
                };

                *query = new_query;
                *search_task = Some(handle.abort_on_drop());

                search_cards
            }
            Message::SearchFinished(result) => {
                let State::Adding {
                    search,
                    search_task,
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                *search = result;
                *search_task = None;

                Task::none()
            }
            Message::Close => {
                self.state = State::Idle;

                Task::none()
            }
            Message::CardShown(card, source) => {
                let Some(card) = database.cards.get(&card) else {
                    return Task::none();
                };

                if self.images.contains_key(&card.id) {
                    match source {
                        Source::Binder => {
                            self.animations
                                .insert(card.id.clone(), AnimationSet::new(now));
                        }
                        Source::Search => {
                            if let State::Adding { animations, .. } = &mut self.state {
                                animations.insert(card.id.clone(), AnimationSet::new(now));
                            }
                        }
                    }

                    return Task::none();
                }

                let _ = self.images.insert(card.id.clone(), Image::Loading);

                Task::perform(
                    card::Image::fetch(card, database, session),
                    Message::ImageFetched.with(card.id.clone()),
                )
            }
            Message::CardHovered(card, source, hovered) => match source {
                Source::Binder => {
                    if let Some(animations) = self.animations.get_mut(&card) {
                        animations.zoom.go_mut(hovered, now);
                    }

                    Task::none()
                }
                Source::Search => {
                    let State::Adding {
                        animations,
                        price_task,
                        ..
                    } = &mut self.state
                    else {
                        return Task::none();
                    };

                    let Some(card) = database.cards.get(&card) else {
                        return Task::none();
                    };

                    for animation in animations.values_mut() {
                        animation.zoom.go_mut(false, now);
                    }

                    if let Some(animations) = animations.get_mut(&card.id) {
                        animations.zoom.go_mut(hovered, now);
                    }

                    if !hovered || prices.contains(&card.id) {
                        *price_task = None;
                        return Task::none();
                    }

                    let (task, handle) = Task::perform(
                        {
                            let fetch_price = card::Pricing::fetch(card, session);

                            async move {
                                time::sleep(milliseconds(500)).await;
                                fetch_price.await
                            }
                        },
                        Message::PriceFetched.with(card.id.clone()),
                    )
                    .abortable();

                    *price_task = Some(handle.abort_on_drop());
                    task
                }
            },
            Message::ShowCard(card) => {
                // TODO
                dbg!(card);

                Task::none()
            }
            Message::AddCard(card) => {
                let State::Adding { variant, .. } = &self.state else {
                    return Task::none();
                };

                let variant = *variant;

                self.add(card, variant, collection, database)
            }
            Message::ImageFetched(card, Ok(image)) => {
                let _ = self.images.insert(
                    card.clone(),
                    Image::Loaded(image::Handle::from_rgba(
                        image.width,
                        image.height,
                        image.rgba,
                    )),
                );

                if let State::Adding { animations, .. } = &mut self.state {
                    animations.insert(card.clone(), AnimationSet::new(now));
                }

                self.animations.insert(card, AnimationSet::new(now));

                Task::none()
            }
            Message::PriceFetched(id, Ok(pricing)) => {
                prices.insert(id, pricing);

                Task::none()
            }
            Message::CollectionSaved(Ok(_)) => Task::none(),
            Message::TabPressed { shift } => {
                let State::Adding {
                    search, animations, ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                let focus = search
                    .matches()
                    .iter()
                    .take(100) // TODO: Remove limit when auto-scrolling
                    .enumerate()
                    .find_map(|(i, card)| {
                        let animation = animations.get(&card.id)?;

                        if animation.zoom.value() {
                            Some((i, card))
                        } else {
                            None
                        }
                    });

                match focus {
                    Some((index, card)) => {
                        if let Some(animation) = animations.get_mut(&card.id) {
                            animation.zoom.go_mut(false, now);
                        }

                        let new_index = if shift {
                            if index == 0 {
                                return operation::focus("search");
                            }

                            index - 1
                        } else {
                            index + 1
                        };

                        if let Some(card) = search.matches().get(new_index)
                            && let Some(animation) = animations.get_mut(&card.id)
                        {
                            animation.zoom.go_mut(true, now);
                        }
                    }
                    None => {
                        if shift {
                            return operation::focus("search");
                        }

                        if let Some(card) = search.matches().first()
                            && let Some(animation) = animations.get_mut(&card.id)
                        {
                            animation.zoom.go_mut(true, now);
                        }
                    }
                }

                // TODO: Unfocus operation
                operation::focus("")
            }
            Message::EnterPressed => match &mut self.state {
                State::Idle => Task::none(),
                State::Adding {
                    search,
                    animations,
                    variant,
                    ..
                } => {
                    let Some(card) = search.matches().iter().find(|card| {
                        animations
                            .get(&card.id)
                            .is_some_and(|animation| animation.zoom.value())
                    }) else {
                        return Task::none();
                    };

                    let card = card.id.clone();
                    let variant = *variant;

                    self.add(card, variant, collection, database)
                }
                State::Scanning {
                    variant,
                    found,
                    added,
                    ..
                } => {
                    let Some(card) = found.clone() else {
                        return Task::none();
                    };

                    added.push(card.clone());

                    *found = None;

                    let variant = *variant;
                    self.add(card, variant, collection, database)
                }
            },
            Message::EscapePressed => {
                match &mut self.state {
                    State::Adding {
                        animations, search, ..
                    } => {
                        for card in search.matches().iter().take(100) {
                            if let Some(animation) = animations.get_mut(&card.id)
                                && animation.zoom.value()
                            {
                                animation.zoom.go_mut(false, now);

                                return Task::none();
                            }
                        }

                        self.state = State::Idle;
                    }
                    State::Scanning { found, .. } => {
                        if found.is_some() {
                            *found = None;
                        } else {
                            self.state = State::Idle;
                        }
                    }
                    _ => {}
                }

                Task::none()
            }
            Message::Tick => Task::none(),
            Message::ImageFetched(card, Err(error)) => {
                log::error!("{error}");

                let _ = self.images.insert(card, Image::Errored);

                Task::none()
            }
            Message::CollectionSaved(Err(error)) | Message::PriceFetched(_, Err(error)) => {
                log::error!("{error}");

                Task::none()
            }
        }
    }

    pub fn add(
        &mut self,
        card: card::Id,
        variant: collection::Variant,
        collection: &mut Collection,
        database: &Database,
    ) -> Task<Message> {
        if let State::Adding { .. } = &self.state {
            self.state = State::Idle;

            if let Some(position) = self.mode.position(&card, database) {
                self.spread = self.binders.spread(self.binders.place(position));
                let _ = self.animations.remove(&card);
            }
        }

        collection.add(card, variant);

        Task::perform(collection.save(), Message::CollectionSaved).discard()
    }

    pub fn view<'a>(
        &'a self,
        collection: &'a Collection,
        database: &'a Database,
        prices: &pricing::Map,
        now: Instant,
    ) -> Element<'a, Message> {
        let Some(pair) = self.binders.open(self.spread) else {
            // TODO
            return center(text("This page does not exist!")).into();
        };

        let page = match (&pair.left, &pair.right) {
            (_, binder::Surface::Content(content)) | (binder::Surface::Content(content), _) => {
                content.page
            }
            _ => binder::Page::default(),
        };

        let header = {
            fn stat<'a>(
                icon: impl Into<Element<'a, Message>>,
                content: String,
            ) -> Element<'a, Message> {
                row![icon.into(), text(content).size(12)]
                    .spacing(10)
                    .align_y(Center)
                    .into()
            }

            let progress = {
                let total_cards = self.mode.total_cards(database);

                stat(
                    pokeball(12),
                    format!(
                        "{owned_pokemon} / {total_cards} ({completion:.1}%)",
                        owned_pokemon = collection.total_pokemon(database),
                        completion = self.mode.progress(collection, database),
                    ),
                )
            };

            let binders = stat(
                icon::book().size(12),
                format!(
                    "{binder} / {total_binders}",
                    binder = pair.binder_number + 1,
                    total_binders = self.binders.len()
                ),
            );

            let pages = {
                stat(
                    icon::binder().size(12),
                    format!("{page} / {pages}", pages = pair.binder.pages),
                )
            };

            let mode = pick_list(binder::Mode::ALL, Some(self.mode), Message::ModeSelected)
                .padding([5, 10])
                .text_size(12);

            let add = button(
                row![
                    icon::add().size(12).height(Fill).center(),
                    text("Add").size(12),
                ]
                .align_y(Center)
                .spacing(5),
            )
            .on_press(Message::Add(collection::Variant::Normal))
            .padding([0, 10]);

            let scan = cfg!(feature = "scanner").then(|| {
                button(
                    row![
                        icon::camera().size(12).height(Fill).center(),
                        text("Scan").size(12),
                    ]
                    .align_y(Center)
                    .spacing(5),
                )
                .on_press(Message::Scan)
                .padding([0, 10])
                .style(button::success)
            });

            let controls = row![mode, add, scan]
                .spacing(10)
                .height(Shrink)
                .align_y(Center);

            row![
                controls,
                space::horizontal(),
                row![progress, binders, pages].spacing(30).align_y(Center)
            ]
            .height(30)
            .align_y(Center)
            .spacing(20)
        };

        let left_page = match pair.left {
            binder::Surface::Cover => center(
                column![
                    text!("{name}'s\nCollection", name = collection.name.as_str())
                        .size(40)
                        .center(),
                    text(to_roman(pair.binder_number + 1)).size(30)
                ]
                .spacing(10)
                .align_x(Center),
            )
            .into(),
            binder::Surface::Content(content) => {
                self.page(pair.binder, content, collection, database, prices, now)
            }
        };

        let right_page = match pair.right {
            binder::Surface::Cover => space::horizontal().into(),
            binder::Surface::Content(content) => {
                self.page(pair.binder, content, collection, database, prices, now)
            }
        };

        let content = column![header, row![left_page, right_page].spacing(20)]
            .spacing(10)
            .padding(10);

        let overlay: Option<Element<'_, Message>> = match &self.state {
            State::Idle => None,
            State::Adding {
                variant,
                query,
                search,
                animations,
                ..
            } => Some(self.adding(
                *variant,
                query,
                search.matches(),
                animations,
                collection,
                database,
                prices,
                now,
            )),
            State::Scanning {
                variant,
                capture,
                found,
                added,
            } => Some(self.scanning(
                *variant,
                capture.as_ref(),
                found.as_ref(),
                added.as_slice(),
                collection,
                database,
                prices,
            )),
        };

        let has_overlay = overlay.is_some();

        stack![
            content,
            overlay.map(|overlay| {
                opaque(container(overlay).width(Fill).height(Fill).style(|_theme| {
                    container::Style::default().background(Color::BLACK.scale_alpha(0.8))
                }))
            }),
            has_overlay.then(|| {
                container(
                    button(icon::cancel().size(24))
                        .on_press(Message::Close)
                        .style(button::text),
                )
                .align_right(Fill)
                .padding(5)
            })
        ]
        .into()
    }

    fn page<'a>(
        &'a self,
        binder: Binder,
        content: binder::Content,
        collection: &Collection,
        database: &'a Database,
        prices: &pricing::Map,
        now: Instant,
    ) -> Element<'a, Message> {
        let total = self.mode.total_cards(database);

        center_y(
            grid(content.slots.map(|slot| {
                match slot {
                    binder::Slot::Empty => unused_slot(),
                    binder::Slot::Pokemon(i) => self
                        .mode
                        .card(i, collection, database)
                        .map(|card| {
                            item(
                                card,
                                self.images.get(&card.id),
                                self.animations.get(&card.id),
                                prices.get(&card.id),
                                database,
                                now,
                                Source::Binder,
                            )
                        })
                        .unwrap_or_else(|| {
                            if i < total {
                                placeholder(i)
                            } else {
                                unused_slot()
                            }
                        }),
                }
            }))
            .columns(binder.columns)
            .height(grid::aspect_ratio(card::Image::WIDTH, card::Image::HEIGHT))
            .spacing(5),
        )
        .into()
    }

    fn adding<'a>(
        &'a self,
        variant: collection::Variant,
        query: &'a str,
        matches: &'a [Card],
        animations: &'a HashMap<card::Id, AnimationSet>,
        collection: &'a Collection,
        database: &'a Database,
        prices: &pricing::Map,
        now: Instant,
    ) -> Element<'a, Message> {
        let input = {
            let reverse = reverse_toggle(variant);

            let input = text_input("Search for your card...", query)
                .on_input(Message::SearchChanged)
                .padding(padding::all(10).right(40))
                .id("search");

            container(stack![input, right_center(reverse).padding(10)]).max_width(600)
        };

        let content: Element<'_, _> = {
            // TODO: Infinite scrolling (?)
            let matches: Element<'_, _> = if !query.is_empty() && matches.is_empty() {
                center(
                    container(text!("No cards were found matching: \"{query}\" :/"))
                        .padding(10)
                        .style(container::bordered_box),
                )
                .into()
            } else {
                scrollable(
                    grid(matches.iter().take(100).map(|card| {
                        stack![
                            container(item(
                                card,
                                self.images.get(&card.id),
                                animations.get(&card.id),
                                prices.get(&card.id),
                                database,
                                now,
                                Source::Search,
                            ))
                            .padding(1),
                            collection.cards.get(&card.id).map(owned_tag.with(10.0))
                        ]
                        .into()
                    }))
                    .fluid(300)
                    .height(grid::aspect_ratio(card::Image::WIDTH, card::Image::HEIGHT))
                    .spacing(8),
                )
                .width(Fill)
                .height(Fill)
                .spacing(10)
                .into()
            };

            column![center_x(input), matches].spacing(10).into()
        };

        center(content).padding(10).into()
    }

    pub fn scanning<'a>(
        &'a self,
        variant: collection::Variant,
        capture: Option<&image::Handle>,
        found: Option<&card::Id>,
        added: &'a [card::Id],
        collection: &'a Collection,
        database: &'a Database,
        prices: &pricing::Map,
    ) -> Element<'a, Message> {
        let log = bottom_right(
            container(
                scrollable(column(added.iter().rev().take(30).filter_map(|card| {
                    let card = database.cards.get(card)?;
                    let name = card.name.get("en").map(String::as_str).unwrap_or("Unknown");

                    Some(text!("{name} ({})", card.id.as_str()).into())
                })))
                .spacing(10),
            )
            .padding(10)
            .style(container::dark),
        )
        .padding(10);

        let scanner: Element<'_, Message> =
            if let Some(card) = found.and_then(|card| database.cards.get(card)) {
                if let Some(Image::Loaded(handle)) = self.images.get(&card.id) {
                    center(stack![
                        container(image(handle)).padding(1),
                        bottom(
                            column![
                                reverse_toggle(variant),
                                stats(card, prices.get(&card.id), database, 30.0)
                            ]
                            .spacing(10)
                        )
                        .padding(30)
                        .style(|_theme| translucent(1.0)),
                        collection.cards.get(&card.id).map(owned_tag.with(20.0))
                    ])
                    .padding(10)
                    .into()
                } else {
                    center(text(card.id.as_str()).center())
                        .padding(10)
                        .style(container::dark)
                        .into()
                }
            } else if let Some(capture) = capture {
                center(image(capture)).into()
            } else {
                space::horizontal().into()
            };

        stack![scanner, (!added.is_empty()).then_some(log)].into()
    }

    pub fn subscription(&self, now: Instant) -> Subscription<Message> {
        let hotkeys = keyboard::listen().filter_map(|event| {
            use keyboard::key::{Key, Named};

            let keyboard::Event::KeyPressed {
                modified_key,
                modifiers,
                ..
            } = event
            else {
                return None;
            };

            Some(match modified_key.as_ref() {
                Key::Named(Named::ArrowLeft) if modifiers.is_empty() => Message::PreviousPage,
                Key::Named(Named::ArrowRight) if modifiers.is_empty() => Message::NextPage,
                Key::Named(Named::Escape) => Message::EscapePressed,
                Key::Named(Named::Tab) => Message::TabPressed {
                    shift: modifiers.shift(),
                },
                Key::Named(Named::Enter) => Message::EnterPressed,
                Key::Character("a") if modifiers.is_empty() => {
                    Message::Add(collection::Variant::Normal)
                }
                Key::Character("r") if modifiers.is_empty() => {
                    Message::Add(collection::Variant::Reverse)
                }
                _ => None?,
            })
        });

        let animation = {
            let is_animating = |animations: &HashMap<card::Id, AnimationSet>| {
                animations
                    .values()
                    .any(|animation| animation.is_animating(now))
            };

            let is_animating = if let State::Adding { animations, .. } = &self.state {
                is_animating(&self.animations) || is_animating(animations)
            } else {
                is_animating(&self.animations)
            };

            if is_animating {
                window::frames().map(|_| Message::Tick)
            } else {
                Subscription::none()
            }
        };

        #[cfg(feature = "scanner")]
        let scanner = match &self.state {
            State::Scanning { found: None, .. } => {
                Subscription::run(pokedetect::run).map(Message::Scanning)
            }
            _ => Subscription::none(),
        };

        #[cfg(not(feature = "scanner"))]
        let scanner = Subscription::none();

        Subscription::batch([hotkeys, animation, scanner])
    }
}

fn item<'a>(
    card: &'a Card,
    thumbnail: Option<&'a Image>,
    animations: Option<&'a AnimationSet>,
    price: Option<card::Pricing>,
    database: &'a Database,
    now: Instant,
    source: Source,
) -> Element<'a, Message> {
    let item: Element<'_, _> = match thumbnail {
        Some(Image::Loaded(handle)) => {
            let (opacity, scale, shadow) = if let Some(animations) = animations {
                (
                    animations.fade_in.interpolate(0.0, 1.0, now),
                    animations.zoom.interpolate(1.0, source.zoom(), now),
                    animations.zoom.interpolate(0.0, 1.0, now),
                )
            } else {
                (0.0, 1.0, 0.0)
            };

            let image = image(handle)
                .width(Fill)
                .content_fit(ContentFit::Contain)
                .opacity(opacity);

            let stats = (shadow > 0.0).then(move || {
                let stats: Element<'_, _> = if shadow == 1.0 {
                    stats(card, price, database, 14.0)
                } else {
                    space::horizontal().into()
                };

                bottom(stats)
                    .padding(7)
                    .width(Fill)
                    .style(move |_theme| translucent(shadow))
            });

            let card = mouse_area(
                button(
                    float(stack![
                        container(image).padding(padding::all(1).top(0)),
                        stats
                    ])
                    .scale(match source {
                        Source::Binder => scale * (1.1 - (0.1 * opacity)),
                        Source::Search => scale,
                    })
                    .translate(move |bounds, viewport| {
                        let scale = source.zoom();
                        let final_bounds = bounds.zoom(scale);

                        final_bounds.offset(&viewport.shrink(10)) * shadow
                    })
                    .style(move |_theme| float::Style {
                        shadow: Shadow {
                            color: Color::BLACK.scale_alpha(shadow),
                            blur_radius: 10.0 * shadow,
                            ..Shadow::default()
                        },
                        shadow_border_radius: border::radius(14.0 * scale),
                    }),
                )
                .on_press_with(move || match source {
                    Source::Binder => Message::ShowCard(card.id.clone()),
                    Source::Search => Message::AddCard(card.id.clone()),
                })
                .padding(0)
                .style(button::text),
            )
            .on_enter(Message::CardHovered(card.id.clone(), source, true))
            .on_exit(Message::CardHovered(card.id.clone(), source, false));

            if opacity < 1.0 {
                slot(card)
            } else {
                card.into()
            }
        }
        Some(Image::Errored) => slot(center(
            card.name
                .get("en")
                .map(text)
                .or_else(|| {
                    card.name
                        .get("ja")
                        .map(|name| text(name).shaping(text::Shaping::Advanced))
                })
                .unwrap_or_else(|| text("Unknown"))
                .center()
                .size(14),
        )),
        _ => slot(space::horizontal()),
    };

    sensor(item)
        .key_ref(card.id.as_str())
        .on_show(move |_size| Message::CardShown(card.id.clone(), source))
        .into()
}

fn placeholder<'a>(index: usize) -> Element<'a, Message> {
    slot(
        bottom_right(text!("#{}", index + 1).style(|theme: &Theme| {
            let palette = theme.extended_palette();

            text::Style {
                color: Some(palette.background.weak.color),
            }
        }))
        .padding([5, 8]),
    )
}

fn slot<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .style(|theme| container::dark(theme).border(border::rounded(8)))
        .into()
}

fn unused_slot<'a>() -> Element<'a, Message> {
    container(space::horizontal())
        .style(|_theme| {
            container::Style::default()
                .background(Color::BLACK.scale_alpha(0.3))
                .border(border::rounded(8))
        })
        .into()
}

struct AnimationSet {
    fade_in: Animation<bool>,
    zoom: Animation<bool>,
}

impl AnimationSet {
    fn new(now: Instant) -> Self {
        Self {
            fade_in: Animation::new(false)
                .easing(animation::Easing::EaseInOut)
                .slow()
                .go(true, now),
            zoom: Animation::new(false).quick(),
        }
    }

    fn is_animating(&self, at: Instant) -> bool {
        self.fade_in.is_animating(at) || self.zoom.is_animating(at)
    }
}

fn to_roman(number: usize) -> String {
    match number {
        1 => "I".to_owned(),
        2 => "II".to_owned(),
        3 => "III".to_owned(),
        4 => "IV".to_owned(),
        5 => "V".to_owned(),
        6 => "VI".to_owned(),
        7 => "VII".to_owned(),
        8 => "VIII".to_owned(),
        9 => "IX".to_owned(),
        10 => "X".to_owned(),
        _ => format!("#{number}"),
    }
}

fn stats<'a>(
    card: &'a Card,
    price: Option<card::Pricing>,
    database: &'a Database,
    font_size: f32,
) -> Element<'a, Message> {
    let name = typewriter(card.name.as_str()).size(font_size);

    let set = database.sets.get(&card.set).map(|set| {
        typewriter(format!("{} (#{})", set.name.as_str(), card.id.as_str()))
            .size(font_size / 2.0)
            .very_quick()
    });

    let pricing = price.map(|price| {
        let dollars = price
            .america
            .spread()
            .map(|spread| typewriter(spread.average.to_string()).size(font_size / 2.0));

        let euros = price
            .europe
            .spread()
            .map(|spread| typewriter(spread.average.to_string()).size(font_size / 2.0));

        row![dollars, euros].spacing(font_size / 2.0)
    });

    column![
        name,
        row![set, space::horizontal(), pricing]
            .spacing(font_size / 2.0)
            .align_y(Bottom),
    ]
    .spacing(font_size / 2.0)
    .into()
}

fn owned_tag(size: f32, amount: &collection::Amount) -> Element<'_, Message> {
    right(
        container(text!("Owned x{amount}", amount = amount.total()).size(size))
            .padding(size / 2.0)
            .style(move |_theme| {
                container::Style::default()
                    .background(Color::BLACK.scale_alpha(0.8))
                    .border(border::rounded(size))
            }),
    )
    .padding(size / 2.0)
    .into()
}

fn reverse_toggle<'a>(variant: collection::Variant) -> Element<'a, Message> {
    tooltip(
        button(text("R").size(14).width(Fill).height(Fill).center())
            .width(20)
            .height(20)
            .padding(0)
            .on_press(Message::ToggleReverseHolofoil)
            .style(move |_theme, _status| {
                use iced::gradient;
                use iced::{Degrees, color};

                let alpha = match variant {
                    collection::Variant::Normal => 0.3,
                    collection::Variant::Reverse => 1.0,
                };

                button::Style {
                    border: border::rounded(2),
                    ..button::Style::default().with_background(
                        gradient::Linear::new(Degrees(135.0))
                            .add_stop(0.0, color!(0xaaffaa).scale_alpha(alpha))
                            .add_stop(0.5, color!(0xffffaa).scale_alpha(alpha))
                            .add_stop(1.0, color!(0xffaaff).scale_alpha(alpha)),
                    )
                }
            }),
        container(text("Reverse Holofoil").size(12))
            .padding(5)
            .style(container::dark),
        tooltip::Position::Bottom,
    )
    .into()
}

fn translucent(shadow: f32) -> container::Style {
    use iced::gradient;

    container::Style::default()
        .background(
            gradient::Linear::new(0)
                .add_stop(0.0, Color::BLACK.scale_alpha(shadow))
                .add_stop(shadow * 0.4, Color::TRANSPARENT),
        )
        .border(border::rounded(14.0))
}
