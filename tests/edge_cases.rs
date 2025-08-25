use dungeon_core::{inventory::{Inventory, format_cp}, loot::parse_loot_into_items, apply_pickpocket_penalty, actions::{TAVERN_TIP_COST_GP}};

// Helper harness snippet for tavern tip logic (non-interactive)
fn simulate_tip(inventory: &mut Inventory, luck_roll_success: bool) -> bool {
    // Emulate internal tavern logic cost & luck
    if inventory.luck_boost { return false; }
    let cost_cp = TAVERN_TIP_COST_GP * 100;
    if !inventory.try_spend_cp(cost_cp) { return false; }
    if luck_roll_success { inventory.luck_boost = true; }
    true
}

#[test]
fn add_copper_converts_to_higher_denominations() {
    let mut inv = Inventory::new();
    inv.add_copper(275); // 2 gp (200), 7 sp (70), 5 cp
    assert_eq!((inv.gold_pieces, inv.silver_pieces, inv.copper_pieces), (2,7,5));
}

#[test]
fn try_spend_cp_insufficient_funds() {
    let mut inv = Inventory::new();
    inv.add_copper(90); // 0g 9s 0c
    let before = inv.clone();
    assert!(!inv.try_spend_cp(200)); // not enough
    assert_eq!(inv.total_cp(), before.total_cp());
}

#[test]
fn try_spend_cp_exact_match() {
    let mut inv = Inventory::new();
    inv.add_copper(150); // 1g 5s 0c
    assert!(inv.try_spend_cp(150));
    assert_eq!(inv.total_cp(), 0);
}

#[test]
fn format_cp_various_cases() {
    assert_eq!(format_cp(0), "0 cp");
    assert_eq!(format_cp(7), "7 cp");
    assert_eq!(format_cp(10), "1 sp");
    assert_eq!(format_cp(15), "1 sp 5 cp");
    assert_eq!(format_cp(100), "1 gp");
    assert_eq!(format_cp(115), "1 gp 1 sp 5 cp");
}

#[test]
fn penalty_zero_percent_no_change() {
    let mut g = 50; let lost = apply_pickpocket_penalty(&mut g, 0); assert_eq!(lost, 0); assert_eq!(g, 50);
}

#[test]
fn parse_loot_mixed_items() {
    let items = parse_loot_into_items("3 gp, 2 sp and an iron key");
    // Should include the numeric currency entries and the item key
    assert!(items.iter().any(|s| s.contains("3 gp")));
    assert!(items.iter().any(|s| s.contains("2 sp")));
    assert!(items.iter().any(|s| s.contains("Iron key")));
}

#[test]
fn tavern_tip_second_attempt_blocked() {
    let mut inv = Inventory::new();
    inv.gold_pieces = 10; // plenty for tips
    let first = simulate_tip(&mut inv, true);
    assert!(first && inv.luck_boost);
    let gold_after_first = inv.gold_pieces;
    let second = simulate_tip(&mut inv, true);
    assert!(!second, "Second tip should be blocked when luck already stored");
    assert_eq!(inv.gold_pieces, gold_after_first, "Gold should not change on blocked second tip");
}
