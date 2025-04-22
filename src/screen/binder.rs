use crate::Collection;
use crate::icon;
use crate::pokebase::Database;
use crate::widget::pokeball;

use iced::keyboard;
use iced::widget::{center, column, container, grid, horizontal_space, pick_list, row, text};
use iced::{Center, Element, Fill, Font, Subscription, Task};

use std::fmt;

pub struct Binder {
    binders: Set,
    page: usize,
    mode: Mode,
}

#[derive(Debug, Clone)]
pub enum Message {
    ModeSelected(Mode),
    PreviousPage,
    NextPage,
}

impl Binder {
    pub fn new() -> Self {
        Self {
            binders: Set::default(),
            page: 0,
            mode: Mode::GottaCatchEmAll,
        }
    }

    pub fn update(&mut self, message: Message, _collection: &mut Collection) -> Task<Message> {
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
        }
    }

    pub fn view<'a>(
        &'a self,
        collection: &'a Collection,
        database: &Database,
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
                row![icon.into(), text(content).font(Font::MONOSPACE).size(12)]
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
                .text_size(12)
                .font(Font::MONOSPACE);

            row![
                mode,
                pokemon,
                horizontal_space(),
                row![binders, pages].spacing(30).align_y(Center)
            ]
            .align_y(Center)
            .spacing(20)
        };

        let left_page: Element<_> = if relative_page > 1 {
            grid(
                (offset..(offset + unit.cards_per_page()))
                    .map(|i| card(i, self.mode, collection, database)),
            )
            .columns(unit.columns)
            .height(Fill)
            .spacing(10)
            .into()
        } else {
            center(
                column![
                    text!("{name}'s\nCollection", name = collection.name.as_str())
                        .font(Font::MONOSPACE)
                        .size(40)
                        .center(),
                    text!("#{}", number + 1).size(20).font(Font::MONOSPACE)
                ]
                .spacing(10)
                .align_x(Center),
            )
            .into()
        };

        let right_page: Element<_> = if relative_page < unit.pages {
            grid(
                (offset + unit.cards_per_page()..(offset + unit.cards_per_page() * 2))
                    .map(|i| card(i, self.mode, collection, database)),
            )
            .columns(unit.columns)
            .height(Fill)
            .spacing(10)
            .into()
        } else {
            horizontal_space().into()
        };

        column![header, row![left_page, right_page].spacing(30)]
            .spacing(10)
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

fn card<'a>(
    index: usize,
    mode: Mode,
    collection: &'a Collection,
    database: &Database,
) -> Element<'a, Message> {
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
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Mode::GottaCatchEmAll => "Gotta Catch 'Em All",
        })
    }
}
