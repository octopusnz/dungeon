use dungeon::{loot::parse_and_format_loot_cached, apply_pickpocket_penalty};

#[test]
fn penalty_zero_gold() {
    let mut g = 0; let lost = apply_pickpocket_penalty(&mut g, 7); assert_eq!(lost, 0); assert_eq!(g, 0);
}

#[test]
fn penalty_minimum_one() {
    let mut g = 5; let lost = apply_pickpocket_penalty(&mut g, 1); assert_eq!(lost, 1); assert_eq!(g, 4);
}

#[test]
fn penalty_percentage_rounding() {
    let mut g = 137; let lost = apply_pickpocket_penalty(&mut g, 7); assert_eq!(lost, 10); assert_eq!(g, 127);
}

#[test]
fn penalty_full_edge() {
    let mut g = 1; let lost = apply_pickpocket_penalty(&mut g, 50); assert_eq!(lost, 1); assert_eq!(g, 0);
}

#[test]
fn loot_parse_and_cache_basic() {
    let desc = "3 gp and a silver ring";
    let (items, formatted) = parse_and_format_loot_cached(desc);
    assert!(items.iter().any(|i| i.contains("gp")));
    assert!(formatted.contains("gold piece"));
    // second call hits cache (cannot directly assert, but should return same data)
    let (items2, formatted2) = parse_and_format_loot_cached(desc);
    assert_eq!(items, items2);
    assert_eq!(formatted, formatted2);
}
