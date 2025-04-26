use crate::binder;
use crate::card;
use crate::icon;
use crate::pokebase::database;
use crate::pokebase::{Card, Database};
use crate::widget::pokeball;
use crate::{Binder, Collection};

use iced::animation;
use iced::border;
use iced::keyboard;
use iced::task;
use iced::time::{Instant, milliseconds};
use iced::widget::{
    bottom_right, button, center, center_x, center_y, column, container, grid, horizontal_space,
    image, mouse_area, opaque, pick_list, pop, right, row, scrollable, stack, text, text_input,
};
use iced::window;
use iced::{
    Animation, Center, Color, ContentFit, Element, Fill, Shadow, Shrink, Subscription, Task, Theme,
};

use function::Binary;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time;

pub struct Binders {
    binders: binder::Set,
    spread: binder::Spread,
    mode: binder::Mode,
    state: State,
    images: HashMap<card::Id, Image>,
    animations: HashMap<card::Id, AnimationSet>,
    now: Instant,
}

enum Image {
    Loading,
    Loaded(image::Handle),
    Errored,
}

enum State {
    Idle,
    Adding {
        search: String,
        matches: Arc<[Card]>,
        task: Option<task::Handle>,
        animations: HashMap<card::Id, AnimationSet>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    ModeSelected(binder::Mode),
    PreviousPage,
    NextPage,
    Add,
    SearchChanged(String),
    SearchFinished(database::Search<Card>),
    Close,
    CardShown(card::Id, Source),
    CardHovered(card::Id, Source, bool),
    CardChosen(card::Id, Source),
    ImageFetched(card::Id, Result<card::Image, anywho::Error>),
    CollectionSaved(Result<(), anywho::Error>),
    TabPressed { shift: bool },
    EscapePressed,
    EnterPressed,
    Tick(Instant),
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
            now: Instant::now(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        collection: &mut Collection,
        database: &Database,
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
                    Task::perform(database.search_cards(""), Message::SearchFinished).abortable();

                self.state = State::Adding {
                    search: String::new(),
                    matches: Arc::new([]),
                    animations: HashMap::new(),
                    task: Some(handle.abort_on_drop()),
                };

                Task::batch([text_input::focus("search"), search_cards])
            }
            Message::SearchChanged(new_search) => {
                let State::Adding { search, task, .. } = &mut self.state else {
                    return Task::none();
                };

                let (search_cards, handle) = {
                    let search = database.search_cards(&new_search);

                    Task::perform(
                        async move {
                            time::sleep(milliseconds(250)).await;
                            search.await
                        },
                        Message::SearchFinished,
                    )
                    .abortable()
                };

                *search = new_search;
                *task = Some(handle.abort_on_drop());

                search_cards
            }
            Message::SearchFinished(search) => {
                let State::Adding { matches, task, .. } = &mut self.state else {
                    return Task::none();
                };

                let old_matches = std::mem::replace(matches, search.matches);
                *task = None;

                Task::future(async move {
                    let _ = tokio::task::spawn_blocking(move || {
                        // Free memory concurrently
                        drop(old_matches);
                    })
                    .await;
                })
                .discard()
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
                            self.animations.insert(card.id.clone(), AnimationSet::new());
                        }
                        Source::Search => {
                            if let State::Adding { animations, .. } = &mut self.state {
                                animations.insert(card.id.clone(), AnimationSet::new());
                            }
                        }
                    }

                    return Task::none();
                }

                let _ = self.images.insert(card.id.clone(), Image::Loading);

                Task::perform(
                    card::Image::fetch(card, database),
                    Message::ImageFetched.with(card.id.clone()),
                )
            }
            Message::CardHovered(card, source, hovered) => {
                match source {
                    Source::Binder => {
                        if let Some(animations) = self.animations.get_mut(&card) {
                            animations.zoom.go_mut(hovered);
                        }
                    }
                    Source::Search => {
                        if let State::Adding { animations, .. } = &mut self.state {
                            for animation in animations.values_mut() {
                                animation.zoom.go_mut(false);
                            }

                            if let Some(animations) = animations.get_mut(&card) {
                                animations.zoom.go_mut(hovered);
                            }
                        }
                    }
                }

                Task::none()
            }
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
                    animations.insert(card.clone(), AnimationSet::new());
                }

                self.animations.insert(card, AnimationSet::new());

                Task::none()
            }
            Message::CollectionSaved(Ok(_)) => Task::none(),
            Message::TabPressed { shift } => {
                let State::Adding {
                    matches,
                    animations,
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                let focus = matches.iter().enumerate().find_map(|(i, card)| {
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
                            animation.zoom.go_mut(false);
                        }

                        let new_index = if shift {
                            if index == 0 {
                                return text_input::focus("search");
                            }

                            index - 1
                        } else {
                            index + 1
                        };

                        if let Some(card) = matches.get(new_index) {
                            if let Some(animation) = animations.get_mut(&card.id) {
                                animation.zoom.go_mut(true);
                            }
                        }
                    }
                    None => {
                        if shift {
                            return text_input::focus("search");
                        }

                        if let Some(card) = matches.first() {
                            if let Some(animation) = animations.get_mut(&card.id) {
                                animation.zoom.go_mut(true);
                            }
                        }
                    }
                }

                // TODO: Unfocus operation
                text_input::focus("")
            }
            Message::EnterPressed => {
                let State::Adding {
                    matches,
                    animations,
                    ..
                } = &self.state
                else {
                    return Task::none();
                };

                let Some(card) = matches.iter().find(|card| {
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
                    matches,
                    animations,
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                for card in matches.iter() {
                    if let Some(animation) = animations.get_mut(&card.id) {
                        if animation.zoom.value() {
                            animation.zoom.go_mut(false);

                            return Task::none();
                        }
                    }
                }

                self.state = State::Idle;

                Task::none()
            }
            Message::Tick(now) => {
                self.now = now;

                Task::none()
            }
            Message::ImageFetched(card, Err(error)) => {
                log::error!("{error}");

                let _ = self.images.insert(card, Image::Errored);

                Task::none()
            }
            Message::CollectionSaved(Err(error)) => {
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
                self.page(pair.binder, content, collection, database)
            }
        };

        let right_page = match pair.right {
            binder::Surface::Cover => horizontal_space().into(),
            binder::Surface::Content(content) => {
                self.page(pair.binder, content, collection, database)
            }
        };

        let content = column![header, row![left_page, right_page].spacing(20)]
            .spacing(10)
            .padding(10);

        let overlay: Option<Element<'_, Message>> = match &self.state {
            State::Idle => None,
            State::Adding {
                search,
                matches,
                animations,
                ..
            } => Some(self.adding(search, matches, animations, collection)),
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
                            self.now,
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
        search: &'a str,
        matches: &'a [Card],
        animations: &'a HashMap<card::Id, AnimationSet>,
        collection: &'a Collection,
    ) -> Element<'a, Message> {
        let input = container(
            text_input("Search for your card...", search)
                .on_input(Message::SearchChanged)
                .padding(10)
                .id("search"),
        )
        .max_width(600);

        let content: Element<_> = {
            // TODO: Infinite scrolling (?)
            let matches: Element<_> = if !search.is_empty() && matches.is_empty() {
                center(
                    container(text!("No cards were found matching: \"{search}\" :/"))
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
                                self.now,
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

    pub fn subscription(&self) -> Subscription<Message> {
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
                    .any(|animation| animation.is_animating(self.now))
            };

            let is_animating = if let State::Adding { animations, .. } = &self.state {
                is_animating(&self.animations) || is_animating(animations)
            } else {
                is_animating(&self.animations)
            };

            if is_animating {
                window::frames().map(Message::Tick)
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

            let card = mouse_area(
                button(
                    image(handle)
                        .width(Fill)
                        .height(Fill)
                        .content_fit(ContentFit::Cover)
                        .opacity(opacity)
                        .scale(scale)
                        .translate(move |bounds, viewport| {
                            let scale = source.zoom();
                            let final_bounds = bounds.zoom(scale);

                            final_bounds.offset(&viewport.shrink(10)) * shadow
                        })
                        .style(move |_theme| image::Style {
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
        _ => slot(horizontal_space()).into(),
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
    fn new() -> Self {
        Self {
            fade_in: Animation::new(false)
                .easing(animation::Easing::EaseInOut)
                .slow()
                .go(true),
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
