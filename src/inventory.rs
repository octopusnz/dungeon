use serde::{Deserialize, Serialize};
#[cfg(any(feature = "cli", test))]
use std::fs;

pub const SAVE_FILE: &str = "inventory.json"; // Exposed so CLI reset flag can remove the file

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Inventory {
    pub items: Vec<String>,
    pub copper_pieces: u32,
    pub silver_pieces: u32,
    pub gold_pieces: u32,
    #[serde(default)]
    pub luck_boost: bool,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            copper_pieces: 0,
            silver_pieces: 0,
            gold_pieces: 0,
            luck_boost: false,
        }
    }

    pub fn add_item(&mut self, item: &str) {
        if let Some((amount, cur)) = self.parse_currency(item) {
            match cur.as_str() {
                "cp" => self.copper_pieces += amount,
                "sp" => self.silver_pieces += amount,
                "gp" => self.gold_pieces += amount,
                _ => self.items.push(item.to_string()),
            }
        } else {
            self.items.push(item.to_string());
        }
    }

    pub fn add_copper(&mut self, mut copper: u32) {
        self.gold_pieces += copper / 100;
        copper %= 100;
        self.silver_pieces += copper / 10;
        copper %= 10;
        self.copper_pieces += copper;
    }

    pub fn total_cp(&self) -> u32 {
        self.gold_pieces * 100 + self.silver_pieces * 10 + self.copper_pieces
    }

    pub fn try_spend_cp(&mut self, cost_cp: u32) -> bool {
        let total = self.total_cp();
        if total < cost_cp {
            return false;
        }
        let remaining = total - cost_cp;
        self.gold_pieces = remaining / 100;
        let rem = remaining % 100;
        self.silver_pieces = rem / 10;
        self.copper_pieces = rem % 10;
        true
    }

    pub fn parse_currency(&self, item: &str) -> Option<(u32, String)> {
        let cre = crate::loot::currency_regex();
        if let Some(caps) = cre.captures(item)
            && let (Some(amount_str), Some(currency_str)) = (caps.get(1), caps.get(2))
            && let Ok(amount) = amount_str.as_str().parse::<u32>()
        {
            return Some((amount, currency_str.as_str().to_string()));
        }
        None
    }

    pub fn save_after_pickup(&mut self) {
        #[cfg(any(feature = "cli", test))]
        {
            if let Err(e) = self.save() {
                println!("⚠️  Failed to save inventory: {}", e);
            }
        }
        // wasm-only build: no-op (avoid fs dependency / size)
    }
    #[cfg(feature = "cli")]
    pub fn show(&self) {
        let has_items = !self.items.is_empty();
        let has_currency = self.copper_pieces > 0 || self.silver_pieces > 0 || self.gold_pieces > 0;
        if !has_items && !has_currency {
            println!("Your inventory is empty.");
            return;
        }
        println!("Your inventory contains:");
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
        if has_items {
            if has_currency {
                println!("🎒 Items:");
            }
            for item in &self.items {
                println!("  • {}", item);
            }
        }
    }

    #[cfg(any(feature = "cli", test))]
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(SAVE_FILE, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
    #[cfg(any(feature = "cli", test))]
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&std::fs::read_to_string(SAVE_FILE)?)?)
    }
    #[cfg(all(feature = "wasm", not(feature = "cli"), not(test)))]
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    #[cfg(all(feature = "wasm", not(feature = "cli"), not(test)))]
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::new())
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_cp(cp: u32) -> String {
    if cp == 0 {
        return "0 cp".to_string();
    }
    let gp = cp / 100;
    let rem = cp % 100;
    let sp = rem / 10;
    let cp_left = rem % 10;
    let mut parts = Vec::new();
    if gp > 0 {
        parts.push(format!("{} gp", gp));
    }
    if sp > 0 {
        parts.push(format!("{} sp", sp));
    }
    if cp_left > 0 {
        parts.push(format!("{} cp", cp_left));
    }
    parts.join(" ")
}
