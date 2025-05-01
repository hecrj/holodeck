<div align="center">

# Holodeck

[![Crates.io](https://img.shields.io/crates/v/holodeck.svg)](https://crates.io/crates/holodeck)
[![License](https://img.shields.io/crates/l/holodeck.svg)](https://github.com/hecrj/holodeck/blob/master/LICENSE)
[![Downloads](https://img.shields.io/crates/d/holodeck.svg)](https://crates.io/crates/holodeck)
[![Test Status](https://img.shields.io/github/actions/workflow/status/hecrj/holodeck/test.yml?branch=master&event=push&label=test)](https://github.com/hecrj/holodeck/actions)
[![Made with iced](https://iced.rs/badge.svg)](https://github.com/iced-rs/iced)

An application to track, manage, and visualize your TCG collection — Pokémon only for now!

<img alt="Holodeck - Binder" src="assets/binder.webp" width="49%">
<img alt="Holodeck - Adding" src="assets/mew.webp" width="49%">

</div>

## Features

- Extensive embedded database (~29,000 cards)
- Binder view for easy IRL collection
- Price tracking
- Multiple profiles
- Minimum remote API usage for maximum speed
- Cool animations!
- Powered by [PokemonTCG] and [TCGdex]
- Very experimental, very work in progress!
- ... more to come!

[PokemonTCG]: https://pokemontcg.io
[TCGdex]: https://tcgdex.dev


## Installation

No pre-built binaries yet! Use `cargo` to try it out:

```bash
cargo install --git https://github.com/hecrj/holodeck.git
```

If you want the highest quality images, it is also recommended that you get an API key
from [PokemonTCG] and place it inside an env variable named `POKEMONTCG_API_KEY`.


## Disclaimer

This application is not affiliated with, endorsed, sponsored, or approved by Nintendo, Game Freak, The Pokémon Company, or any other official TCG publisher.

All trademarks and copyrighted materials are the property of their respective owners.
