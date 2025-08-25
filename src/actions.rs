use rand::prelude::IndexedRandom;
use rand::Rng;
use rand::seq::SliceRandom;
use crate::inventory::{Inventory, format_cp};
use crate::loot::{parse_and_format_loot_cached, currency_regex};
use crate::rng::with_rng;
use dialoguer::{MultiSelect, Confirm, Select};

// Probabilities
pub const EVENT_CHANCE: f64 = 0.05;
pub const PICKPOCKET_SUCCESS: f64 = 0.50;

// Tavern constants
pub const TAVERN_DRINK_COST_SP: u32 = 5;
pub const TAVERN_FOOD_COST_SP: u32 = 12;
pub const TAVERN_STAY_COST_GP: u32 = 2;
pub const TAVERN_TIP_COST_GP: u32 = 5;
pub const TAVERN_LUCK_CHANCE: f64 = 0.40;
pub const TAVERN_FLIRT_COST_GP: u32 = 10; // cost to flirt with barmaid
pub const TAVERN_FLIRT_KISS_CHANCE: f64 = 0.05; // 5% chance to gain luck via kiss

#[derive(Clone, Copy)]
pub struct Monster { pub name: &'static str, pub strength: u8 }

pub fn pick_pocket(inv: &mut Inventory, loot_items: &[String]) {
    let before = inv.clone();
    let mut non_currency_added: Vec<String> = Vec::with_capacity(4);
    let mut narrative: Vec<String> = Vec::with_capacity(2);
    let mut title = String::from("Pickpocket");
    let boosted = inv.luck_boost; let event_chance = if boosted { 0.90 } else { EVENT_CHANCE };
    if with_rng(|r| r.random_bool(event_chance)) {
        title = "Mysterious Figure".into();
        narrative.push("A mysterious figure emerges from the shadows...".into());
        inv.add_item("1000 gp");
    } else if with_rng(|r| r.random_bool(PICKPOCKET_SUCCESS)) {
        if let Some(desc) = with_rng(|r| loot_items.choose(r).cloned()) {
            let (items, formatted) = parse_and_format_loot_cached(&desc);
            title = "Successful Pickpocket".into();
            narrative.push(format!("You found: {}", formatted));
            let cre = currency_regex();
            for it in items.iter() {
                inv.add_item(it);
                if !cre.is_match(it) { non_currency_added.push(it.clone()); }
            }
        }
    } else {
        title = "Caught Pickpocketing".into();
        let loss_percent = with_rng(|r| r.random_range(5..=10));
        let loss = crate::apply_pickpocket_penalty(&mut inv.gold_pieces, loss_percent);
        narrative.push(if loss>0 { format!("You drop {} gold pieces ({}%) while fleeing!", loss, loss_percent) } else { "Luckily you carried no gold.".into() });
    }
    if boosted { inv.luck_boost=false; narrative.push("(Your stored luck dissipates.)".into()); }
    inv.save_after_pickup();
    crate::print_event_summary(&title, &before, inv, &non_currency_added, &[]);
    for line in narrative { println!("  • {}", line); }
}

pub fn fight_monster(inv: &mut Inventory) {
    const MONSTERS: &[Monster] = &[
        Monster { name: "Goblin Sneak", strength: 1 }, Monster { name: "Cave Rat", strength: 1 },
        Monster { name: "Skeleton Guard", strength: 2 }, Monster { name: "Orc Marauder", strength: 3 },
        Monster { name: "Ghoul", strength: 4 }, Monster { name: "Ogre Brute", strength: 5 },
        Monster { name: "Wyvern", strength: 6 }, Monster { name: "Vampire Stalker", strength: 7 },
        Monster { name: "Stone Golem", strength: 8 }, Monster { name: "Ancient Lich", strength: 9 },
        Monster { name: "Dragon Wyrm", strength: 10 },
    ];
    crate::print_simple_header("Battle");
    let monster = with_rng(|r| *MONSTERS.choose(r).unwrap());
    println!("⚔️  A wild {} (strength {}) appears!", monster.name, monster.strength);
    let success_chance = (0.85_f64 - (monster.strength as f64 * 0.07)).max(0.05);
    println!("🧮 Success chance: {:>2}%", (success_chance * 100.0).round() as u32);
    let success = with_rng(|r| r.random_bool(success_chance));
    let before = inv.clone();
    if success {
        let (min_gp, max_gp) = {
            let min_gp = (20 * (monster.strength as u32).max(1) / 2).max(10);
            let max_gp = (50 * monster.strength as u32).min(500).max(min_gp + 5);
            (min_gp, max_gp)
        };
        let reward = with_rng(|r| r.random_range(min_gp..=max_gp));
        inv.gold_pieces = inv.gold_pieces.saturating_add(reward);
        inv.save_after_pickup();
        crate::print_event_summary("Victory", &before, inv, &[], &[]);
        println!("🏆 You defeated the {}!", monster.name);
        println!("💰 Loot: {} gold pieces", reward);
    } else {
        if inv.gold_pieces == 0 {
            crate::print_event_summary("Defeat", &before, inv, &[], &[]);
            println!("😣 You were defeated by the {}, but had no gold.", monster.name);
            return;
        }
        let loss_percent = with_rng(|r| r.random_range(5..=10));
        let loss = ((inv.gold_pieces as f64) * (loss_percent as f64 / 100.0)).round() as u32;
        let loss = loss.clamp(1, inv.gold_pieces);
        inv.gold_pieces -= loss;
        inv.save_after_pickup();
        crate::print_event_summary("Defeat", &before, inv, &[], &[]);
        println!("💀 The {} overpowered you! Lost {} gp ({}%).", monster.name, loss, loss_percent);
    }
}

pub const HAGGLE_SUCCESS_CHANCE: f64 = 0.50; // 50% default base chance

#[derive(Clone, Copy, Debug)]
enum Rarity { Common, Uncommon, Rare, Epic, Legendary }

impl Rarity {
    fn price_range_cp(&self) -> std::ops::RangeInclusive<u32> {
        match self {
            // Ranges constrained to overall 5cp .. 500gp (5 .. 50000 cp)
            Rarity::Common => 5..=50,             // 5cp - 5sp
            Rarity::Uncommon => 50..=500,         // 5sp - 5gp
            Rarity::Rare => 500..=5_000,          // 5gp - 50gp
            Rarity::Epic => 5_000..=20_000,       // 50gp - 200gp
            Rarity::Legendary => 20_000..=50_000, // 200gp - 500gp
        }
    }
    fn label(&self) -> &'static str {
        match self { Rarity::Common=>"Common", Rarity::Uncommon=>"Uncommon", Rarity::Rare=>"Rare", Rarity::Epic=>"Epic", Rarity::Legendary=>"Legendary" }
    }
}

pub fn visit_shop(inv: &mut Inventory) {
    loop {
        crate::print_simple_header("Shop");
        println!("🛒 You enter a cluttered shop filled with wares.");
        println!("What would you like to do?");
        let mut options = vec!["Buy Items".to_string(), "Sell Items".to_string(), "Leave Shop".to_string()];
        if inv.items.is_empty() { options[1] = "Sell Items (none to sell)".into(); }
        let choice = Select::new().items(&options).default(0).interact();
        let Ok(choice) = choice else { println!("You step back from the shop."); return; };
        match choice {
            0 => buy_items(inv),
            1 => if !inv.items.is_empty() { sell_items(inv) } else { println!("You have nothing to sell.") },
            2 => { println!("You leave the shop."); return; },
            _ => unreachable!()
        }
    }
}

fn sell_items(inv: &mut Inventory) {
    if inv.items.is_empty() { return; }
    let prices_cp: Vec<u32> = with_rng(|r| inv.items.iter().map(|_| r.random_range(30..=1000)).collect());
    let display: Vec<String> = inv.items.iter().zip(&prices_cp).map(|(it,p)| format!("{} (offers {})", it, format_cp(*p))).collect();
    println!("Select items to sell:");
    let selections = MultiSelect::new().items(&display).interact();
    let selected = match selections { Ok(v) if !v.is_empty() => v, Ok(_) => { println!("Nothing sold."); return; }, Err(e) => { println!("Sale aborted: {}", e); return; } };
    let mut total_cp: u32 = 0; for &idx in &selected { total_cp = total_cp.saturating_add(prices_cp[idx]); }
    println!("💰 Offer: {} for {} item(s).", format_cp(total_cp), selected.len());
    if !Confirm::new().with_prompt("Accept deal?").default(true).interact().unwrap_or(false) { println!("You decline."); return; }
    let before = inv.clone();
    let set: std::collections::HashSet<usize> = selected.iter().copied().collect();
    let mut removed = Vec::new();
    inv.items = inv.items.iter().enumerate().filter_map(|(i,it)| { if set.contains(&i) { removed.push(it.clone()); None } else { Some(it.clone()) } }).collect();
    inv.add_copper(total_cp); inv.save_after_pickup();
    crate::print_event_summary("Shop Sale", &before, inv, &[], &removed);
    println!("✅ Sold {} item(s).", removed.len());
}

fn buy_items(inv: &mut Inventory) {
    // Stock generation (fresh each visit to Buy screen) with rarity tiers
    const STOCK: &[(&str, Rarity)] = &[
        // Mundane (Common / Uncommon)
        ("Rope (50ft)", Rarity::Common), ("Torch", Rarity::Common), ("Lantern", Rarity::Common), ("Oil Flask", Rarity::Common),
        ("Iron Rations", Rarity::Common), ("Waterskin", Rarity::Common), ("Lockpicks", Rarity::Uncommon), ("Bedroll", Rarity::Common),
        ("Backpack", Rarity::Common), ("Shovel", Rarity::Common), ("Grappling Hook", Rarity::Uncommon), ("Hammer & Pitons", Rarity::Common),
        ("Herb Bundle", Rarity::Common), ("Ink & Quill", Rarity::Common), ("Chalk Pouch", Rarity::Common),
        // Magical lower tier
        ("Potion of Healing", Rarity::Uncommon), ("Potion of Invisibility", Rarity::Rare), ("Scroll of Fireball", Rarity::Rare),
        ("Scroll of Shielding", Rarity::Uncommon), ("Ring of Protection", Rarity::Epic), ("Amulet of Light", Rarity::Epic),
        ("Wand of Sparks", Rarity::Uncommon), ("Boots of Silence", Rarity::Rare), ("Cloak of Shadows", Rarity::Epic),
        ("Elixir of Luck", Rarity::Rare), ("Orb of Annihilation Shard", Rarity::Legendary)
    ];
    // Shuffle and take slice
    let mut pool: Vec<(&str, Rarity)> = STOCK.to_vec();
    with_rng(|r| pool.shuffle(r));
    let stock_count = with_rng(|r| r.random_range(6..=10).min(pool.len() as u32)) as usize;
    let stock: Vec<(&str, Rarity)> = pool.into_iter().take(stock_count).collect();
    // Generate prices per rarity
    let prices_cp: Vec<u32> = with_rng(|r| {
        stock.iter().map(|(_, rar)| {
            let range = rar.price_range_cp();
            // Weight slightly toward lower end by sampling two and taking min for higher rarities
            let sample = |r: &mut rand::rngs::SmallRng| r.random_range(*range.start()..=*range.end());
            match rar { Rarity::Epic | Rarity::Legendary => sample(r).min(sample(r)), _ => sample(r) }
        }).collect()
    });
    let display: Vec<String> = stock.iter().enumerate().map(|(i,(name,rar))| {
        format!("{} [{}] ({} )", name, rar.label(), format_cp(prices_cp[i]))
    }).collect();
    println!("Select items to buy (rarity influences price):");
    let selections = MultiSelect::new().items(&display).interact();
    let selected = match selections { Ok(v) if !v.is_empty() => v, Ok(_) => { println!("You buy nothing."); return; }, Err(e) => { println!("Purchase aborted: {}", e); return; } };
    let mut total_cp: u32 = 0; for &idx in &selected { total_cp = total_cp.saturating_add(prices_cp[idx]); }
    println!("🧾 Base total for {} item(s): {}", selected.len(), format_cp(total_cp));
    // Haggle (luck increases success chance and is consumed if present)
    let mut final_cp = total_cp;
    if Confirm::new().with_prompt("Attempt to haggle? (-25% success, +10% failure)").default(false).interact().unwrap_or(false) {
        let mut success_chance = HAGGLE_SUCCESS_CHANCE;
        if inv.luck_boost { success_chance = 0.85; }
        let success = with_rng(|r| r.random_bool(success_chance));
        if success { final_cp = ((final_cp as f64) * 0.75).round() as u32; println!("🤑 Haggle success! New price: {} (chance {:.0}%)", format_cp(final_cp), success_chance*100.0); }
        else { final_cp = ((final_cp as f64) * 1.10).round() as u32; println!("😬 Haggle failed. Merchant raises price to {} (chance {:.0}%)", format_cp(final_cp), success_chance*100.0); }
        if inv.luck_boost { inv.luck_boost = false; println!("✨ Your stored luck is spent in the negotiation."); }
    }
    println!("💰 Final price: {} (you have {})", format_cp(final_cp), format_cp(inv.total_cp()));
    if !Confirm::new().with_prompt("Proceed with purchase?").default(true).interact().unwrap_or(false) { println!("You decide not to buy."); return; }
    if inv.total_cp() < final_cp { println!("Not enough funds."); return; }
    if !inv.try_spend_cp(final_cp) { println!("Payment processing error."); return; }
    let before = inv.clone();
    let mut added: Vec<String> = Vec::new();
    for &idx in &selected { let name = stock[idx].0.to_string(); inv.add_item(&name); added.push(name); }
    inv.save_after_pickup();
    crate::print_event_summary("Shop Purchase", &before, inv, &added, &[]);
    println!("✅ Purchased {} item(s).", added.len());
}

pub fn visit_tavern(inv: &mut Inventory) {
    loop {
        let options = vec![
            format!("Buy Drink ({} sp)", TAVERN_DRINK_COST_SP),
            format!("Buy Food ({} sp)", TAVERN_FOOD_COST_SP),
            format!("Stay The Night ({} gp)", TAVERN_STAY_COST_GP),
            format!("Tip Bartender ({} gp, {}% luck)", TAVERN_TIP_COST_GP, (TAVERN_LUCK_CHANCE*100.0) as u32),
            format!("Flirt With Barmaid ({} gp, {}% kiss for luck)", TAVERN_FLIRT_COST_GP, (TAVERN_FLIRT_KISS_CHANCE*100.0) as u32),
            "Leave Tavern".to_string(),
        ];
        crate::print_simple_header("Tavern"); println!("🍺 You enter a bustling tavern."); if inv.luck_boost { println!("✨ Stored luck awaits."); }
        let choice = Select::new().items(&options).default(0).interact(); let Ok(choice)=choice else { println!("You back out of the tavern."); return; };
        match choice { 0=>buy_drink(inv),1=>buy_food(inv),2=>stay_night(inv),3=>tip_bartender(inv),4=>flirt_barmaid(inv),5=>{println!("You leave the tavern."); return;}, _=>unreachable!() }
    }
}

fn buy_drink(inv: &mut Inventory) {
    println!("Drink costs {} ({} sp).", format_cp(TAVERN_DRINK_COST_SP * 10), TAVERN_DRINK_COST_SP);
    if !Confirm::new().with_prompt("Buy drink?").default(true).interact().unwrap_or(false) { return; }
    if !inv.try_spend_cp(TAVERN_DRINK_COST_SP * 10) { println!("Not enough coin."); return; }
    println!("🥃 You savor a drink.");
    inv.save_after_pickup();
}

fn buy_food(inv: &mut Inventory) {
    println!("Food costs {} ({} sp).", format_cp(TAVERN_FOOD_COST_SP * 10), TAVERN_FOOD_COST_SP);
    if !Confirm::new().with_prompt("Buy food?").default(true).interact().unwrap_or(false) { return; }
    if !inv.try_spend_cp(TAVERN_FOOD_COST_SP * 10) { println!("Can't afford meal."); return; }
    println!("🍖 Warm meal restores you.");
    inv.save_after_pickup();
}

fn stay_night(inv: &mut Inventory) {
    println!("Room costs {} ({} gp).", format_cp(TAVERN_STAY_COST_GP * 100), TAVERN_STAY_COST_GP);
    if !Confirm::new().with_prompt("Pay for room?").default(true).interact().unwrap_or(false) { return; }
    if !inv.try_spend_cp(TAVERN_STAY_COST_GP * 100) { println!("Can't afford room."); return; }
    println!("🛏️  You rest deeply (benefit TBD).");
    inv.save_after_pickup();
}

fn tip_bartender(inv: &mut Inventory) {
    if inv.luck_boost { println!("Luck already stored."); return; }
    println!(
        "Tip costs {} ({} gp) for {}% luck chance.",
        format_cp(TAVERN_TIP_COST_GP * 100),
        TAVERN_TIP_COST_GP,
        (TAVERN_LUCK_CHANCE * 100.0) as u32
    );
    if !Confirm::new().with_prompt("Leave tip?").default(true).interact().unwrap_or(false) { return; }
    if !inv.try_spend_cp(TAVERN_TIP_COST_GP * 100) { println!("Need more gold."); return; }
    if with_rng(|r| r.random_bool(TAVERN_LUCK_CHANCE)) {
        inv.luck_boost = true; println!("🍀 Luck boon gained for next pickpocket.");
    } else { println!("🍂 No luck this time."); }
    inv.save_after_pickup();
}

fn flirt_barmaid(inv: &mut Inventory) {
    use dialoguer::Confirm;
    println!("Flirting costs {} ({} gp).", format_cp(TAVERN_FLIRT_COST_GP * 100), TAVERN_FLIRT_COST_GP);
    if !Confirm::new().with_prompt("Attempt to flirt?").default(true).interact().unwrap_or(false) { return; }
    if !inv.try_spend_cp(TAVERN_FLIRT_COST_GP * 100) { println!("You can't afford her attention right now."); return; }
    if with_rng(|r| r.random_bool(TAVERN_FLIRT_KISS_CHANCE)) {
        if !inv.luck_boost { inv.luck_boost = true; println!("💋 The barmaid gives you a quick kiss. You feel luck swirling for your next pickpocket."); }
        else { println!("💋 Another quick kiss, though you already feel lucky."); }
    } else { println!("🙂 She laughs and shakes her head politely. Maybe next time."); }
    inv.save_after_pickup();
}