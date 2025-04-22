use pokebase::Database;

use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), anywho::Error> {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/raw/tcgdex/server/generated");
    let database = Database::generate(data)?;

    fs::write(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/series.ron"),
        ron::ser::to_string_pretty(database.series.values(), ron::ser::PrettyConfig::default())?,
    )?;

    fs::write(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/sets.ron"),
        ron::ser::to_string_pretty(database.sets.values(), ron::ser::PrettyConfig::default())?,
    )?;

    fs::write(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/cards.ron"),
        ron::ser::to_string_pretty(database.cards.values(), ron::ser::PrettyConfig::default())?,
    )?;

    Ok(())
}
