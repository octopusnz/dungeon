use rand::prelude::IndexedRandom;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{OnceLock, Mutex};
use std::collections::HashMap;

const MENU_OPTIONS: &[&str] = &["PickPocket", "Show Inventory", "Visit Shop", "Fight Monster", "Visit Tavern", "Exit program"];
const SAVE_FILE: &str = "inventory.json";
const LOOT_FILE: &str = "loot.json";

// Probabilities (0.0 - 1.0)
const EVENT_CHANCE: f64 = 0.05;      // 5% chance mysterious figure appears
const PICKPOCKET_SUCCESS: f64 = 0.50; // 50% base success rate

// Static regex caches (compiled once)
static RE_STANDALONE_MONEY: OnceLock<Regex> = OnceLock::new();
static RE_CURRENCY: OnceLock<Regex> = OnceLock::new();
type LootCacheEntry = (Vec<String>, String);
static LOOT_CACHE: OnceLock<Mutex<HashMap<String, LootCacheEntry>>> = OnceLock::new();

fn standalone_money_regex() -> &'static Regex {
    RE_STANDALONE_MONEY.get_or_init(|| Regex::new(r"^\d+\s*(gp|sp|cp)$").unwrap())
}

fn currency_regex() -> &'static Regex {
    RE_CURRENCY.get_or_init(|| Regex::new(r"^(\d+)\s*(cp|sp|gp)$").unwrap())
}

fn loot_cache() -> &'static Mutex<HashMap<String, LootCacheEntry>> {
    LOOT_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn load_loot_items() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(LOOT_FILE)?;
    let items: Vec<String> = serde_json::from_str(&data)?;
    Ok(items)
}

fn parse_loot_into_items(loot_description: &str) -> Vec<String> {
    let mut items = Vec::new();

    // Collect split parts (by " and " then commas) without allocating intermediate owned Strings where possible
    let mut parts: Vec<&str> = Vec::new();
    for and_part in loot_description.split(" and ") {
        for comma_part in and_part.split(',') {
            let trimmed = comma_part.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed);
            }
        }
    }

    let money_re = standalone_money_regex();

    for raw in parts {
        // Strip common leading determiners (case sensitive variants)
        let mut s = raw;
        for prefix in [
            "A ", "a ", "An ", "an ", "some ", "Some ", "several ", "Several "
        ] {
            if let Some(rest) = s.strip_prefix(prefix) { s = rest; }
        }
        let item = s.trim();
        if item.is_empty() { continue; }

        let is_money = money_re.is_match(item);
        let ends_with_currency = item.ends_with(" gp") || item.ends_with(" sp") || item.ends_with(" cp");
        if is_money || item.contains('(') || !ends_with_currency {
            items.push(capitalize_first_letter(item));
        }
    }

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
    let cre = currency_regex();
    let formatted_items: Vec<String> = items.iter().map(|item| {
        if let Some(caps) = cre.captures(item)
            && let (Some(amount_str), Some(currency_str)) = (caps.get(1), caps.get(2)) {
                let amount = amount_str.as_str();
                let expanded = match currency_str.as_str() {
                    "cp" => if amount == "1" { "copper piece" } else { "copper pieces" },
                    "sp" => if amount == "1" { "silver piece" } else { "silver pieces" },
                    "gp" => if amount == "1" { "gold piece" } else { "gold pieces" },
                    _ => return add_article(item),
                };
                return format!("{} {}", amount, expanded);
            }
        add_article(item)
    }).collect();

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
            2 => visit_shop(&mut inventory),
            3 => fight_monster(&mut inventory),
            4 => visit_tavern(&mut inventory),
            5 => {
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
    let before = inventory.clone();
    let mut items_added: Vec<String> = Vec::new();
    let mut narrative: Vec<String> = Vec::new();
    let mut title = String::from("Pickpocket");
    let boosted = inventory.luck_boost;
    let event_chance = if boosted { 0.90 } else { EVENT_CHANCE };

    if rng.random_bool(event_chance) {
        title = "Mysterious Figure".into();
        narrative.push("A mysterious figure emerges from the shadows...".into());
        narrative.push("\"I have been watching you, young thief. Your boldness amuses me.\"".into());
        narrative.push("The figure tosses you a heavy pouch and vanishes.".into());
        inventory.add_item("1000 gp");
        items_added.push("1000 gp".into());
    } else if rng.random_bool(PICKPOCKET_SUCCESS) {
        if let Some(loot_description) = loot_items.choose(&mut rng) {
            let (individual_items, formatted_display) = parse_and_format_loot_cached(loot_description);
            title = "Successful Pickpocket".into();
            narrative.push(format!("You found: {}", formatted_display));
            for item in individual_items { inventory.add_item(&item); items_added.push(item); }
        }
    } else {
        title = "Caught Pickpocketing".into();
        narrative.push("The NPC notices you and calls for guards!".into());
    }

    // Luck boost is consumed after one pickpocket attempt (regardless of outcome)
    if boosted {
        inventory.luck_boost = false;
        narrative.push("(Your stored luck dissipates.)".into());
    }

    inventory.save_after_pickup();
    // Filter out pure currency strings so they don't appear twice (currency delta already shown)
    let currency_re = currency_regex();
    let non_currency_items: Vec<String> = items_added.into_iter()
        .filter(|i| !currency_re.is_match(i))
        .collect();
    print_event_summary(&title, &before, inventory, &non_currency_items, &[]);
    for line in narrative { println!("  • {}", line); }
}

// Cached wrapper: returns (parsed_items, formatted_display)
fn parse_and_format_loot_cached(loot_description: &str) -> (Vec<String>, String) {
    // Fast path: check cache
    if let Ok(cache) = loot_cache().lock()
        && let Some((items, formatted)) = cache.get(loot_description) {
        return (items.clone(), formatted.clone());
    }
    // Compute
    let items = parse_loot_into_items(loot_description);
    let formatted = format_items_for_display(&items);
    // Insert
    if let Ok(mut cache) = loot_cache().lock() {
        cache.insert(loot_description.to_string(), (items.clone(), formatted.clone()));
    }
    (items, formatted)
}

#[derive(Clone, Copy)]
struct Monster {
    name: &'static str,
    strength: u8, // 1 (weak) .. 10 (deadly)
}

fn fight_monster(inventory: &mut Inventory) {
    let mut rng = rand::rng();
    // Monster catalog (ordered roughly by strength)
    const MONSTERS: &[Monster] = &[
        Monster { name: "Goblin Sneak", strength: 1 },
        Monster { name: "Cave Rat", strength: 1 },
        Monster { name: "Skeleton Guard", strength: 2 },
        Monster { name: "Orc Marauder", strength: 3 },
        Monster { name: "Ghoul", strength: 4 },
        Monster { name: "Ogre Brute", strength: 5 },
        Monster { name: "Wyvern", strength: 6 },
        Monster { name: "Vampire Stalker", strength: 7 },
        Monster { name: "Stone Golem", strength: 8 },
        Monster { name: "Ancient Lich", strength: 9 },
        Monster { name: "Dragon Wyrm", strength: 10 },
    ];

    let monster = *MONSTERS.choose(&mut rng).unwrap();
    // Success chance decreases with strength (floor at 5%)
    let success_chance = (0.85_f64 - (monster.strength as f64 * 0.07)).max(0.05);
    print_simple_header("Battle");
    println!("⚔️  A wild {} (strength {}) appears!", monster.name, monster.strength);
    println!("🧮 Success chance: {:>2}%", (success_chance * 100.0).round() as u32);

    // Determine outcome
    let success = rng.random_bool(success_chance);
    let before = inventory.clone();
    if success {
        // Reward scales with strength; min 20 gp * (strength / 2), max up to 50 gp * strength capped at 500
        let min_gp = (20 * (monster.strength as u32).max(1) / 2).max(10); // ensure at least 10
        let max_gp = (50 * monster.strength as u32).min(500).max(min_gp + 5);
        let reward_gp = rng.random_range(min_gp..=max_gp);
        inventory.gold_pieces = inventory.gold_pieces.saturating_add(reward_gp);
        inventory.save_after_pickup();
        print_event_summary("Victory", &before, inventory, &[], &[]);
        println!("🏆 You defeated the {}!", monster.name);
        println!("💰 Loot: {} gold pieces", reward_gp);
    } else {
        // Lose between 5% and 10% of current gold pieces
        if inventory.gold_pieces == 0 {
            print_event_summary("Defeat", &before, inventory, &[], &[]);
            println!("😣 You were defeated by the {}, but you carried no gold to lose.", monster.name);
            return;
        }
        let loss_percent = rng.random_range(5..=10);
        let loss = ((inventory.gold_pieces as f64) * (loss_percent as f64 / 100.0)).round() as u32;
        let loss = loss.clamp(1, inventory.gold_pieces); // at least 1 if you have gold
        inventory.gold_pieces -= loss;
        inventory.save_after_pickup();
        print_event_summary("Defeat", &before, inventory, &[], &[]);
        println!("💀 The {} overpowered you! You drop {} gold pieces ({}%).", monster.name, loss, loss_percent);
    }
}

fn visit_shop(inventory: &mut Inventory) {
    use dialoguer::{MultiSelect, Confirm};
    if inventory.items.is_empty() {
    print_simple_header("Shop");
    println!("🛒 The shop is quiet. You have no items to sell.");
        return;
    }

    // Generate a random price list (in copper pieces) for this visit
    let mut rng = rand::rng();
    // Price range: 30 cp (0.3 gp) to 1000 cp (10 gp)
    let prices_cp: Vec<u32> = inventory.items.iter().map(|_| rng.random_range(30..=1000)).collect();

    // Build display list with formatted prices
    let mut display: Vec<String> = Vec::with_capacity(inventory.items.len());
    for (item, price_cp) in inventory.items.iter().zip(&prices_cp) {
        display.push(format!("{} (offers {})", item, format_cp(*price_cp)));
    }

    print_simple_header("Shop");
    println!("🛒 You enter a dimly lit shop. The dealer eyes your goods...");
    println!("Select items to sell (space to toggle, enter to confirm):");

    let selections = MultiSelect::new()
        .items(&display)
        .interact();

    let selected_indices = match selections {
        Ok(v) if !v.is_empty() => v,
        Ok(_) => { println!("You decide not to sell anything."); return; },
        Err(e) => { println!("Shop interaction failed: {}", e); return; }
    };

    let mut total_cp: u32 = 0;
    for &idx in &selected_indices {
        total_cp = total_cp.saturating_add(prices_cp[idx]);
    }

    println!("💰 The dealer totals the offer: {} for {} item(s).", format_cp(total_cp), selected_indices.len());
    if !Confirm::new().with_prompt("Accept the deal?").default(true).interact().unwrap_or(false) {
        println!("You decline the offer.");
        return;
    }

    // Snapshot before
    let before = inventory.clone();
    // Remove sold items (retain those not selected)
    let selected_set: std::collections::HashSet<usize> = selected_indices.iter().copied().collect();
    let mut removed: Vec<String> = Vec::new();
    inventory.items = inventory.items.iter().enumerate()
        .filter_map(|(i, it)| {
            if selected_set.contains(&i) { removed.push(it.clone()); None } else { Some(it.clone()) }
        })
        .collect();

    inventory.add_copper(total_cp);
    inventory.save_after_pickup();
    print_event_summary("Shop Sale", &before, inventory, &[], &removed);
    println!("✅ Sold {} item(s): {}", removed.len(), if removed.len() <= 5 { removed.join(", ") } else { format!("{} items", removed.len()) });
}

// ---------------- Tavern Feature ----------------
// Assumptions (can be adjusted later):
// Drink cost: 5 sp, Food cost: 12 sp (1 gp 2 sp), Stay night: 2 gp, Bartender tip: 5 gp for 40% luck gain chance.
// Luck grants a 90% chance next pickpocket will trigger mysterious figure (consumed after attempt).

const TAVERN_DRINK_COST_SP: u32 = 5;      // 0g 5s 0c
const TAVERN_FOOD_COST_SP: u32 = 12;      // 1g 2s? Actually 1g=10s so 12s = 1g 2s but we store in breakdown
const TAVERN_STAY_COST_GP: u32 = 2;       // 2g
const TAVERN_TIP_COST_GP: u32 = 5;        // 5g
const TAVERN_LUCK_CHANCE: f64 = 0.40;     // 40%

fn visit_tavern(inventory: &mut Inventory) {
    use dialoguer::Select;
    loop {
    let options = [
            "Buy Drink", "Buy Food", "Stay The Night", "Tip Bartender (Luck)", "Leave Tavern"
        ];
        print_simple_header("Tavern");
        println!("🍺 You enter a bustling tavern filled with adventurers.");
        if inventory.luck_boost { println!("✨ You feel luck coiled within you awaiting a pickpocket attempt."); }
    let choice = Select::new().items(options).default(0).interact();
        let Ok(choice) = choice else { println!("You awkwardly back out of the tavern."); return; };
        match choice {
            0 => tavern_buy_drink(inventory),
            1 => tavern_buy_food(inventory),
            2 => tavern_stay_night(inventory),
            3 => tavern_tip_bartender(inventory),
            4 => { println!("You leave the tavern."); return; },
            _ => unreachable!(),
        }
    }
}

fn tavern_buy_drink(inventory: &mut Inventory) {
    let total_cp = TAVERN_DRINK_COST_SP * 10; // convert sp to cp
    if !inventory.try_spend_cp(total_cp) { println!("Not enough coin for a drink."); return; }
    println!("🥃 You savor a strong drink. Your spirits lift slightly.");
    inventory.save_after_pickup();
}

fn tavern_buy_food(inventory: &mut Inventory) {
    let total_cp = TAVERN_FOOD_COST_SP * 10; // 12 sp -> 120 cp
    if !inventory.try_spend_cp(total_cp) { println!("You can't afford a hearty meal."); return; }
    println!("🍖 A warm meal restores your resolve.");
    inventory.save_after_pickup();
}

fn tavern_stay_night(inventory: &mut Inventory) {
    let total_cp = TAVERN_STAY_COST_GP * 100; // 2 gp -> 200 cp
    if !inventory.try_spend_cp(total_cp) { println!("You can't afford a room for the night."); return; }
    println!("🛏️  You rest deeply and feel refreshed (mechanical benefit TBD).");
    inventory.save_after_pickup();
}

fn tavern_tip_bartender(inventory: &mut Inventory) {
    if inventory.luck_boost { println!("You already have a stored luck boon."); return; }
    let total_cp = TAVERN_TIP_COST_GP * 100;
    if !inventory.try_spend_cp(total_cp) { println!("The bartender scoffs: 'Gold first, friend.'"); return; }
    let mut rng = rand::rng();
    if rng.random_bool(TAVERN_LUCK_CHANCE) {
        inventory.luck_boost = true;
        println!("🍀 The bartender whispers a charm. You feel fortune gathering for your next pickpocket.");
    } else {
        println!("🍂 The charm fizzles. No luck gained this time.");
    }
    inventory.save_after_pickup();
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Inventory {
    items: Vec<String>,
    copper_pieces: u32,
    silver_pieces: u32,
    gold_pieces: u32,
    #[serde(default)]
    luck_boost: bool,
}

impl Inventory {
    fn new() -> Self {
        Inventory { 
            items: Vec::new(),
            copper_pieces: 0,
            silver_pieces: 0,
            gold_pieces: 0,
            luck_boost: false,
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

    fn add_copper(&mut self, mut copper: u32) {
        // Promote to gold & silver
        self.gold_pieces += copper / 100;
        copper %= 100;
        self.silver_pieces += copper / 10;
        copper %= 10;
        self.copper_pieces += copper;
    }

    fn total_cp(&self) -> u32 {
        self.gold_pieces * 100 + self.silver_pieces * 10 + self.copper_pieces
    }

    // Spend currency using copper as base unit; returns false if insufficient funds
    fn try_spend_cp(&mut self, cost_cp: u32) -> bool {
        let total = self.total_cp();
        if total < cost_cp { return false; }
        let remaining = total - cost_cp;
        self.gold_pieces = remaining / 100;
        let rem = remaining % 100;
        self.silver_pieces = rem / 10;
        self.copper_pieces = rem % 10;
        true
    }
    
    fn parse_currency(&self, item: &str) -> Option<(u32, String)> {
        let cre = currency_regex();
        if let Some(caps) = cre.captures(item)
            && let (Some(amount_str), Some(currency_str)) = (caps.get(1), caps.get(2))
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

fn format_cp(cp: u32) -> String {
    if cp == 0 { return "0 cp".to_string(); }
    let gp = cp / 100;
    let rem = cp % 100;
    let sp = rem / 10;
    let cp_left = rem % 10;
    let mut parts = Vec::new();
    if gp > 0 { parts.push(format!("{} gp", gp)); }
    if sp > 0 { parts.push(format!("{} sp", sp)); }
    if cp_left > 0 { parts.push(format!("{} cp", cp_left)); }
    parts.join(" ")
}

// Unified event display helpers
fn print_simple_header(title: &str) { println!("\n──── {} ────", title); }

fn print_event_summary(title: &str, before: &Inventory, after: &Inventory, items_added: &[String], items_removed: &[String]) {
    print_simple_header(title);
    let dg = after.gold_pieces as i64 - before.gold_pieces as i64;
    let ds = after.silver_pieces as i64 - before.silver_pieces as i64;
    let dc = after.copper_pieces as i64 - before.copper_pieces as i64;
    let mut deltas: Vec<String> = Vec::new();
    if dg != 0 { deltas.push(format_delta(dg, "gp")); }
    if ds != 0 { deltas.push(format_delta(ds, "sp")); }
    if dc != 0 { deltas.push(format_delta(dc, "cp")); }
    if !deltas.is_empty() { println!("💱 Currency change: {}", deltas.join(", ")); }
    if !items_added.is_empty() { println!("➕ Items gained: {}", summarize_items(items_added)); }
    if !items_removed.is_empty() { println!("➖ Items lost: {}", summarize_items(items_removed)); }
    println!("🏦 Holdings: {} gp, {} sp, {} cp", after.gold_pieces, after.silver_pieces, after.copper_pieces);
}

fn format_delta(delta: i64, unit: &str) -> String {
    if delta > 0 { format!("+{} {}", delta, unit) } else { format!("{} {}", delta, unit) }
}

fn summarize_items(items: &[String]) -> String {
    if items.len() <= 5 { items.join(", ") } else { format!("{} items", items.len()) }
}


