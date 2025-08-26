pub mod actions;
pub mod inventory;
pub mod loot;
pub mod rng;
pub mod ui;
#[cfg(feature = "wasm")]
pub mod wasm_api;

// Use a smaller allocator for wasm builds to reduce code size
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub use ui::{print_event_summary, print_simple_header};

// Shared helper
pub fn apply_pickpocket_penalty(gold_pieces: &mut u32, loss_percent: u32) -> u32 {
    if *gold_pieces == 0 || loss_percent == 0 {
        return 0;
    }
    let raw_loss = ((*gold_pieces as f64) * (loss_percent as f64 / 100.0)).round() as u32;
    let loss = raw_loss.clamp(1, *gold_pieces);
    *gold_pieces -= loss;
    loss
}

#[cfg(test)]
mod tests {
    use super::apply_pickpocket_penalty;

    #[test]
    fn penalty_basic() {
        let mut g = 10;
        let lost = apply_pickpocket_penalty(&mut g, 10);
        assert_eq!(lost, 1);
        assert_eq!(g, 9);
    }
}
