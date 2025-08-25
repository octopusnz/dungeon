// Clean minimal entrypoint (legacy code moved into library modules)
use dungeon::{inventory::Inventory, ui::{prompt_main_action, MainAction}, actions::{pick_pocket, visit_shop, fight_monster, visit_tavern}};
use std::fs;

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

fn main() {
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


