use dungeon_core::{
    actions::{Rarity, fight_monster_outcome, pick_pocket},
    inventory::Inventory,
    rng::reseed,
}; // FightOutcome now includes hp fields; test still focuses on reward/loss determinism

// Helper to run pick_pocket deterministically and return inventory diff
fn run_pick(inv: &mut Inventory, loot: &[&str]) {
    let items: Vec<String> = loot.iter().map(|s| s.to_string()).collect();
    pick_pocket(inv, &items);
}

#[test]
fn rarity_price_ranges_are_consistent() {
    // Just assert the inclusive bounds ordering for sanity
    let ranges = [
        (Rarity::Common, 5, 50),
        (Rarity::Uncommon, 50, 500),
        (Rarity::Rare, 500, 5_000),
        (Rarity::Epic, 5_000, 20_000),
        (Rarity::Legendary, 20_000, 50_000),
    ];
    for (rar, lo, hi) in ranges {
        let r = rar.price_range_cp();
        assert_eq!((*r.start(), *r.end()), (lo, hi));
    }
}

#[test]
fn fight_monster_outcome_deterministic_sequence() {
    // Reseed so that outcome sequence is reproducible
    reseed(42);
    let mut inv = Inventory::new();
    inv.gold_pieces = 100;
    // Run several fights capturing results; ensure deterministic given same seed
    let mut rewards = Vec::new();
    let mut losses = Vec::new();
    for _ in 0..5 {
        let o = fight_monster_outcome(&mut inv);
        if o.victory {
            rewards.push(o.reward_gp);
        } else {
            losses.push(o.loss_gp);
        }
    }
    // Reset and repeat with same seed and expect identical vectors
    let mut inv2 = Inventory::new();
    inv2.gold_pieces = 100;
    reseed(42);
    let mut rewards2 = Vec::new();
    let mut losses2 = Vec::new();
    for _ in 0..5 {
        let o = fight_monster_outcome(&mut inv2);
        if o.victory {
            rewards2.push(o.reward_gp);
        } else {
            losses2.push(o.loss_gp);
        }
    }
    assert_eq!(
        rewards, rewards2,
        "Victory reward sequence should match with same seed"
    );
    assert_eq!(losses, losses2, "Loss sequence should match with same seed");
}

#[test]
fn pickpocket_event_trigger_with_luck_consumes_flag() {
    // High event chance when luck present (90%). Deterministic seed ensures path.
    let mut inv = Inventory::new();
    inv.luck_boost = true;
    reseed(7);
    run_pick(&mut inv, &["3 gp and a silver ring"]);
    assert!(
        !inv.luck_boost,
        "Luck should be consumed after pickpocket attempt"
    );
}

#[test]
fn pickpocket_without_luck_lower_event_rate_path() {
    let mut inv = Inventory::new();
    reseed(999); // arbitrary
    run_pick(&mut inv, &["5 gp and a ruby"]);
    // We can't assert exact branch, but ensure inventory save logic didn't grant spurious luck
    assert!(!inv.luck_boost);
}
