use rand::prelude::IndexedRandom;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;

const MENU_OPTIONS: &[&str] = &["PickPocket", "Show Inventory", "Exit program"];
const SAVE_FILE: &str = "inventory.json";
const LOOT_FILE: &str = "loot.json";

fn load_loot_items() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(LOOT_FILE)?;
    let items: Vec<String> = serde_json::from_str(&data)?;
    Ok(items)
}

fn parse_loot_into_items(loot_description: &str) -> Vec<String> {
    let mut items = Vec::new();
    
    // Compile regex once outside the loop for efficiency
    let standalone_money_check = Regex::new(r"^\d+\s*(gp|sp|cp)$").unwrap();
    
    // Split by " and " and commas first, then handle parenthetical prices per item
    let mut all_parts = Vec::new();
    
    // First split by " and "
    for and_part in loot_description.split(" and ") {
        // Then split each part by commas
        for comma_part in and_part.split(',') {
            all_parts.push(comma_part.trim());
        }
    }
    
    for part in all_parts {
        if part.is_empty() {
            continue;
        }
        
        let item = part
            .trim()
            .trim_start_matches("A ")
            .trim_start_matches("a ")
            .trim_start_matches("An ")
            .trim_start_matches("an ")
            .trim_start_matches("some ")
            .trim_start_matches("Some ")
            .trim_start_matches("several ")
            .trim_start_matches("Several ")
            .trim()
            .to_string();
        
        // Check if this is a standalone monetary value (should be kept as loot)
        let is_standalone_money = standalone_money_check.is_match(&item);
        
        // Skip items that end with currency but aren't standalone money and don't have parentheses
        // Keep standalone money, items with parenthetical prices, and non-monetary items
        if !item.is_empty() && 
           (is_standalone_money || 
            item.contains('(') ||  // Keep items with parenthetical prices
            (!item.ends_with(" gp") && !item.ends_with(" sp") && !item.ends_with(" cp"))) {
            items.push(capitalize_first_letter(&item));
        }
    }
    
    // If no items were parsed, return the original description
    if items.is_empty() {
        items.push(loot_description.to_string());
    }
    
    items
}

fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn add_article(item: &str) -> String {
    let vowels = ['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U'];
    let first_char = item.chars().next().unwrap_or('a');
    
    let article = if vowels.contains(&first_char) { "an" } else { "a" };
    format!("{} {}", article, item)
}

fn format_items_for_display(items: &[String]) -> String {
    let currency_regex = Regex::new(r"^(\d+)\s*(cp|sp|gp)$").unwrap();
    
    let formatted_items: Vec<String> = items.iter().map(|item| {
        if let Some(captures) = currency_regex.captures(item)
            && let (Some(amount_str), Some(currency_str)) = (captures.get(1), captures.get(2)) {
                let amount = amount_str.as_str();
                let expanded_currency = match currency_str.as_str() {
                    "cp" => if amount == "1" { "copper piece" } else { "copper pieces" },
                    "sp" => if amount == "1" { "silver piece" } else { "silver pieces" },
                    "gp" => if amount == "1" { "gold piece" } else { "gold pieces" },
                    _ => return add_article(item),
                };
                return format!("{} {}", amount, expanded_currency);
            }
        add_article(item)
    }).collect();
    
    // Join items with proper grammar
    match formatted_items.len() {
        0 => String::new(),
        1 => formatted_items[0].clone(),
        2 => format!("{} and {}", formatted_items[0], formatted_items[1]),
        _ => {
            let (last, rest) = formatted_items.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

fn main() {
    // Load loot items from JSON file
    let loot_items = load_loot_items().unwrap_or_else(|e| {
        println!("⚠️  Failed to load loot.json: {}. Using default items.", e);
        vec!["Gold Coin".to_string(), "Silver Ring".to_string(), "Rusty Dagger".to_string(), "Health Potion".to_string()]
    });
    
    println!("Loaded {} loot items from {}", loot_items.len(), LOOT_FILE);

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
            0 => pick_pocket(&mut inventory, &loot_items),
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

fn pick_pocket(inventory: &mut Inventory, loot_items: &[String]) {
    let mut rng = rand::rng();
    let random_event = rng.random_range(0.0..1.0);
    
    // 5% chance for mysterious figure event
    if random_event < 0.05 {
        println!("🌟 A mysterious figure emerges from the shadows...");
        println!("👤 \"I have been watching you, young thief. Your boldness amuses me.\"");
        println!("💰 The figure tosses you a heavy pouch and vanishes into the darkness!");
        println!("✨ You found: 1000 gold pieces");
        
        inventory.add_item("1000 gp");
        inventory.save_after_pickup();
    } else {
        // Normal pickpocket logic with 50% success rate
        let success = rng.random_bool(0.5);
        
        if success {
            if let Some(loot_description) = loot_items.choose(&mut rng) {
                let individual_items = parse_loot_into_items(loot_description);
                let formatted_display = format_items_for_display(&individual_items);
                println!("👜 You found: {}", formatted_display);
                
                for item in individual_items {
                    inventory.add_item(&item);
                }
                
                inventory.save_after_pickup();
            }
        } else {
            println!("🚨 Caught! The NPC noticed you and calls for guards!");
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Inventory {
    items: Vec<String>,
    copper_pieces: u32,
    silver_pieces: u32,
    gold_pieces: u32,
}

impl Inventory {
    fn new() -> Self {
        Inventory { 
            items: Vec::new(),
            copper_pieces: 0,
            silver_pieces: 0,
            gold_pieces: 0,
        }
    }

    fn add_item(&mut self, item: &str) {
        // Check if this is a monetary item
        if let Some(amount) = self.parse_currency(item) {
            match amount.1.as_str() {
                "cp" => self.copper_pieces += amount.0,
                "sp" => self.silver_pieces += amount.0,
                "gp" => self.gold_pieces += amount.0,
                _ => self.items.push(item.to_string()), // fallback for unknown currency
            }
        } else {
            self.items.push(item.to_string());
        }
    }
    
    fn parse_currency(&self, item: &str) -> Option<(u32, String)> {
        let currency_regex = Regex::new(r"^(\d+)\s*(cp|sp|gp)$").unwrap();
        if let Some(captures) = currency_regex.captures(item)
            && let (Some(amount_str), Some(currency_str)) = (captures.get(1), captures.get(2))
            && let Ok(amount) = amount_str.as_str().parse::<u32>() {
                return Some((amount, currency_str.as_str().to_string()));
            }
        None
    }
    
    fn save_after_pickup(&mut self) {
        // Auto-save after adding items
        if let Err(e) = self.save() {
            println!("⚠️  Failed to save inventory: {}", e);
        }
    }

    fn show(&self) {
        let has_items = !self.items.is_empty();
        let has_currency = self.copper_pieces > 0 || self.silver_pieces > 0 || self.gold_pieces > 0;
        
        if !has_items && !has_currency {
            println!("Your inventory is empty.");
            return;
        }
        
        println!("Your inventory contains:");
        
        // Show currency first
        if has_currency {
            println!("💰 Currency:");
            if self.gold_pieces > 0 {
                println!("  • {} gold pieces", self.gold_pieces);
            }
            if self.silver_pieces > 0 {
                println!("  • {} silver pieces", self.silver_pieces);
            }
            if self.copper_pieces > 0 {
                println!("  • {} copper pieces", self.copper_pieces);
            }
        }
        
        // Show items
        if has_items {
            if has_currency {
                println!("🎒 Items:");
            }
            for item in &self.items {
                println!("  • {}", item);
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


