// Clean minimal entrypoint (legacy code moved into library modules)
use dungeon_core::{inventory::{Inventory, SAVE_FILE}, ui::{prompt_main_action, MainAction}, actions::{pick_pocket, visit_shop, fight_monster, visit_tavern}};
use std::fs;
use std::env;

const LOOT_FILE: &str = "loot.json";

fn load_loot_items() -> Vec<String> {
    fs::read_to_string(LOOT_FILE)
        .ok()
        .and_then(|d| serde_json::from_str(&d).ok())
        .unwrap_or_else(|| {
            println!("⚠️  Failed to load loot.json. Using default items.");
            vec!["Gold Coin".into(), "Silver Ring".into(), "Rusty Dagger".into(), "Health Potion".into()]
        })
}

fn print_version_and_exit() {
    // Version comes from Cargo.toml via env! macro at compile time
    println!("dungeon v{}", env!("CARGO_PKG_VERSION"));
}

fn handle_reset_flag() {
    if std::fs::remove_file(SAVE_FILE).is_ok() {
        println!("Inventory reset ({} removed)", SAVE_FILE);
    } else {
        println!("No existing inventory to reset ({} not found)", SAVE_FILE);
    }
}

fn print_help_and_exit() {
    println!("Usage: dungeon [OPTIONS]\n\nOptions:\n  -v, --version    Show version and exit\n  -r, --reset      Reset inventory (delete {save})\n  -h, --help       Show this help and exit\n\nShort flags can be clustered, e.g. -rv.", save = SAVE_FILE);
}

fn main() {
    // Lightweight manual flag parsing (keep dependencies minimal)
    let mut args: Vec<String> = env::args().skip(1).collect();
    if !args.is_empty() {
        // Support combined short flags like -vr (order independent)
    let mut did_action = false;
        let mut remaining: Vec<String> = Vec::new();
        for a in args.drain(..) {
            match a.as_str() {
        "-v" | "--version" => { print_version_and_exit(); return; }
        "-r" | "--reset" => { handle_reset_flag(); did_action = true; }
        "-h" | "--help" => { print_help_and_exit(); return; }
                _ if a.starts_with('-') && !a.starts_with("--") && a.len() > 2 => {
                    // Split clustered short flags like -vr
                    for ch in a.chars().skip(1) { match ch {
                        'v' => { print_version_and_exit(); return; }
                        'r' => { handle_reset_flag(); did_action = true; }
            'h' => { print_help_and_exit(); return; }
                        other => { println!("Ignoring unknown short flag -{}", other); }
                    }}
                }
                _ => remaining.push(a)
            }
        }
        args = remaining;
        // If we only performed a reset action and nothing else, exit early
        if did_action && args.is_empty() { return; }
    }
    let loot_items = load_loot_items();
    println!("Loaded {} loot items from {}", loot_items.len(), LOOT_FILE);
    let mut inventory = Inventory::load().unwrap_or_else(|_| { println!("No existing inventory found, starting fresh!"); Inventory::new() });
    loop {
        match prompt_main_action() {
            MainAction::PickPocket => pick_pocket(&mut inventory, &loot_items),
            MainAction::Inventory => inventory.show(),
            MainAction::Shop => visit_shop(&mut inventory),
            MainAction::Fight => fight_monster(&mut inventory),
            MainAction::Tavern => visit_tavern(&mut inventory),
            MainAction::Exit => {
                if let Err(e) = inventory.save() { println!("Failed to save inventory: {}", e); } else { println!("Inventory saved!"); }
                println!("Exiting");
                break;
            }
        }
    }
}


