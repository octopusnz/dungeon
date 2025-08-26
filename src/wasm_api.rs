use crate::{
    actions::{
        HAGGLE_SUCCESS_CHANCE, Rarity, TAVERN_DRINK_COST_SP, TAVERN_FLIRT_COST_GP,
        TAVERN_FLIRT_KISS_CHANCE, TAVERN_FOOD_COST_SP, TAVERN_LUCK_CHANCE, TAVERN_STAY_COST_GP,
        TAVERN_TIP_COST_GP, fight_monster_outcome, pick_pocket,
    },
    apply_pickpocket_penalty,
    inventory::Inventory,
    loot::parse_and_format_loot_cached,
};
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Clone)]
pub struct WasmInventory {
    pub items: Vec<String>,
    pub gp: u32,
    pub sp: u32,
    pub cp: u32,
    pub luck: bool,
    pub max_hp: u32,
    pub current_hp: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WasmResult {
    pub state: WasmInventory,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ShopItem {
    pub id: u32,
    pub name: String,
    pub rarity: String,
    pub price_cp: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ShopState {
    pub items: Vec<ShopItem>,
    pub haggle_applied: bool,
}

impl From<Inventory> for WasmInventory {
    fn from(i: Inventory) -> Self {
        Self {
            items: i.items,
            gp: i.gold_pieces,
            sp: i.silver_pieces,
            cp: i.copper_pieces,
            luck: i.luck_boost,
            max_hp: i.max_hp,
            current_hp: i.current_hp,
        }
    }
}

impl From<WasmInventory> for Inventory {
    fn from(w: WasmInventory) -> Self {
        Inventory {
            items: w.items,
            gold_pieces: w.gp,
            silver_pieces: w.sp,
            copper_pieces: w.cp,
            luck_boost: w.luck,
            max_hp: if w.max_hp == 0 { 20 } else { w.max_hp },
            current_hp: if w.current_hp == 0 { w.max_hp.max(20) } else { w.current_hp.min(w.max_hp.max(20)) },
        }
    }
}

#[wasm_bindgen]
#[derive(Default)]
pub struct Game {
    inv: Inventory,
    shop: Option<Vec<ShopItem>>,
    active_fight: Option<FightEncounter>,
}

#[derive(Clone)]
struct FightEncounter {
    monster: crate::actions::Monster,
    monster_hp: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WasmFightState {
    pub inventory: WasmInventory,
    pub message: String,
    pub in_fight: bool,
    pub monster: Option<String>,
    pub monster_hp: u32,
    pub monster_max_hp: u32,
    pub player_hp: u32,
    pub player_max_hp: u32,
    pub lines: Vec<String>,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Game {
        Game {
            inv: Inventory::new(),
            shop: None,
            active_fight: None,
        }
    }

    #[wasm_bindgen]
    pub fn get_state(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&WasmInventory::from(self.inv.clone())).unwrap()
    }

    fn wrap(&self, msg: impl Into<String>) -> JsValue {
        let res = WasmResult {
            state: WasmInventory::from(self.inv.clone()),
            message: msg.into(),
        };
        serde_wasm_bindgen::to_value(&res).unwrap()
    }

    #[wasm_bindgen]
    pub fn add_loot(&mut self, desc: &str) -> JsValue {
        let (items, _) = parse_and_format_loot_cached(desc);
        for it in items.iter() {
            self.inv.add_item(it);
        }
        self.wrap(format!("Added loot: {}", desc))
    }

    #[wasm_bindgen]
    pub fn apply_penalty(&mut self, percent: u32) -> JsValue {
        let lost = apply_pickpocket_penalty(&mut self.inv.gold_pieces, percent);
        self.wrap(format!("Applied {}% penalty (lost {} gp)", percent, lost))
    }

    /// Perform a pickpocket attempt using provided loot candidate descriptions (comma separated list)
    #[wasm_bindgen]
    pub fn pickpocket(&mut self, loot_candidates: &str) -> JsValue {
        // If caller supplies candidates, use them; otherwise generate a random candidate set.
        let mut items: Vec<String> = loot_candidates
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if items.is_empty() {
            // Auto-generate 5 random candidate loot descriptions (currency + trinket)
            const TRINKETS: &[&str] = &[
                "silver ring",
                "brass key",
                "tiny idol",
                "opal shard",
                "bloodstone",
                "engraved locket",
                "vellum scroll",
                "jeweled clasp",
                "carved bone die",
                "amber bead",
                "ancient coin",
                "silk ribbon",
            ];
            let mut rng = rand::rngs::SmallRng::from_entropy();
            let count = 5;
            for _ in 0..count {
                let gp = rng.gen_range(1..=25); // modest gold range
                let trinket = TRINKETS[rng.gen_range(0..TRINKETS.len())];
                // 50% chance to mix silver instead of gp for variety
                let desc = if rng.gen_bool(0.5) {
                    let sp = rng.gen_range(2..=40);
                    format!("{} gp and {} sp and a {}", gp, sp, trinket)
                } else {
                    format!("{} gp and a {}", gp, trinket)
                };
                items.push(desc);
            }
        }
        let before_gp = self.inv.gold_pieces;
        let before_items = self.inv.items.len();
        let had_luck = self.inv.luck_boost;
        pick_pocket(&mut self.inv, &items);
        let gained_gp = self.inv.gold_pieces.saturating_sub(before_gp);
        let added_items = self.inv.items.len().saturating_sub(before_items);
        let mut parts = Vec::new();
        if gained_gp > 0 {
            parts.push(format!("+{} gp", gained_gp));
        }
        if added_items > 0 {
            parts.push(format!("{} item(s) added", added_items));
        }
        if had_luck && !self.inv.luck_boost {
            parts.push("luck spent".into());
        }
        if parts.is_empty() {
            parts.push("no gain".into());
        }
        self.wrap(format!("Pickpocket: {}", parts.join(", ")))
    }

    /// Simulate a monster fight (random outcome & reward internally)
    #[wasm_bindgen]
    pub fn fight(&mut self) -> JsValue {
        let outcome = fight_monster_outcome(&mut self.inv);
        let msg = if outcome.victory {
            format!(
                "Victory over {} (+{} gp)",
                outcome.monster, outcome.reward_gp
            )
        } else if outcome.loss_gp == 0 {
            format!("Defeated by {} (no gold to lose)", outcome.monster)
        } else {
            format!("Defeated by {} (-{} gp)", outcome.monster, outcome.loss_gp)
        };
        self.wrap(msg)
    }

    /// Simulate buying items (name1[:rarity]|name2 ...) applying haggle flag; rarity optional (Common,Uncommon,Rare,Epic,Legendary)
    #[wasm_bindgen]
    pub fn shop_buy(&mut self, items_spec: &str, attempt_haggle: bool, luck: bool) -> JsValue {
        use rand::Rng;
        let mut rng = rand::rngs::SmallRng::from_entropy();
        if luck {
            self.inv.luck_boost = true;
        }
        let mut total_cp = 0u32;
        let mut parsed: Vec<(String, Rarity, u32)> = Vec::new();
        for raw in items_spec
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let mut parts = raw.split(':');
            let name = parts.next().unwrap().trim();
            let rar = parts
                .next()
                .map(|r| match r.to_lowercase().as_str() {
                    "uncommon" => Rarity::Uncommon,
                    "rare" => Rarity::Rare,
                    "epic" => Rarity::Epic,
                    "legendary" => Rarity::Legendary,
                    _ => Rarity::Common,
                })
                .unwrap_or(Rarity::Common);
            let (min, max) = match rar {
                Rarity::Common => (5, 50),
                Rarity::Uncommon => (50, 500),
                Rarity::Rare => (500, 5_000),
                Rarity::Epic => (5_000, 20_000),
                Rarity::Legendary => (20_000, 50_000),
            };
            let price = rng.gen_range(min..=max);
            total_cp = total_cp.saturating_add(price);
            parsed.push((name.to_string(), rar, price));
        }
        if attempt_haggle && !parsed.is_empty() {
            let success_chance = if self.inv.luck_boost { 0.85 } else { 0.50 };
            let success = rng.gen_bool(success_chance);
            if success {
                total_cp = ((total_cp as f64) * 0.75).round() as u32;
            } else {
                total_cp = ((total_cp as f64) * 1.10).round() as u32;
            }
            if self.inv.luck_boost {
                self.inv.luck_boost = false;
            }
        }
        let msg = if self.inv.total_cp() >= total_cp {
            let _ = self.inv.try_spend_cp(total_cp);
            for (n, _, _) in parsed {
                self.inv.add_item(&n);
            }
            format!("Purchased items total {} cp", total_cp)
        } else {
            format!("Insufficient funds for {} cp purchase", total_cp)
        };
        self.wrap(msg)
    }

    // --- Enhanced gameplay style APIs ---
    #[wasm_bindgen]
    pub fn reset(&mut self) -> JsValue {
        self.inv = Inventory::new();
        self.shop = None;
    self.active_fight = None;
        self.wrap("Inventory reset")
    }

    #[wasm_bindgen]
    pub fn generate_shop(&mut self) -> JsValue {
        use rand::Rng;
        use rand::seq::SliceRandom;
        const STOCK: &[(&str, Rarity)] = &[
            ("Rope (50ft)", Rarity::Common),
            ("Torch", Rarity::Common),
            ("Lantern", Rarity::Common),
            ("Oil Flask", Rarity::Common),
            ("Iron Rations", Rarity::Common),
            ("Waterskin", Rarity::Common),
            ("Lockpicks", Rarity::Uncommon),
            ("Bedroll", Rarity::Common),
            ("Backpack", Rarity::Common),
            ("Shovel", Rarity::Common),
            ("Grappling Hook", Rarity::Uncommon),
            ("Hammer & Pitons", Rarity::Common),
            ("Herb Bundle", Rarity::Common),
            ("Ink & Quill", Rarity::Common),
            ("Chalk Pouch", Rarity::Common),
            ("Potion of Healing", Rarity::Uncommon),
            ("Potion of Invisibility", Rarity::Rare),
            ("Scroll of Fireball", Rarity::Rare),
            ("Scroll of Shielding", Rarity::Uncommon),
            ("Ring of Protection", Rarity::Epic),
            ("Amulet of Light", Rarity::Epic),
            ("Wand of Sparks", Rarity::Uncommon),
            ("Boots of Silence", Rarity::Rare),
            ("Cloak of Shadows", Rarity::Epic),
            ("Elixir of Luck", Rarity::Rare),
            ("Orb of Annihilation Shard", Rarity::Legendary),
        ];
        let mut pool: Vec<(&str, Rarity)> = STOCK.to_vec();
        let mut rng = rand::rngs::SmallRng::from_entropy();
        pool.shuffle(&mut rng);
        let count = rng.gen_range(6..=10).min(pool.len() as u32) as usize;
        let chosen = pool.into_iter().take(count);
        let items: Vec<ShopItem> = chosen
            .enumerate()
            .map(|(id, (name, rar))| {
                let range = rar.price_range_cp();
                let price = rng.gen_range(*range.start()..=*range.end());
                ShopItem {
                    id: id as u32,
                    name: name.to_string(),
                    rarity: rar.label().to_string(),
                    price_cp: price,
                }
            })
            .collect();
        self.shop = Some(items.clone());
        serde_wasm_bindgen::to_value(&ShopState {
            items,
            haggle_applied: false,
        })
        .unwrap()
    }

    #[wasm_bindgen]
    pub fn shop_purchase(
        &mut self,
        indices: Vec<u32>,
        attempt_haggle: bool,
        spend_luck: bool,
    ) -> JsValue {
        if self.shop.is_none() {
            return self.wrap("No shop stock generated yet");
        }
        if spend_luck {
            self.inv.luck_boost = true;
        }
        let stock = self.shop.as_ref().unwrap();
        if indices.is_empty() {
            return self.wrap("No items selected");
        }
        let mut total: u32 = 0;
        let mut names: Vec<String> = Vec::new();
        for i in indices.iter() {
            if let Some(it) = stock.iter().find(|s| s.id == *i) {
                total = total.saturating_add(it.price_cp);
                names.push(it.name.clone());
            }
        }
        if total == 0 {
            return self.wrap("Selection invalid");
        }
        let mut final_total = total;
        let mut haggle_msg = String::new();
        if attempt_haggle {
            let chance = if self.inv.luck_boost {
                0.85
            } else {
                HAGGLE_SUCCESS_CHANCE
            };
            let mut rng = rand::rngs::SmallRng::from_entropy();
            let success = rng.gen_bool(chance);
            if success {
                final_total = ((final_total as f64) * 0.75).round() as u32;
                haggle_msg = format!("Haggle success ({}%)", (chance * 100.0) as u32);
            } else {
                final_total = ((final_total as f64) * 1.10).round() as u32;
                haggle_msg = format!("Haggle failed ({}%)", (chance * 100.0) as u32);
            }
            if self.inv.luck_boost {
                self.inv.luck_boost = false;
            }
        }
        if self.inv.total_cp() < final_total {
            return self.wrap(format!(
                "Need {} cp but only have {} cp",
                final_total,
                self.inv.total_cp()
            ));
        }
        let _ = self.inv.try_spend_cp(final_total);
        for n in names.iter() {
            self.inv.add_item(n);
        }
        self.wrap(format!(
            "Bought {} item(s) for {} cp. {}",
            names.len(),
            final_total,
            haggle_msg
        ))
    }

    #[wasm_bindgen]
    pub fn tavern(&mut self, action: &str) -> JsValue {
        use rand::Rng;
        let mut rng = rand::rngs::SmallRng::from_entropy();
        let msg = match action.to_lowercase().as_str() {
            "drink" => {
                if !self.inv.try_spend_cp(TAVERN_DRINK_COST_SP * 10) {
                    "Not enough coin for drink".into()
                } else {
                    "Enjoyed a stiff drink".into()
                }
            }
            "food" => {
                if !self.inv.try_spend_cp(TAVERN_FOOD_COST_SP * 10) {
                    "Can't afford meal".into()
                } else {
                    "A hearty meal restores you".into()
                }
            }
            "stay" => {
                if !self.inv.try_spend_cp(TAVERN_STAY_COST_GP * 100) {
                    "Not enough for room".into()
                } else {
                    "You rest peacefully".into()
                }
            }
            "tip" => {
                if self.inv.luck_boost {
                    "Luck already stored".into()
                } else if !self.inv.try_spend_cp(TAVERN_TIP_COST_GP * 100) {
                    "Need more gold to tip".into()
                } else if rng.gen_bool(TAVERN_LUCK_CHANCE) {
                    self.inv.luck_boost = true;
                    "Luck granted from generous tip".into()
                } else {
                    "Tip given, no luck".into()
                }
            }
            "flirt" => {
                if !self.inv.try_spend_cp(TAVERN_FLIRT_COST_GP * 100) {
                    "Can't afford to flirt".into()
                } else if rng.gen_bool(TAVERN_FLIRT_KISS_CHANCE) {
                    if !self.inv.luck_boost {
                        self.inv.luck_boost = true;
                        "A kiss grants you luck".into()
                    } else {
                        "Another kiss, luck unchanged".into()
                    }
                } else {
                    "No luck this time".into()
                }
            }
            other => format!("Unknown action: {}", other),
        };
        self.wrap(msg)
    }

    // --- Interactive fight API (browser) ---
    fn fight_state(&self, message: impl Into<String>, lines: Vec<String>) -> JsValue {
        let (monster, m_hp, m_max) = if let Some(f) = &self.active_fight {
            (Some(f.monster.name.to_string()), f.monster_hp, f.monster.max_hp())
        } else {
            (None, 0, 0)
        };
        let fs = WasmFightState {
            inventory: WasmInventory::from(self.inv.clone()),
            message: message.into(),
            in_fight: self.active_fight.is_some(),
            monster,
            monster_hp: m_hp,
            monster_max_hp: m_max,
            player_hp: self.inv.current_hp,
            player_max_hp: self.inv.max_hp,
            lines,
        };
        serde_wasm_bindgen::to_value(&fs).unwrap()
    }

    #[wasm_bindgen]
    pub fn fight_start(&mut self) -> JsValue {
        if self.active_fight.is_some() {
            return self.fight_state("Already in battle", vec![]);
        }
        // Initialize player hp if needed
        if self.inv.max_hp == 0 { self.inv.max_hp = 20; }
        if self.inv.current_hp == 0 || self.inv.current_hp > self.inv.max_hp { self.inv.current_hp = self.inv.max_hp; }
        // Monster table (mirror of CLI)
        const MONSTERS: &[crate::actions::Monster] = &[
            crate::actions::Monster { name: "Goblin Sneak", strength: 1 },
            crate::actions::Monster { name: "Cave Rat", strength: 1 },
            crate::actions::Monster { name: "Skeleton Guard", strength: 2 },
            crate::actions::Monster { name: "Orc Marauder", strength: 3 },
            crate::actions::Monster { name: "Ghoul", strength: 4 },
            crate::actions::Monster { name: "Ogre Brute", strength: 5 },
            crate::actions::Monster { name: "Wyvern", strength: 6 },
            crate::actions::Monster { name: "Vampire Stalker", strength: 7 },
            crate::actions::Monster { name: "Stone Golem", strength: 8 },
            crate::actions::Monster { name: "Ancient Lich", strength: 9 },
            crate::actions::Monster { name: "Dragon Wyrm", strength: 10 },
        ];
        let mut rng = rand::rngs::SmallRng::from_entropy();
        use rand::seq::SliceRandom;
        let monster = *MONSTERS.choose(&mut rng).unwrap();
        let encounter = FightEncounter { monster, monster_hp: monster.max_hp() };
        self.active_fight = Some(encounter);
    self.fight_state(format!("A {} appears!", monster.name), vec![format!("A {} appears with {} HP!", monster.name, monster.max_hp())])
    }

    #[wasm_bindgen]
    pub fn fight_attack(&mut self) -> JsValue {
        if self.active_fight.is_none() { return self.fight_state("No active fight", vec![]); }
        let mut rng = rand::rngs::SmallRng::from_entropy();
        // Player attack
        let mut lines: Vec<String> = Vec::new();
        if let Some(enc) = &mut self.active_fight {
            let dmg = rng.gen_range(2..=6);
            enc.monster_hp = enc.monster_hp.saturating_sub(dmg);
            lines.push(format!("You strike the {} for {} damage", enc.monster.name, dmg));
            if enc.monster_hp == 0 {
                // Victory & reward
                let min_gp = (10 * (enc.monster.strength as u32).max(1)).max(5);
                let max_gp = (40 * enc.monster.strength as u32).min(400).max(min_gp + 5);
                let reward = rng.gen_range(min_gp..=max_gp);
                self.inv.gold_pieces = self.inv.gold_pieces.saturating_add(reward);
                self.inv.save_after_pickup();
                let name = enc.monster.name.to_string();
                self.active_fight = None;
                lines.push(format!("You defeat the {} and gain {} gp", name, reward));
                return self.fight_state(format!("Victory over {}", name), lines);
            }
        }
        // Monster retaliates if still alive
        if let Some(enc) = &mut self.active_fight {
            let dmg = rng.gen_range(enc.monster.damage_range());
            self.inv.current_hp = self.inv.current_hp.saturating_sub(dmg);
            lines.push(format!("The {} hits you for {} damage", enc.monster.name, dmg));
            if self.inv.current_hp == 0 {
                // Defeat penalty: 10% gold & up to 3 items, restore hp
                let before_gp = self.inv.gold_pieces;
                let loss = ((before_gp as f64) * 0.10).round() as u32;
                let loss = loss.clamp(0, self.inv.gold_pieces);
                self.inv.gold_pieces -= loss;
                // Remove up to 3 random items
                for _ in 0..3 { if self.inv.items.is_empty() { break; } let idx = rng.gen_range(0..self.inv.items.len()); self.inv.items.remove(idx); }
                self.inv.current_hp = self.inv.max_hp; // restore
                self.inv.save_after_pickup();
                let name = enc.monster.name.to_string();
                self.active_fight = None;
                lines.push(format!("You are defeated by {} (-{} gp)", name, loss));
                return self.fight_state(format!("Defeated by {}", name), lines);
            }
        }
        self.fight_state("Exchange blows", lines)
    }

    #[wasm_bindgen]
    pub fn fight_flee(&mut self) -> JsValue {
    if self.active_fight.is_none() { return self.fight_state("No active fight", vec![]); }
        // Flee penalty: lose 5% gold (rounded) & 1 random item
        let mut rng = rand::rngs::SmallRng::from_entropy();
        let before_gold = self.inv.gold_pieces;
        let gold_loss = ((before_gold as f64) * 0.05).round() as u32;
        let gold_loss = gold_loss.clamp(0, self.inv.gold_pieces);
        self.inv.gold_pieces -= gold_loss;
        if !self.inv.items.is_empty() { let idx = rng.gen_range(0..self.inv.items.len()); self.inv.items.remove(idx); }
        self.inv.save_after_pickup();
        self.active_fight = None;
    self.fight_state(format!("You flee"), vec![format!("You flee, dropping {} gp", gold_loss)])
    }

    #[wasm_bindgen]
    pub fn fight_quit(&mut self) -> JsValue {
    if self.active_fight.is_none() { return self.fight_state("No active fight", vec![]); }
        self.active_fight = None;
    self.fight_state("You withdraw", vec!["You withdraw from the battle".into()])
    }
}
