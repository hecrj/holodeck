pub fn main() {
    println!("cargo::rerun-if-changed=fonts/pokedeck-icons.toml");
    iced_fontello::build("fonts/pokedeck-icons.toml").expect("Build pokedeck-icons font");
}
