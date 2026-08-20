use std::num::NonZeroU32;
use std::time::Duration;

use governor::{Quota, RateLimiter};

pub type GlobalLimiter = RateLimiter<
    governor::state::NotKeyed,
    governor::state::InMemoryState,
    governor::clock::DefaultClock,
    governor::middleware::NoOpMiddleware,
>;

pub type PerRecipientLimiter = RateLimiter<
    String,
    governor::state::keyed::DashMapStateStore<String>,
    governor::clock::DefaultClock,
    governor::middleware::NoOpMiddleware,
>;

const MIN_SEND_INTERVAL: Duration = Duration::from_secs(1);

#[must_use]
pub fn build_global(max_per_minute: u32) -> GlobalLimiter {
    let n = NonZeroU32::new(max_per_minute).unwrap_or(NonZeroU32::MIN);
    RateLimiter::direct(Quota::per_minute(n))
}

/// # Panics
///
/// Panics only if `Duration::from_secs(1)` is rejected by `Quota::with_period`,
/// which cannot happen.
#[must_use]
pub fn build_per_recipient(cooldown_secs: u64) -> PerRecipientLimiter {
    let period = Duration::from_secs(cooldown_secs.max(1));
    RateLimiter::keyed(
        Quota::with_period(period)
            .unwrap_or_else(|| Quota::with_period(MIN_SEND_INTERVAL).unwrap()),
    )
}
