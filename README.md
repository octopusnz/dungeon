Dungeon
=======

Terminal roguelite mini-game with pickpocketing, shop, tavern & WASM demo.

CLI Run
-------
`cargo run --features cli`

WASM Dev (Local)
----------------
1. Add target: `rustup target add wasm32-unknown-unknown`
2. Build: `cargo build --target wasm32-unknown-unknown --features wasm`
3. Bindgen (after installing `wasm-bindgen-cli`):
   `wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/debug/dungeon.wasm`
4. Serve `web/` directory (e.g. `python3 -m http.server -d web`).

GitHub Pages
------------
Workflow `.github/workflows/gh-pages.yml` builds the wasm artifact on pushes to `master` and publishes `web/` to Pages.

Features Flags
--------------
- `cli` (default): Enables interactive terminal (dialoguer).
- `wasm`: Exposes `wasm_api` module for browser usage.

WASM API (Minimal)
------------------
Constructor `Game::new()` and methods:
- `get_state()` -> JSON of inventory
- `add_loot(desc)` parse and add loot
- `apply_penalty(percent)` apply gold loss percent

License
-------
MIT OR Apache-2.0