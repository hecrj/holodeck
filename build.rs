pub fn main() {
    let name = env!("CARGO_PKG_NAME");

    println!("cargo::rerun-if-changed=fonts/{name}-icons.toml");
    iced_fontello::build(format!("fonts/{name}-icons.toml")).expect("Build icons font");
}
