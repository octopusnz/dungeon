use wasm_bindgen::prelude::*;
use crate::{inventory::Inventory, loot::parse_and_format_loot_cached, apply_pickpocket_penalty, actions::{pick_pocket, fight_monster_outcome, Rarity,
    TAVERN_DRINK_COST_SP, TAVERN_FOOD_COST_SP, TAVERN_STAY_COST_GP, TAVERN_TIP_COST_GP, TAVERN_LUCK_CHANCE, TAVERN_FLIRT_COST_GP, TAVERN_FLIRT_KISS_CHANCE,
    HAGGLE_SUCCESS_CHANCE
}};
use rand::Rng;
use rand::SeedableRng;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct WasmInventory { pub items: Vec<String>, pub gp: u32, pub sp: u32, pub cp: u32, pub luck: bool }

#[derive(Serialize, Deserialize, Clone)]
pub struct WasmResult { pub state: WasmInventory, pub message: String }

#[derive(Serialize, Deserialize, Clone)]
pub struct ShopItem { pub id: u32, pub name: String, pub rarity: String, pub price_cp: u32 }

#[derive(Serialize, Deserialize, Clone)]
pub struct ShopState { pub items: Vec<ShopItem>, pub haggle_applied: bool }

impl From<Inventory> for WasmInventory {
    fn from(i: Inventory) -> Self { Self { items: i.items, gp: i.gold_pieces, sp: i.silver_pieces, cp: i.copper_pieces, luck: i.luck_boost } }
}

impl From<WasmInventory> for Inventory {
    fn from(w: WasmInventory) -> Self { Inventory { items: w.items, gold_pieces: w.gp, silver_pieces: w.sp, copper_pieces: w.cp, luck_boost: w.luck } }
}

#[wasm_bindgen]
#[derive(Default)]
pub struct Game { inv: Inventory, shop: Option<Vec<ShopItem>> }

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Game { Game { inv: Inventory::new(), shop: None } }

    #[wasm_bindgen]
    pub fn get_state(&self) -> JsValue { serde_wasm_bindgen::to_value(&WasmInventory::from(self.inv.clone())).unwrap() }

    fn wrap(&self, msg: impl Into<String>) -> JsValue {
        let res = WasmResult { state: WasmInventory::from(self.inv.clone()), message: msg.into() };
        serde_wasm_bindgen::to_value(&res).unwrap()
    }

    #[wasm_bindgen]
    pub fn add_loot(&mut self, desc: &str) -> JsValue {
        let (items, _) = parse_and_format_loot_cached(desc);
        for it in items.iter() { self.inv.add_item(it); }
        self.wrap(format!("Added loot: {}", desc))
    }

    #[wasm_bindgen]
    pub fn apply_penalty(&mut self, percent: u32) -> JsValue {
        let lost = apply_pickpocket_penalty(&mut self.inv.gold_pieces, percent); self.wrap(format!("Applied {}% penalty (lost {} gp)", percent, lost))
    }

    /// Perform a pickpocket attempt using provided loot candidate descriptions (comma separated list)
    #[wasm_bindgen]
    pub fn pickpocket(&mut self, loot_candidates: &str) -> JsValue {
        let items: Vec<String> = loot_candidates.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if items.is_empty() { return self.wrap("No loot candidates provided"); }
        pick_pocket(&mut self.inv, &items); self.wrap("Performed pickpocket attempt")
    }

    /// Simulate a monster fight (random outcome & reward internally)
    #[wasm_bindgen]
    pub fn fight(&mut self) -> JsValue {
        let outcome = fight_monster_outcome(&mut self.inv);
        let msg = if outcome.victory { format!("Victory over {} (+{} gp)", outcome.monster, outcome.reward_gp) } else if outcome.loss_gp==0 { format!("Defeated by {} (no gold to lose)", outcome.monster) } else { format!("Defeated by {} (-{} gp)", outcome.monster, outcome.loss_gp) };
        self.wrap(msg)
    }

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
            if success {
                total_cp = ((total_cp as f64) * 0.75).round() as u32;
            } else {
                total_cp = ((total_cp as f64) * 1.10).round() as u32;
            }
            if self.inv.luck_boost { self.inv.luck_boost = false; }
        }
    let msg = if self.inv.total_cp() >= total_cp { let _ = self.inv.try_spend_cp(total_cp); for (n,_,_) in parsed { self.inv.add_item(&n); } format!("Purchased items total {} cp", total_cp) } else { format!("Insufficient funds for {} cp purchase", total_cp) };
        self.wrap(msg)
    }

    // --- Enhanced gameplay style APIs ---
    #[wasm_bindgen]
    pub fn reset(&mut self) -> JsValue { self.inv = Inventory::new(); self.shop=None; self.wrap("Inventory reset") }

    #[wasm_bindgen]
    pub fn generate_shop(&mut self) -> JsValue {
        use rand::Rng;
        use rand::seq::SliceRandom;
        const STOCK: &[(&str, Rarity)] = &[
            ("Rope (50ft)", Rarity::Common),("Torch", Rarity::Common),("Lantern", Rarity::Common),("Oil Flask", Rarity::Common),
            ("Iron Rations", Rarity::Common),("Waterskin", Rarity::Common),("Lockpicks", Rarity::Uncommon),("Bedroll", Rarity::Common),
            ("Backpack", Rarity::Common),("Shovel", Rarity::Common),("Grappling Hook", Rarity::Uncommon),("Hammer & Pitons", Rarity::Common),
            ("Herb Bundle", Rarity::Common),("Ink & Quill", Rarity::Common),("Chalk Pouch", Rarity::Common),
            ("Potion of Healing", Rarity::Uncommon),("Potion of Invisibility", Rarity::Rare),("Scroll of Fireball", Rarity::Rare),
            ("Scroll of Shielding", Rarity::Uncommon),("Ring of Protection", Rarity::Epic),("Amulet of Light", Rarity::Epic),
            ("Wand of Sparks", Rarity::Uncommon),("Boots of Silence", Rarity::Rare),("Cloak of Shadows", Rarity::Epic),
            ("Elixir of Luck", Rarity::Rare),("Orb of Annihilation Shard", Rarity::Legendary)
        ];
        let mut pool: Vec<(&str,Rarity)> = STOCK.to_vec();
        let mut rng = rand::rngs::SmallRng::from_entropy();
        pool.shuffle(&mut rng);
        let count = rng.gen_range(6..=10).min(pool.len() as u32) as usize;
        let chosen = pool.into_iter().take(count);
        let items: Vec<ShopItem> = chosen.enumerate().map(|(id,(name,rar))| {
            let range = rar.price_range_cp();
            let price = rng.gen_range(*range.start()..=*range.end());
            ShopItem { id: id as u32, name: name.to_string(), rarity: rar.label().to_string(), price_cp: price }
        }).collect();
        self.shop = Some(items.clone());
        serde_wasm_bindgen::to_value(&ShopState { items, haggle_applied: false }).unwrap()
    }

    #[wasm_bindgen]
    pub fn shop_purchase(&mut self, indices: Vec<u32>, attempt_haggle: bool, spend_luck: bool) -> JsValue {
        if self.shop.is_none() { return self.wrap("No shop stock generated yet"); }
        if spend_luck { self.inv.luck_boost = true; }
        let stock = self.shop.as_ref().unwrap();
        if indices.is_empty() { return self.wrap("No items selected"); }
        let mut total: u32 = 0; let mut names: Vec<String> = Vec::new();
        for i in indices.iter() { if let Some(it) = stock.iter().find(|s| s.id==*i) { total = total.saturating_add(it.price_cp); names.push(it.name.clone()); } }
        if total==0 { return self.wrap("Selection invalid"); }
        let mut final_total = total; let mut haggle_msg = String::new();
        if attempt_haggle {
            let chance = if self.inv.luck_boost {0.85} else {HAGGLE_SUCCESS_CHANCE};
            let mut rng = rand::rngs::SmallRng::from_entropy();
            let success = rng.gen_bool(chance);
            if success {
                final_total = ((final_total as f64)*0.75).round() as u32;
                haggle_msg = format!("Haggle success ({}%)", (chance*100.0) as u32);
            } else {
                final_total = ((final_total as f64)*1.10).round() as u32;
                haggle_msg = format!("Haggle failed ({}%)", (chance*100.0) as u32);
            }
            if self.inv.luck_boost { self.inv.luck_boost=false; }
        }
        if self.inv.total_cp() < final_total { return self.wrap(format!("Need {} cp but only have {} cp", final_total, self.inv.total_cp())); }
        let _ = self.inv.try_spend_cp(final_total);
        for n in names.iter() { self.inv.add_item(n); }
        self.wrap(format!("Bought {} item(s) for {} cp. {}", names.len(), final_total, haggle_msg))
    }

    #[wasm_bindgen]
    pub fn tavern(&mut self, action: &str) -> JsValue {
        use rand::Rng;
        let mut rng = rand::rngs::SmallRng::from_entropy();
        let msg = match action.to_lowercase().as_str() {
            "drink" => { if !self.inv.try_spend_cp(TAVERN_DRINK_COST_SP*10) { "Not enough coin for drink".into() } else { "Enjoyed a stiff drink".into() } },
            "food" => { if !self.inv.try_spend_cp(TAVERN_FOOD_COST_SP*10) { "Can't afford meal".into() } else { "A hearty meal restores you".into() } },
            "stay" => { if !self.inv.try_spend_cp(TAVERN_STAY_COST_GP*100) { "Not enough for room".into() } else { "You rest peacefully".into() } },
            "tip" => { if self.inv.luck_boost { "Luck already stored".into() } else if !self.inv.try_spend_cp(TAVERN_TIP_COST_GP*100) { "Need more gold to tip".into() } else if rng.gen_bool(TAVERN_LUCK_CHANCE) { self.inv.luck_boost=true; "Luck granted from generous tip".into() } else { "Tip given, no luck".into() } },
            "flirt" => { if !self.inv.try_spend_cp(TAVERN_FLIRT_COST_GP*100) { "Can't afford to flirt".into() } else if rng.gen_bool(TAVERN_FLIRT_KISS_CHANCE) { if !self.inv.luck_boost { self.inv.luck_boost=true; "A kiss grants you luck".into() } else { "Another kiss, luck unchanged".into() } } else { "No luck this time".into() } },
            other => format!("Unknown action: {}", other)
        };
        self.wrap(msg)
    }
}
