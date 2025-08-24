use rand::prelude::IndexedRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;

const MENU_OPTIONS: &[&str] = &["PickPocket", "Show Inventory", "Exit program"];
const LOOT_ITEMS: &[&str] = &["Gold Coin", "Silver Ring", "Rusty Dagger", "Health Potion"];
const SAVE_FILE: &str = "inventory.json";

fn main() {
    let mut inventory = Inventory::load().unwrap_or_else(|_| {
        println!("No existing inventory found, starting fresh!");
        Inventory::new()
    });

    loop {
        let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Choose an option")
            .items(MENU_OPTIONS)
            .default(0)
            .interact()
            .unwrap();

        match selection {
            0 => pick_pocket(&mut inventory),
            1 => inventory.show(),
            2 => {
                if let Err(e) = inventory.save() {
                    println!("Failed to save inventory: {}", e);
                } else {
                    println!("Inventory saved!");
                }
                println!("Exiting");
                break;
            },
            _ => unreachable!(),
        }
    }
}

fn pick_pocket(inventory: &mut Inventory) {
    let mut rng = rand::rng();
    let success = rng.random_bool(0.5); // 50% chance of success
    
    if success {
        if let Some(item) = LOOT_ITEMS.choose(&mut rng) {
            inventory.add(item);
        }
    } else {
        println!("🚨 Caught! The NPC noticed you and calls for guards!");
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Inventory {
    items: Vec<String>,
}

impl Inventory {
    fn new() -> Self {
        Inventory { items: Vec::new() }
    }

    fn add(&mut self, item: &str) {
        self.items.push(item.to_string());
        println!("👜 You stole: {}", item);
        
        // Auto-save after adding item
        if let Err(e) = self.save() {
            println!("⚠️  Failed to save inventory: {}", e);
        }
    }

    fn show(&self) {
        if self.items.is_empty() {
            println!("Your inventory is empty.");
        } else {
            println!("Your inventory contains:");
            for item in &self.items {
                println!("• {}", item);
            }
        }
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(SAVE_FILE, json)?;
        Ok(())
    }
    
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let data = fs::read_to_string(SAVE_FILE)?;
        let inventory = serde_json::from_str(&data)?;
        Ok(inventory)
    }
}


