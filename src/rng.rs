use rand::rngs::SmallRng;
use rand::SeedableRng; // for from_rng
use std::cell::RefCell;

thread_local! {
    static TL_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_rng(&mut rand::rng()));
}

/// Execute closure with a fast thread-local SmallRng.
pub fn with_rng<F, T>(f: F) -> T
where
    F: FnOnce(&mut SmallRng) -> T,
{
    TL_RNG.with(|r| f(&mut r.borrow_mut()))
}