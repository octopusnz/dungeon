use crate::inventory::Inventory;

#[derive(Debug, Clone, Copy)]
pub enum MainAction {
    PickPocket,
    Inventory,
    Shop,
    Fight,
    Tavern,
    Exit,
}

pub fn prompt_main_action() -> MainAction {
    use std::io::{self, Write};
    println!("\n===== Actions =====");
    println!("[P]ickpocket  [I]nventory  [S]hop  [F]ight  [T]avern  E[x]it / [Q]uit");
    print!("Enter choice: ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return MainAction::Exit;
    }
    let input = line.trim();
    if input.is_empty() {
        return MainAction::PickPocket;
    }
    match input.chars().next().unwrap().to_ascii_lowercase() {
        'p' => MainAction::PickPocket,
        'i' => MainAction::Inventory,
        's' => MainAction::Shop,
        'f' => MainAction::Fight,
        't' => MainAction::Tavern,
        'x' | 'q' | 'e' => MainAction::Exit,
        other => {
            println!("Unrecognized option '{}'. (P/I/S/F/T/Q)", other);
            MainAction::PickPocket
        }
    }
}

pub fn print_simple_header(title: &str) {
    println!("\n──── {} ────", title);
}

pub fn print_event_summary(
    title: &str,
    before: &Inventory,
    after: &Inventory,
    items_added: &[String],
    items_removed: &[String],
) {
    print_simple_header(title);
    let dg = after.gold_pieces as i64 - before.gold_pieces as i64;
    let ds = after.silver_pieces as i64 - before.silver_pieces as i64;
    let dc = after.copper_pieces as i64 - before.copper_pieces as i64;
    let mut deltas = Vec::new();
    if dg != 0 {
        deltas.push(format_delta(dg, "gp"));
    }
    if ds != 0 {
        deltas.push(format_delta(ds, "sp"));
    }
    if dc != 0 {
        deltas.push(format_delta(dc, "cp"));
    }
    if !deltas.is_empty() {
        println!("💱 Currency change: {}", deltas.join(", "));
    }
    if !items_added.is_empty() {
        println!("➕ Items gained: {}", summarize_items(items_added));
    }
    if !items_removed.is_empty() {
        println!("➖ Items lost: {}", summarize_items(items_removed));
    }
    println!(
        "🏦 Holdings: {} gp, {} sp, {} cp",
        after.gold_pieces, after.silver_pieces, after.copper_pieces
    );
}

fn format_delta(delta: i64, unit: &str) -> String {
    if delta > 0 {
        format!("+{} {}", delta, unit)
    } else {
        format!("{} {}", delta, unit)
    }
}
fn summarize_items(items: &[String]) -> String {
    if items.len() <= 5 {
        items.join(", ")
    } else {
        format!("{} items", items.len())
    }
}
