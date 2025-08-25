use dungeon_core::{inventory::Inventory, loot::parse_and_format_loot_cached, apply_pickpocket_penalty, actions::{TAVERN_DRINK_COST_SP, TAVERN_FOOD_COST_SP, TAVERN_STAY_COST_GP, TAVERN_TIP_COST_GP, TAVERN_FLIRT_COST_GP}};

// Local lightweight harness (mirrors earlier inline tests)
#[derive(Debug)]
enum TavernAction { Drink, Food, Stay, Tip { luck_roll_success: bool }, Flirt { kiss_success: bool } }

fn simulate_pickpocket(
    inventory: &mut Inventory,
    loot_items: &[String],
    event_trigger: bool,
    success: bool,
    selected_loot_index: Option<usize>,
    loss_percent: u32,
) -> (String, Vec<String>) {
    let mut added = Vec::new();
    if event_trigger {
        inventory.add_item("1000 gp");
        added.push("1000 gp".into());
        return ("event".into(), added);
    }
    if success {
        if let Some(i) = selected_loot_index && let Some(desc) = loot_items.get(i) {
            let (its, _) = parse_and_format_loot_cached(desc);
            for it in its.iter() { inventory.add_item(it); added.push(it.clone()); }
        }
        ("success".into(), added)
    } else {
        let _ = apply_pickpocket_penalty(&mut inventory.gold_pieces, loss_percent);
        ("failure".into(), added)
    }
}

fn simulate_fight(inventory: &mut Inventory, success: bool, reward_gp: u32, loss_percent: u32) -> &'static str {
    if success { inventory.gold_pieces = inventory.gold_pieces.saturating_add(reward_gp); "victory" } else { if inventory.gold_pieces > 0 { let _ = apply_pickpocket_penalty(&mut inventory.gold_pieces, loss_percent.max(1)); } "defeat" }
}

fn simulate_shop_sale(inventory: &mut Inventory, selected_indices: &[usize], prices_cp: &[u32], accept: bool) -> u32 {
    if !accept { return 0; }
    let mut total_cp = 0u32; let mut removed = Vec::new();
    for (idx, item) in inventory.items.iter().enumerate() { if selected_indices.contains(&idx) { removed.push(item.clone()); if let Some(p) = prices_cp.get(idx) { total_cp = total_cp.saturating_add(*p); } } }
    inventory.items = inventory.items.iter().enumerate().filter_map(|(i,it)| if selected_indices.contains(&i) { None } else { Some(it.clone()) }).collect();
    inventory.add_copper(total_cp); removed.len() as u32
}

fn simulate_tavern_action(inventory: &mut Inventory, action: TavernAction, confirm: bool) -> bool {
    if !confirm { return false; }
    match action {
        TavernAction::Drink => inventory.try_spend_cp(TAVERN_DRINK_COST_SP * 10),
        TavernAction::Food => inventory.try_spend_cp(TAVERN_FOOD_COST_SP * 10),
        TavernAction::Stay => inventory.try_spend_cp(TAVERN_STAY_COST_GP * 100),
        TavernAction::Tip { luck_roll_success } => {
            if inventory.luck_boost { return false; }
            if !inventory.try_spend_cp(TAVERN_TIP_COST_GP * 100) { return false; }
            if luck_roll_success { inventory.luck_boost = true; }
            true
        }
        TavernAction::Flirt { kiss_success } => {
            if !inventory.try_spend_cp(TAVERN_FLIRT_COST_GP * 100) { return false; }
            if kiss_success && !inventory.luck_boost { inventory.luck_boost = true; }
            true
        }
    }
}

#[test]
fn pickpocket_event_awards_1000_gp() {
    let mut inv = Inventory::new(); let loot = vec!["3 gp and a silver ring".into()]; simulate_pickpocket(&mut inv, &loot, true, false, None, 7); assert_eq!(inv.gold_pieces, 1000);
}

#[test]
fn pickpocket_failure_penalty_applied() {
    let mut inv = Inventory::new(); inv.gold_pieces = 200; simulate_pickpocket(&mut inv, &[], false, false, None, 10); assert!(inv.gold_pieces <= 190 && inv.gold_pieces >= 180);
}

#[test]
fn pickpocket_success_adds_items() {
    let mut inv = Inventory::new(); let loot = vec!["5 gp, 2 sp and a ruby".into()]; let (_k, added) = simulate_pickpocket(&mut inv, &loot, false, true, Some(0), 7); assert!(!added.is_empty()); assert!(inv.gold_pieces >= 5); assert!(inv.silver_pieces >= 2);
}

#[test]
fn fight_victory_adds_reward() {
    let mut inv = Inventory::new(); inv.gold_pieces = 10; let outcome = simulate_fight(&mut inv, true, 120, 7); assert_eq!(outcome, "victory"); assert_eq!(inv.gold_pieces, 130);
}

#[test]
fn fight_defeat_loses_gold() {
    let mut inv = Inventory::new(); inv.gold_pieces = 100; let outcome = simulate_fight(&mut inv, false, 0, 8); assert_eq!(outcome, "defeat"); assert!(inv.gold_pieces < 100);
}

#[test]
fn shop_sale_converts_items_to_currency() {
    let mut inv = Inventory::new(); inv.items = vec!["Rusty Dagger".into(), "Silver Ring".into(), "Herb Bundle".into()]; let prices = vec![100,230,50]; let sold = simulate_shop_sale(&mut inv, &[0,2], &prices, true); assert_eq!(sold, 2); assert_eq!(inv.items, vec!["Silver Ring"]); assert_eq!(inv.gold_pieces, 1); assert_eq!(inv.silver_pieces, 5);
}

#[test]
fn tavern_drink_and_tip_flow() {
    let mut inv = Inventory::new(); inv.gold_pieces = 10; let drink = simulate_tavern_action(&mut inv, TavernAction::Drink, true); assert!(drink); let pre_tip_gold = inv.gold_pieces; let tip_ok = simulate_tavern_action(&mut inv, TavernAction::Tip { luck_roll_success: true }, true); assert!(tip_ok); assert!(inv.gold_pieces < pre_tip_gold); assert!(inv.luck_boost);
}

#[test]
fn tavern_food_flow() {
    let mut inv = Inventory::new(); inv.gold_pieces = 2; // enough for food (12 sp = 1g2s)
    let ok = simulate_tavern_action(&mut inv, TavernAction::Food, true); assert!(ok);
    assert!(inv.total_cp() < 200); // spent something
}

#[test]
fn tavern_stay_flow() {
    let mut inv = Inventory::new(); inv.gold_pieces = 5; let ok = simulate_tavern_action(&mut inv, TavernAction::Stay, true); assert!(ok); assert!(inv.gold_pieces < 5);
}

#[test]
fn tavern_flirt_flow_kiss_grants_luck() {
    let mut inv = Inventory::new(); inv.gold_pieces = 20;
    let ok = simulate_tavern_action(&mut inv, TavernAction::Flirt { kiss_success: true }, true); assert!(ok);
    assert!(inv.luck_boost, "Successful kiss should grant luck");
}
