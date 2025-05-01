use crate::Collection;
use crate::pokebase::card;
use crate::pokebase::{Card, Database};

use std::fmt;
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
pub struct Binder {
    pub columns: usize,
    pub rows: usize,
    pub pages: usize,
}

impl Binder {
    pub fn cards_per_page(self) -> usize {
        self.columns * self.rows
    }

    pub fn capacity(self) -> usize {
        self.cards_per_page() * self.pages
    }
}

pub struct Set {
    binders: Vec<Binder>,
}

impl Set {
    pub fn new() -> Self {
        Self {
            binders: vec![
                Binder {
                    columns: 4,
                    rows: 3,
                    pages: 52,
                },
                Binder {
                    columns: 4,
                    rows: 3,
                    pages: 52,
                },
            ],
        }
    }

    pub fn total_pages(&self, mut total_cards: usize) -> Page {
        let mut pages = 0;

        for binder in &self.binders {
            let needed_pages = total_cards / binder.cards_per_page();

            if needed_pages < binder.pages {
                pages += needed_pages.max(1);
                break;
            } else {
                pages += binder.pages;
                total_cards -= binder.capacity();
            }
        }

        Page(pages)
    }

    pub fn place(&self, mut position: usize) -> Page {
        let mut page = 0;

        for binder in &self.binders {
            if position < binder.capacity() {
                return Page(page + position / binder.cards_per_page());
            } else {
                position -= binder.capacity();
                page += binder.pages;
            }
        }

        Page(page)
    }

    pub fn spread(&self, page: Page) -> Spread {
        let Page(page) = page;

        let mut pages = 0;

        let covers = self
            .binders
            .iter()
            .take_while(|binder| {
                pages += binder.pages;
                pages < page
            })
            .count()
            * 2
            + 1;

        Spread((page + covers) / 2)
    }

    pub fn open(&self, spread: Spread) -> Option<Pair> {
        let Spread(spread) = spread;
        let mut page = spread * 2;
        let mut offset = 0;

        for (i, binder) in self.binders.iter().enumerate() {
            // Remove cover
            page = page.saturating_sub(1);

            if page < binder.pages {
                let offset = offset + page * binder.cards_per_page();

                return Some(Pair {
                    binder: *binder,
                    binder_number: i,
                    left: if page == 0 {
                        Surface::Cover
                    } else {
                        Surface::Content(Content {
                            page: Page(page),
                            range: (offset..offset + binder.cards_per_page()),
                        })
                    },
                    right: if page + 1 == binder.pages {
                        Surface::Cover
                    } else if page == 0 {
                        Surface::Content(Content {
                            page: Page(page),
                            range: (offset..offset + binder.cards_per_page()),
                        })
                    } else {
                        Surface::Content(Content {
                            page: Page(page + 1),
                            range: (offset + binder.cards_per_page()
                                ..offset + 2 * binder.cards_per_page()),
                        })
                    },
                });
            }

            page -= binder.pages + 1; // Remove back
            offset += binder.capacity();
        }

        None
    }

    pub fn len(&self) -> usize {
        self.binders.len()
    }
}

pub struct Pair {
    pub binder: Binder,
    pub binder_number: usize,
    pub left: Surface,
    pub right: Surface,
}

#[derive(Debug, Clone)]
pub enum Surface {
    Cover,
    Content(Content),
}

#[derive(Debug, Clone)]
pub struct Content {
    pub page: Page,
    pub range: Range<usize>,
}

impl Default for Set {
    fn default() -> Self {
        Set::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    GottaCatchEmAll,
}

impl Mode {
    pub const ALL: &[Self] = &[Self::GottaCatchEmAll];

    pub fn total_cards(self, database: &Database) -> usize {
        match self {
            Self::GottaCatchEmAll => database.pokemon.len(),
        }
    }

    pub fn progress(self, collection: &Collection, database: &Database) -> f32 {
        match self {
            Self::GottaCatchEmAll => {
                collection.total_pokemon(database) as f32 / database.pokemon.len() as f32 * 100.0
            }
        }
    }

    pub fn card<'a>(
        self,
        index: usize,
        collection: &Collection,
        database: &'a Database,
    ) -> Option<&'a Card> {
        match self {
            Mode::GottaCatchEmAll => {
                let pokemon = database.pokemon.values().get(index)?;

                let mut cards: Vec<_> = collection
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
                    .collect();

                cards.sort_unstable_by(|a, b| a.rarity.cmp(&b.rarity).reverse());
                cards.first().copied()
            }
        }
    }

    pub fn position(self, card: &card::Id, database: &Database) -> Option<usize> {
        match self {
            Mode::GottaCatchEmAll => {
                let card = database.cards.get(card)?;

                card.pokedex
                    .first()
                    .copied()
                    .map(|pokemon| pokemon.number() - 1)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Page(usize);

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0 + 1).fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Spread(usize);

impl Spread {
    pub fn increment(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn decrement(self) -> Option<Self> {
        if self.0 == 0 {
            return None;
        }

        Some(Self(self.0 - 1))
    }
}
