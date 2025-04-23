pub fn main() {
    println!("cargo::rerun-if-changed=fonts/holobyte-icons.toml");
    iced_fontello::build("fonts/holobyte-icons.toml").expect("Build pokedeck-icons font");
}
