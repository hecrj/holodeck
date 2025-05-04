use crate::binder;
use crate::card;
use crate::card::pricing;
use crate::icon;
use crate::pokebase::{Card, Database, Session};
use crate::widget::pokeball;
use crate::{Binder, Collection};

use iced::animation;
use iced::border;
use iced::keyboard;
use iced::task;
use iced::time::{Instant, milliseconds};
use iced::widget::{
    bottom, bottom_right, button, center, center_x, center_y, column, container, float, grid,
    horizontal_space, image, mouse_area, opaque, pick_list, pop, right, row, scrollable, stack,
    text, text_input,
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
        query: String,
        search: card::Search,
        animations: HashMap<card::Id, AnimationSet>,
        search_task: Option<task::Handle>,
        price_task: Option<task::Handle>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    ModeSelected(binder::Mode),
    PreviousPage,
    NextPage,
    Add,
    SearchChanged(String),
    SearchFinished(card::Search),
    Close,
    CardShown(card::Id, Source),
    CardHovered(card::Id, Source, bool),
    CardChosen(card::Id, Source),
    ImageFetched(card::Id, Result<card::Image, anywho::Error>),
    PriceFetched(card::Id, Result<card::Pricing, anywho::Error>),
    CollectionSaved(Result<(), anywho::Error>),
    TabPressed { shift: bool },
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
            Message::Add => {
                if let State::Adding { .. } = &self.state {
                    return Task::none();
                }

                let (search_cards, handle) =
                    Task::perform(card::search("", database), Message::SearchFinished).abortable();

                self.state = State::Adding {
                    query: String::new(),
                    search: card::Search::new([]),
                    animations: HashMap::new(),
                    search_task: Some(handle.abort_on_drop()),
                    price_task: None,
                };

                Task::batch([text_input::focus("search"), search_cards])
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
            Message::CardChosen(card, source) => match source {
                Source::Binder => {
                    // TODO: Open card details
                    Task::none()
                }
                Source::Search => self.add(card, collection, database),
            },
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
                                return text_input::focus("search");
                            }

                            index - 1
                        } else {
                            index + 1
                        };

                        if let Some(card) = search.matches().get(new_index) {
                            if let Some(animation) = animations.get_mut(&card.id) {
                                animation.zoom.go_mut(true, now);
                            }
                        }
                    }
                    None => {
                        if shift {
                            return text_input::focus("search");
                        }

                        if let Some(card) = search.matches().first() {
                            if let Some(animation) = animations.get_mut(&card.id) {
                                animation.zoom.go_mut(true, now);
                            }
                        }
                    }
                }

                // TODO: Unfocus operation
                text_input::focus("")
            }
            Message::EnterPressed => {
                let State::Adding {
                    search, animations, ..
                } = &self.state
                else {
                    return Task::none();
                };

                let Some(card) = search.matches().iter().find(|card| {
                    animations
                        .get(&card.id)
                        .is_some_and(|animation| animation.zoom.value())
                }) else {
                    return Task::none();
                };

                self.add(card.id.clone(), collection, database)
            }
            Message::EscapePressed => {
                let State::Adding {
                    search, animations, ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                for card in search.matches().iter().take(100) {
                    if let Some(animation) = animations.get_mut(&card.id) {
                        if animation.zoom.value() {
                            animation.zoom.go_mut(false, now);

                            return Task::none();
                        }
                    }
                }

                self.state = State::Idle;

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
        collection: &mut Collection,
        database: &Database,
    ) -> Task<Message> {
        self.state = State::Idle;

        if let Some(position) = self.mode.position(&card, database) {
            self.spread = self.binders.spread(self.binders.place(position));
            let _ = self.animations.remove(&card);
        }

        collection.add(card);

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
            .on_press(Message::Add)
            .padding([0, 10]);

            let controls = row![mode, add].spacing(10).height(Shrink).align_y(Center);

            row![
                controls,
                horizontal_space(),
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
            binder::Surface::Cover => horizontal_space().into(),
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
                query,
                search,
                animations,
                ..
            } => Some(self.adding(
                query,
                search.matches(),
                animations,
                collection,
                database,
                prices,
                now,
            )),
        };

        let has_overlay = overlay.is_some();

        stack![content]
            .push_maybe(overlay.map(|overlay| {
                opaque(container(overlay).width(Fill).height(Fill).style(|_theme| {
                    container::Style::default().background(Color::BLACK.scale_alpha(0.8))
                }))
            }))
            .push_maybe(has_overlay.then(|| {
                container(
                    button(icon::cancel().size(24))
                        .on_press(Message::Close)
                        .style(button::text),
                )
                .align_right(Fill)
                .padding(5)
            }))
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
            grid(content.range.map(|i| {
                self.mode
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
                    })
            }))
            .columns(binder.columns)
            .height(grid::aspect_ratio(734, 1024))
            .spacing(5),
        )
        .into()
    }

    fn adding<'a>(
        &'a self,
        query: &'a str,
        matches: &'a [Card],
        animations: &'a HashMap<card::Id, AnimationSet>,
        collection: &'a Collection,
        database: &'a Database,
        prices: &pricing::Map,
        now: Instant,
    ) -> Element<'a, Message> {
        let input = container(
            text_input("Search for your card...", query)
                .on_input(Message::SearchChanged)
                .padding(10)
                .id("search"),
        )
        .max_width(600);

        let content: Element<_> = {
            // TODO: Infinite scrolling (?)
            let matches: Element<_> = if !query.is_empty() && matches.is_empty() {
                center(
                    container(text!("No cards were found matching: \"{query}\" :/"))
                        .padding(10)
                        .style(container::bordered_box),
                )
                .into()
            } else {
                scrollable(
                    grid(matches.iter().take(100).map(|card| {
                        let owned_tag = |amount| {
                            right(
                                container(text!("Owned x{amount}").size(10))
                                    .padding(5)
                                    .style(|_theme| {
                                        container::Style::default()
                                            .background(Color::BLACK.scale_alpha(0.8))
                                            .border(border::rounded(8))
                                    }),
                            )
                            .padding(5)
                        };

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
                            .padding(1)
                        ]
                        .push_maybe(collection.cards.get(&card.id).map(owned_tag))
                        .into()
                    }))
                    .fluid(300)
                    .height(grid::aspect_ratio(734, 1024))
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

    pub fn subscription(&self, now: Instant) -> Subscription<Message> {
        let hotkeys = keyboard::on_key_press(|key, modifiers| {
            use keyboard::key::{Key, Named};

            Some(match key.as_ref() {
                Key::Named(Named::ArrowLeft) if modifiers.is_empty() => Message::PreviousPage,
                Key::Named(Named::ArrowRight) if modifiers.is_empty() => Message::NextPage,
                Key::Named(Named::Escape) => Message::EscapePressed,
                Key::Named(Named::Tab) => Message::TabPressed {
                    shift: modifiers.shift(),
                },
                Key::Named(Named::Enter) => Message::EnterPressed,
                Key::Character("a") if modifiers.is_empty() => Message::Add,
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

        Subscription::batch([hotkeys, animation])
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
    let item: Element<_> = match thumbnail {
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
                .height(Fill)
                .content_fit(ContentFit::Cover)
                .opacity(opacity);

            let stats = (shadow > 0.0).then(move || {
                let translucent = move |_theme: &_| {
                    use iced::gradient;

                    container::Style::default()
                        .background(
                            gradient::Linear::new(0)
                                .add_stop(0.0, Color::BLACK.scale_alpha(shadow))
                                .add_stop(shadow * 0.4, Color::TRANSPARENT),
                        )
                        .border(border::rounded(10))
                };

                let metadata = {
                    let name = typewriter(card.name.as_str()).size(12);

                    let set = database.sets.get(&card.set).map(|set| {
                        typewriter(format!("{} (#{})", set.name.as_str(), card.id.as_str()))
                            .size(7)
                            .very_quick()
                    });

                    column![name].push_maybe(set).spacing(5)
                };

                let pricing = price.map(|price| {
                    let dollars = price
                        .america
                        .spread()
                        .map(|spread| typewriter(spread.average.to_string()).size(7));

                    let euros = price
                        .europe
                        .spread()
                        .map(|spread| typewriter(spread.average.to_string()).size(7));

                    row![].push_maybe(dollars).push_maybe(euros).spacing(8)
                });

                let stats: Element<_> = if shadow == 1.0 {
                    container(
                        row![metadata, horizontal_space()]
                            .push_maybe(pricing)
                            .spacing(5)
                            .align_y(Bottom),
                    )
                    .padding(8)
                    .into()
                } else {
                    horizontal_space().into()
                };

                bottom(stats).width(Fill).style(translucent)
            });

            let card = mouse_area(
                button(
                    float(stack![container(image).padding(1)].push_maybe(stats))
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
                            shadow_border_radius: border::radius(10.0 * scale),
                        }),
                )
                .on_press_with(move || Message::CardChosen(card.id.clone(), source))
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
        _ => slot(horizontal_space()),
    };

    pop(item)
        .key(card.id.as_str())
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
    container(horizontal_space())
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
