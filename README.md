# Dungeon

Rust mini game (inventory / gold management + lightweight RPG flavor) runnable as an interactive terminal application and as a WebAssembly demo with two UI themes (Fantasy & Retro BBS).

## Table of Contents

- [Gameplay Systems](#gameplay-systems)
- [Web UI](#web-ui)
- [Image Assets (SVG + PNG)](#image-assets-svg--png)
- [CLI Usage](#cli-usage)
- [Feature Flags](#feature-flags)
- [WASM Development](#wasm-development-local)
- [WASM API](#wasm-api-current)
- [Testing & Linting](#testing--linting)
- [GitHub Pages Deployment](#github-pages-deployment)
- [Directory Overview](#directory-overview)
- [Planned / Possible Enhancements](#planned--possible-enhancements)
- [License](#license)
- [Contribution](#contribution)

## Gameplay Systems

Inventory‑centric loop with several lightweight actions:

- **Pickpocket**: Auto‑generates candidate loot each attempt. Stored "luck" can trigger a special windfall event.
- **Fight**: Random monster encounter; victory grants gold, defeat risks a percentage loss (never below 1 gp if you have any).
- **Tavern**: Drink, food, stay, tip, or flirt actions trade coin for small benefits and potential to store a single luck boost.
- **Shop**: Procedurally generated stock with rarity tiers; optional haggling (success reduces total, failure adds a penalty); luck can improve haggle chances.
- **Luck**: Binary stored flag that amplifies certain outcomes (pickpocket event chance, haggle bonus) and is consumed on use.

## Web UI

Located in `web/` and powered by the `wasm` feature + `wasm-bindgen` glue. Key traits:

- Theme toggle (button labeled THEME) switches between:
  - **Fantasy**: Styled panels, scene image (pickpocket / fight / tavern) swapping based on latest action.
  - **BBS**: Monospace, ASCII panel headers, log‑centric retro feel.
- Live inventory & currency (gp / sp / cp) display with automatic denomination management.
- Luck status indicator (READY / NONE).
- Shop table with selectable items + haggle & spend‑luck options.
- Rolling timestamped log of actions.

## Image Assets (SVG + PNG)

Scene images default to the SVG files (`web/assets/scene_*.svg`). If you add corresponding PNGs (e.g. `scene_pickpocket.png`) the runtime attempts to load the PNG first and silently falls back to the SVG if the PNG is missing. This allows higher‑fidelity raster artwork while keeping lightweight vector defaults.

## CLI Usage

```bash
cargo run --features cli
```

Flags (short may be clustered, e.g. `-rv`):

- `-v` / `--version` – Print version and exit
- `-r` / `--reset`   – Reset stored inventory
- `-h` / `--help`    – Help text

## Feature Flags

- `cli` (default): Enables dialoguer based terminal UI & related prompts.
- `wasm`: Exposes `wasm_api` (no terminal prompts) for browser build.

## WASM Development (Local)

1. Add target (once):

   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. Build release + bindgen (matching the CI pattern):

   ```bash
   cargo build --release --target wasm32-unknown-unknown --features wasm --lib
   wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/release/dungeon_core.wasm
   ```

3. (Optional) Add a simple version stamp:

   ```bash
   echo "build_sha=$(git rev-parse --short HEAD)" > web/pkg/version.txt
   ```

4. Serve locally:

   ```bash
   python3 -m http.server -d web 8000
   ```

   Open: <http://localhost:8000>

## WASM API (Current)

`Game` constructor + methods (all return a JSON object containing the new state and a message unless otherwise noted):

- `get_state()` – Current inventory snapshot (gp / sp / cp / items / luck)
- `add_loot(desc: &str)` – Parse a human readable loot string into currency/items
- `apply_penalty(percent: u32)` – Apply a percentage gold loss (minimum 1 gp if positive gold exists)
- `pickpocket(candidates: &str)` – Attempt; empty string auto‑generates candidates; may consume luck
- `fight()` – Run a monster encounter
- `reset()` – Reset inventory & shop state
- `generate_shop()` – Produce a new shop stock (rarity + price ranges) and persist it
- `shop_purchase(indices: Vec<u32>, attempt_haggle: bool, spend_luck: bool)` – Buy selected items by id
- `tavern(action: &str)` – Perform tavern actions: `drink|food|stay|tip|flirt`

## Testing & Linting

Strict settings in CI (GitHub Actions workflow) enforce:

- `cargo test --features cli`
- `cargo clippy --all-targets --all-features -- -D warnings`

Formatting:

- `cargo fmt -- --check` for verification
- `cargo fmt` to apply formatting

## GitHub Pages Deployment

Workflow: `./github/workflows/gh-pages.yml`:

- Caches Cargo, runs tests & clippy
- Builds release wasm + matches `wasm-bindgen-cli` version to the lockfile
- Uploads `web/` as a Pages artifact and deploys (includes `version.txt`)

Published site (once enabled): <https://octopusnz.github.io/dungeon/> (adjust if forked).

## Directory Overview

- `src/` – Core logic (inventory, actions, wasm API wrapper, RNG helpers, terminal UI prompts)
- `web/` – Browser assets, index HTML, generated `pkg/` (post bindgen), SVG scene images (PNG overrides optional)
- `tests/` – Unit & scenario tests (penalties, edge cases, flows)
- `inventory.json` / `loot.json` – Sample data / starting loot list

## Planned / Possible Enhancements

- Animated scene transitions & more monster‑specific art
- Local storage persistence of inventory (currently only theme preference)
- Accessibility audit (ARIA labels for buttons & log live region)
- Optional wasm binary size optimization (`wasm-opt`)

## License

Dual licensed under either:

- [MIT License](LICENSE-MIT) (if present) or standard MIT text
- [Apache License, Version 2.0](LICENSE-APACHE) (if present) or standard Apache 2.0 text

You may choose either license.

## Contribution

Issues / PRs welcome. Keep clippy clean (no warnings) and include a minimal test when changing core logic.
