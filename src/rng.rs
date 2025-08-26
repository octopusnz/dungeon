use rand::SeedableRng; // for from_entropy
use rand::rngs::SmallRng;
use std::cell::RefCell;

thread_local! {
    static TL_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_entropy());
}

/// Execute closure with a fast thread-local SmallRng.
pub fn with_rng<F, T>(f: F) -> T
where
    F: FnOnce(&mut SmallRng) -> T,
{
    TL_RNG.with(|r| f(&mut r.borrow_mut()))
}

/// Reseed the thread-local RNG for deterministic testing.
/// Safe to call multiple times; affects subsequent calls to `with_rng` in the current thread.
pub fn reseed(seed: u64) {
    TL_RNG.with(|r| *r.borrow_mut() = SmallRng::seed_from_u64(seed));
}
