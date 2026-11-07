//! Reconnect with jittered exponential backoff (`docs/protocol.md` §6.7–§6.8).

/// Backoff policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Backoff {
    /// First delay (ms).
    pub initial_ms: u32,
    /// Maximum delay (ms).
    pub max_ms: u32,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            initial_ms: 250,
            max_ms: 30_000,
        }
    }
}

impl Backoff {
    /// Delay before reconnect attempt `attempt` (0-based), given a random
    /// value in `[0, 1)` for jitter and an optional server hint.
    #[must_use]
    pub fn delay_ms(&self, _attempt: u32, _jitter: f64, _retry_after_ms: Option<u32>) -> u32 {
        // TODO(M3, #7): full-jitter backoff honouring ServerRestarting (#9).
        todo!("reconnect backoff (M3, #7)")
    }
}
