//! Monotonic local clock (ADR-0005).

use core::fmt;

/// A cheap, monotonic, nanosecond clock for the timeline and probe paths.
///
/// Backed by [`quanta`] (TSC where available, the OS monotonic clock
/// otherwise). Readings are nanoseconds since this clock was created; they
/// never jump backwards and are unaffected by NTP steps, DST, or the user
/// changing the system time. `SystemTime` MUST NOT be used instead
/// (ADR-0005).
///
/// Clones share the same origin, so readings from clones are comparable.
#[derive(Clone)]
pub struct MonotonicClock {
    clock: quanta::Clock,
    origin: quanta::Instant,
}

impl MonotonicClock {
    /// Creates a clock whose origin (reading `0`) is now.
    #[must_use]
    pub fn new() -> Self {
        let clock = quanta::Clock::new();
        let origin = clock.now();
        Self { clock, origin }
    }

    /// Nanoseconds elapsed since the clock was created. Saturates at
    /// `u64::MAX` (after ~584 years).
    #[must_use]
    pub fn now_ns(&self) -> u64 {
        let elapsed = self.clock.now().saturating_duration_since(self.origin);
        u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX)
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for MonotonicClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonotonicClock")
            .field("now_ns", &self.now_ns())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_monotonic() {
        let clock = MonotonicClock::new();
        let mut prev = clock.now_ns();
        for _ in 0..10_000 {
            let now = clock.now_ns();
            assert!(now >= prev);
            prev = now;
        }
    }

    #[test]
    fn clones_share_origin() {
        let a = MonotonicClock::new();
        let b = a.clone();
        let ta = a.now_ns();
        let tb = b.now_ns();
        assert!(tb >= ta);
    }
}
