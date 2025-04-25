use std::path::Path;

pub fn main() {
    println!("cargo::rerun-if-changed=data/cards.ron");
    println!("cargo::rerun-if-changed=data/pokemon.ron");
    println!("cargo::rerun-if-changed=data/series.ron");
    println!("cargo::rerun-if-changed=data/sets.ron");

    compress("data/cards.ron");
    compress("data/pokemon.ron");
    compress("data/series.ron");
    compress("data/sets.ron");
}

fn compress(path: impl AsRef<Path>) {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::fs::{File, exists};
    use std::io::{BufReader, BufWriter, copy};

    let path = path.as_ref();

    if !exists(path).unwrap_or(false) {
        return;
    }

    let mut origin = BufReader::new(File::open(path).expect("Read file"));

    let mut encoder = {
        let destination =
            BufWriter::new(File::create(path.with_extension("ron.gz")).expect("Create file"));

        GzEncoder::new(destination, Compression::default())
    };

    copy(&mut origin, &mut encoder).expect("Compress file");

    let _ = encoder.finish().expect("Finish encoding");
}
