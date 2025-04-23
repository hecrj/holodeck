use crate::Collection;
use crate::card;
use crate::icon;
use crate::pokebase::database;
use crate::pokebase::pokemon;
use crate::pokebase::{Card, Database};
use crate::widget::pokeball;

use iced::animation;
use iced::border;
use iced::keyboard;
use iced::task;
use iced::time::Instant;
use iced::widget::{
    bottom_right, button, center, center_x, center_y, column, container, grid, horizontal_space,
    hover, image, mouse_area, opaque, pick_list, pop, row, scrollable, stack, text, text_input,
};
use iced::window;
use iced::{Animation, Center, Color, ContentFit, Element, Fill, Shrink, Subscription, Task};

use function::Binary;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

pub struct Binder {
    binders: Set,
    page: usize,
    mode: Mode,
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
    ModeSelected(Mode),
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
    Tick(Instant),
}

#[derive(Debug, Clone, Copy)]
pub enum Source {
    Binder,
    Search,
}

impl Binder {
    pub fn new() -> Self {
        Self {
            binders: Set::default(),
            page: 0,
            mode: Mode::GottaCatchEmAll,
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

                let new_page = self.page.saturating_sub(2);

                if new_page != self.page {
                    self.page = new_page;
                    self.animations.clear();
                }

                Task::none()
            }
            Message::NextPage => {
                let State::Idle = self.state else {
                    return Task::none();
                };

                let new_page = (self.page + 2)
                    .min(self.binders.pages_needed(self.mode.total_cards(database)) - 1);

                if new_page != self.page {
                    self.page = new_page;
                    self.animations.clear();
                }

                Task::none()
            }
            Message::Add => {
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

                let (search_cards, handle) =
                    Task::perform(database.search_cards(&new_search), Message::SearchFinished)
                        .abortable();

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
                Source::Search => {
                    self.state = State::Idle;

                    if let Some(position) = self.mode.position(&card, database) {
                        self.page = self.binders.page(position);

                        if self.page % 2 != 0 {
                            self.page = self.page.saturating_sub(1);
                        }
                    }

                    collection.add(card);

                    Task::perform(collection.save(), Message::CollectionSaved).discard()
                }
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

    pub fn view<'a>(
        &'a self,
        collection: &'a Collection,
        database: &'a Database,
    ) -> Element<'a, Message> {
        let Some((unit, number, relative_page, offset)) = self.binders.open(self.page) else {
            // TODO
            return center(text("This page does not exist!")).into();
        };

        let header = {
            let total_cards = self.mode.total_cards(database);

            fn stat<'a>(
                icon: impl Into<Element<'a, Message>>,
                content: String,
            ) -> Element<'a, Message> {
                row![icon.into(), text(content).size(12)]
                    .spacing(10)
                    .align_y(Center)
                    .into()
            }

            let pokemon = stat(
                pokeball(12),
                format!(
                    "{owned_pokemon} / {total_pokemon} ({completion:.1}%)",
                    owned_pokemon = collection.total_pokemon(database),
                    total_pokemon = database.pokemon.len(),
                    completion = self.mode.progress(collection, database),
                ),
            );

            let binders = stat(
                icon::book().size(12),
                format!(
                    "{number} / {total_binders}",
                    number = number + 1,
                    total_binders = self.binders.len()
                ),
            );

            let pages = stat(
                icon::binder().size(12),
                format!(
                    "{page} / {pages_needed}",
                    page = self.page + 1,
                    pages_needed = self.binders.pages_needed(total_cards)
                ),
            );

            let mode = pick_list(Mode::ALL, Some(self.mode), Message::ModeSelected)
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
                row![pokemon, binders, pages].spacing(30).align_y(Center)
            ]
            .height(30)
            .align_y(Center)
            .spacing(20)
        };

        let binder_page = |range: std::ops::Range<usize>| {
            center_y(
                grid(range.map(|i| {
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
                        .unwrap_or_else(|| placeholder(i))
                }))
                .columns(unit.columns)
                .height(grid::aspect_ratio(734, 1024))
                .spacing(10),
            )
            .into()
        };

        let left_page: Element<_> = if relative_page > 1 {
            binder_page(offset - unit.cards_per_page()..offset)
        } else {
            center(
                column![
                    text!("{name}'s\nCollection", name = collection.name.as_str())
                        .size(40)
                        .center(),
                    text!("#{}", number + 1).size(20)
                ]
                .spacing(10)
                .align_x(Center),
            )
            .into()
        };

        let right_page: Element<_> = if relative_page < unit.pages {
            binder_page(offset..(offset + unit.cards_per_page()))
        } else {
            horizontal_space().into()
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
            } => {
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
                                item(
                                    card,
                                    self.images.get(&card.id),
                                    animations.get(&card.id),
                                    self.now,
                                    Source::Search,
                                )
                            }))
                            .fluid(300)
                            .height(grid::aspect_ratio(734, 1024))
                            .spacing(10),
                        )
                        .width(Fill)
                        .height(Fill)
                        .spacing(10)
                        .into()
                    };

                    column![center_x(input), matches].spacing(10).into()
                };

                Some(center(content).padding(10).into())
            }
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
                .padding(10)
            }))
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let hotkeys = keyboard::on_key_press(|key, modifiers| {
            use keyboard::key::{Key, Named};

            Some(match key.as_ref() {
                Key::Named(Named::ArrowLeft) if modifiers.is_empty() => Message::PreviousPage,
                Key::Named(Named::ArrowRight) if modifiers.is_empty() => Message::NextPage,
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
            let (opacity, scale) = if let Some(animations) = animations {
                (
                    animations.fade_in.interpolate(0.0, 1.0, now),
                    animations.zoom.interpolate(1.0, 1.1, now),
                )
            } else {
                (0.0, 1.0)
            };

            mouse_area(
                button(
                    image(handle)
                        .width(Fill)
                        .height(Fill)
                        .content_fit(ContentFit::Cover)
                        .opacity(opacity)
                        .scale(scale),
                )
                .on_press_with(move || Message::CardChosen(card.id.clone(), source))
                .padding(0)
                .style(button::text),
            )
            .on_enter(Message::CardHovered(card.id.clone(), source, true))
            .on_exit(Message::CardHovered(card.id.clone(), source, false))
            .into()
        }
        Some(Image::Errored) => container(center(
            text(card.name.get("en").map(String::as_str).unwrap_or("Unknown"))
                .center()
                .size(14),
        ))
        .style(container::dark)
        .into(),
        _ => horizontal_space().into(),
    };

    pop(item)
        .key(card.id.as_str())
        .on_show(move |_size| Message::CardShown(card.id.clone(), source))
        .into()
}

fn placeholder<'a>(index: usize) -> Element<'a, Message> {
    hover(
        container(horizontal_space()).style(|theme| {
            let style = container::dark(theme);

            style.border(border::rounded(6))
        }),
        bottom_right(text!("#{}", index + 1).size(10)).padding(5),
    )
    .into()
}

struct Set {
    units: Vec<Unit>,
}

impl Set {
    pub fn new() -> Self {
        Self {
            units: vec![
                Unit {
                    columns: 4,
                    rows: 3,
                    pages: 52,
                },
                Unit {
                    columns: 4,
                    rows: 3,
                    pages: 52,
                },
            ],
        }
    }

    pub fn pages_needed(&self, mut total_cards: usize) -> usize {
        let mut pages = 0;

        for unit in &self.units {
            let needed_pages = total_cards / unit.cards_per_page();

            if needed_pages < unit.pages {
                pages += needed_pages.max(1);
                break;
            } else {
                pages += unit.pages;
                total_cards -= unit.capacity();
            }
        }

        pages
    }

    pub fn open(&self, mut page: usize) -> Option<(&Unit, usize, usize, usize)> {
        let mut offset = 0;

        for (number, unit) in self.units.iter().enumerate() {
            if page < unit.pages {
                return Some((unit, number, page, offset + (page * unit.cards_per_page())));
            } else {
                offset += unit.capacity();
                page -= unit.pages;
            }
        }

        None
    }

    pub fn page(&self, mut position: usize) -> usize {
        let mut page = 0;

        for unit in &self.units {
            if position < unit.capacity() {
                return page + position / unit.cards_per_page();
            } else {
                position -= unit.capacity();
                page += unit.pages;
            }
        }

        page
    }

    fn len(&self) -> usize {
        self.units.len()
    }
}

impl Default for Set {
    fn default() -> Self {
        Set::new()
    }
}

#[derive(Debug, Clone, Copy)]
struct Unit {
    columns: usize,
    rows: usize,
    pages: usize,
}

impl Unit {
    pub fn cards_per_page(self) -> usize {
        self.columns * self.rows
    }

    pub fn capacity(self) -> usize {
        self.cards_per_page() * self.pages
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    GottaCatchEmAll,
}

impl Mode {
    const ALL: &[Self] = &[Self::GottaCatchEmAll];

    fn total_cards(self, database: &Database) -> usize {
        match self {
            Self::GottaCatchEmAll => database.pokemon.len(),
        }
    }

    fn progress(self, collection: &Collection, database: &Database) -> f32 {
        match self {
            Self::GottaCatchEmAll => {
                collection.total_pokemon(database) as f32 / database.pokemon.len() as f32 * 100.0
            }
        }
    }

    fn card<'a>(
        self,
        index: usize,
        collection: &Collection,
        database: &'a Database,
    ) -> Option<&'a Card> {
        match self {
            Mode::GottaCatchEmAll => {
                let pokemon = database.pokemon.values().get(index)?;

                let card = collection
                    .cards
                    .keys()
                    .filter_map(|card| {
                        let card = database.cards.get(card)?;

                        if card.pokedex.contains(&pokemon.id) {
                            Some(card)
                        } else {
                            None
                        }
                    })
                    .next()?;

                Some(card)
            }
        }
    }

    fn position(self, card: &card::Id, database: &Database) -> Option<usize> {
        match self {
            Mode::GottaCatchEmAll => {
                let card = database.cards.get(&card)?;

                card.pokedex.first().copied().map(pokemon::Id::number)
            }
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Mode::GottaCatchEmAll => "Gotta Catch 'Em All",
        })
    }
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
