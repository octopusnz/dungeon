use wasm_bindgen::prelude::*;
use crate::{inventory::Inventory, loot::parse_and_format_loot_cached, apply_pickpocket_penalty, actions::{pick_pocket, fight_monster, Rarity}};
use rand::SeedableRng;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct WasmInventory { pub items: Vec<String>, pub gp: u32, pub sp: u32, pub cp: u32, pub luck: bool }

impl From<Inventory> for WasmInventory {
    fn from(i: Inventory) -> Self { Self { items: i.items, gp: i.gold_pieces, sp: i.silver_pieces, cp: i.copper_pieces, luck: i.luck_boost } }
}

impl From<WasmInventory> for Inventory {
    fn from(w: WasmInventory) -> Self { Inventory { items: w.items, gold_pieces: w.gp, silver_pieces: w.sp, copper_pieces: w.cp, luck_boost: w.luck } }
}

#[wasm_bindgen]
#[derive(Default)]
pub struct Game { inv: Inventory }

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Game { Game { inv: Inventory::new() } }

    #[wasm_bindgen]
    pub fn get_state(&self) -> JsValue { serde_wasm_bindgen::to_value(&WasmInventory::from(self.inv.clone())).unwrap() }

    #[wasm_bindgen]
    pub fn add_loot(&mut self, desc: &str) -> JsValue {
        let (items, _) = parse_and_format_loot_cached(desc);
        for it in items.iter() { self.inv.add_item(it); }
        self.get_state()
    }

    #[wasm_bindgen]
    pub fn apply_penalty(&mut self, percent: u32) -> JsValue {
        let _ = apply_pickpocket_penalty(&mut self.inv.gold_pieces, percent); self.get_state()
    }

    /// Perform a pickpocket attempt using provided loot candidate descriptions (comma separated list)
    #[wasm_bindgen]
    pub fn pickpocket(&mut self, loot_candidates: &str) -> JsValue {
        let items: Vec<String> = loot_candidates.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if items.is_empty() { return self.get_state(); }
        pick_pocket(&mut self.inv, &items); self.get_state()
    }

    /// Simulate a monster fight (random outcome & reward internally)
    #[wasm_bindgen]
    pub fn fight(&mut self) -> JsValue { fight_monster(&mut self.inv); self.get_state() }

    /// Simulate buying items (name1[:rarity]|name2 ...) applying haggle flag; rarity optional (Common,Uncommon,Rare,Epic,Legendary)
    #[wasm_bindgen]
    pub fn shop_buy(&mut self, items_spec: &str, attempt_haggle: bool, luck: bool) -> JsValue {
        use rand::Rng;
        let mut rng = rand::rngs::SmallRng::from_entropy();
        if luck { self.inv.luck_boost = true; }
        let mut total_cp = 0u32; let mut parsed: Vec<(String,Rarity,u32)> = Vec::new();
        for raw in items_spec.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            let mut parts = raw.split(':');
            let name = parts.next().unwrap().trim();
            let rar = parts.next().map(|r| match r.to_lowercase().as_str() {
                "uncommon"=>Rarity::Uncommon, "rare"=>Rarity::Rare, "epic"=>Rarity::Epic, "legendary"=>Rarity::Legendary, _=>Rarity::Common
            }).unwrap_or(Rarity::Common);
            let (min,max) = match rar {
                Rarity::Common => (5,50), Rarity::Uncommon => (50,500), Rarity::Rare => (500,5_000), Rarity::Epic => (5_000,20_000), Rarity::Legendary => (20_000,50_000)
            };
            let price = rng.gen_range(min..=max);
            total_cp = total_cp.saturating_add(price); parsed.push((name.to_string(),rar,price));
        }
        if attempt_haggle && !parsed.is_empty() {
            let success_chance = if self.inv.luck_boost { 0.85 } else { 0.50 };
            let success = rng.gen_bool(success_chance);
            if success { total_cp = ((total_cp as f64)*0.75).round() as u32; }
            else { total_cp = ((total_cp as f64)*1.10).round() as u32; }
            if self.inv.luck_boost { self.inv.luck_boost = false; }
        }
        if self.inv.total_cp() >= total_cp { let _ = self.inv.try_spend_cp(total_cp); for (n,_,_) in parsed { self.inv.add_item(&n); } }
        self.get_state()
    }
}
