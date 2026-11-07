//! Adaptive probe scheduler (`docs/protocol.md` §4.4, `docs/clock-sync.md` §5).

/// Probe scheduling phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbePhase {
    /// Cold start: 10 probes at 10 Hz.
    ColdStart {
        /// Probes sent so far.
        sent: u8,
    },
    /// Quiet network: 1 Hz.
    Steady,
    /// Jitter detected: up to 10 Hz.
    Agitated,
}

/// Decides when the next probe is due.
#[derive(Clone, Debug)]
pub struct ProbeScheduler {
    phase: ProbePhase,
}

impl ProbeScheduler {
    /// A scheduler in the cold-start phase.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            phase: ProbePhase::ColdStart { sent: 0 },
        }
    }

    /// Current phase.
    #[must_use]
    pub const fn phase(&self) -> ProbePhase {
        self.phase
    }

    /// Local monotonic time (ns) at which the next probe should be sent.
    #[must_use]
    pub fn next_probe_at(&self, _now_ns: u64) -> u64 {
        // TODO(M1, #2): 10 Hz burst → 1 Hz, jitter-driven escalation with hysteresis.
        todo!("probe scheduler (M1, #2)")
    }
}

impl Default for ProbeScheduler {
    fn default() -> Self {
        Self::new()
    }
}
