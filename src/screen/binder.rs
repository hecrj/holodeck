use crate::Collection;
use crate::card;
use crate::icon;
use crate::pokebase::database;
use crate::pokebase::{Card, Database};
use crate::widget::pokeball;

use iced::keyboard;
use iced::task;
use iced::widget::{
    button, center, center_x, center_y, column, container, grid, horizontal_space, image, opaque,
    pick_list, pop, row, scrollable, stack, text, text_input,
};
use iced::{Center, Color, ContentFit, Element, Fill, Shrink, Subscription, Task};

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
    CardShown(card::Id),
    ImageFetched(card::Id, Result<card::Image, anywho::Error>),
}

impl Binder {
    pub fn new() -> Self {
        Self {
            binders: Set::default(),
            page: 0,
            mode: Mode::GottaCatchEmAll,
            state: State::Idle,
            images: HashMap::new(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        _collection: &mut Collection,
        database: &Database,
    ) -> Task<Message> {
        match message {
            Message::ModeSelected(mode) => {
                self.mode = mode;

                Task::none()
            }
            Message::PreviousPage => {
                self.page = self.page.saturating_sub(2);

                Task::none()
            }
            Message::NextPage => {
                self.page = (self.page + 2).min(self.binders.total_pages() - 1);

                Task::none()
            }
            Message::Add => {
                let (search_cards, handle) =
                    Task::perform(database.search_cards(""), Message::SearchFinished).abortable();

                self.state = State::Adding {
                    search: String::new(),
                    matches: Arc::new([]),
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
            Message::CardShown(card) => {
                let Some(card) = database.cards.get(&card) else {
                    return Task::none();
                };

                if self.images.contains_key(&card.id) {
                    return Task::none();
                }

                let _ = self.images.insert(card.id.clone(), Image::Loading);

                Task::perform(
                    card::Image::fetch(card, database),
                    Message::ImageFetched.with(card.id.clone()),
                )
            }
            Message::ImageFetched(card, Ok(image)) => {
                let _ = self.images.insert(
                    card,
                    Image::Loaded(image::Handle::from_rgba(
                        image.width,
                        image.height,
                        image.rgba,
                    )),
                );

                Task::none()
            }
            Message::ImageFetched(card, Err(error)) => {
                log::error!("{error}");

                let _ = self.images.insert(card, Image::Errored);

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
                        .map(|card| item(card, self.images.get(&card.id)))
                        .unwrap_or_else(placeholder)
                }))
                .columns(unit.columns)
                .height(grid::aspect_ratio(734, 1024))
                .spacing(10),
            )
            .into()
        };

        let left_page: Element<_> = if relative_page > 1 {
            binder_page(offset..offset + unit.cards_per_page())
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
            binder_page((offset + unit.cards_per_page())..(offset + 2 * unit.cards_per_page()))
        } else {
            horizontal_space().into()
        };

        let content = column![header, row![left_page, right_page].spacing(20)]
            .spacing(10)
            .padding(10);

        let overlay: Option<Element<'_, Message>> = match &self.state {
            State::Idle => None,
            State::Adding {
                search, matches, ..
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
                            grid(
                                matches
                                    .iter()
                                    .take(100)
                                    .map(|card| item(card, self.images.get(&card.id))),
                            )
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
        keyboard::on_key_press(|key, modifiers| {
            use keyboard::key::{Key, Named};

            Some(match key.as_ref() {
                Key::Named(Named::ArrowLeft) if modifiers.is_empty() => Message::PreviousPage,
                Key::Named(Named::ArrowRight) if modifiers.is_empty() => Message::NextPage,
                _ => None?,
            })
        })
    }
}

fn item<'a>(card: &'a Card, thumbnail: Option<&'a Image>) -> Element<'a, Message> {
    if let Some(Image::Loaded(handle)) = thumbnail {
        image(handle)
            .width(Fill)
            .height(Fill)
            .content_fit(ContentFit::Cover)
            .into()
    } else {
        pop(container(center(
            text(card.name.get("en").map(String::as_str).unwrap_or("Unknown"))
                .center()
                .size(14),
        ))
        .style(container::dark))
        .on_show(|_size| Message::CardShown(card.id.clone()))
        .into()
    }
}

fn placeholder<'a>() -> Element<'a, Message> {
    container(horizontal_space()).style(container::dark).into()
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

    pub fn total_pages(&self) -> usize {
        self.units.iter().map(|unit| unit.pages).sum()
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
                collection.total_pokemon(database) as f32 / database.pokemon.len() as f32
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
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Mode::GottaCatchEmAll => "Gotta Catch 'Em All",
        })
    }
}
