//! Time and position primitives shared by the wire format and the clock layer.
//!
//! All timestamps in this module are **monotonic** (ADR-0005). Wall-clock time
//! never appears here.

use core::fmt;

use serde::{Deserialize, Serialize};

/// A point on the **room clock**, in nanoseconds since an arbitrary,
/// per-epoch origin chosen by the server.
///
/// The room clock is the server's monotonic clock. It is only guaranteed to
/// be continuous within one connection: a server restart or room migration
/// may change its origin (`docs/protocol.md` §4.1).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct HostTime(pub u64);

impl HostTime {
    /// Nanoseconds since the room clock origin.
    #[must_use]
    pub const fn as_nanos(self) -> u64 {
        self.0
    }
}

/// A media position in milliseconds from the start of the current track.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct PosMs(pub u64);

/// Session epoch. Incremented on every authority transfer and every server
/// restart (ADR-0004). Messages carrying a stale epoch MUST be discarded.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Epoch(pub u64);

/// Session revision. Incremented on every session state change (ADR-0004).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Rev(pub u64);

/// Playback rate as a fixed-point number in **millionths** (`1_000_000` = 1.0×).
///
/// A fixed-point representation keeps the encoding byte-exact across
/// languages and platforms (no floating point on the wire). A rate of `0`
/// means the timeline is stopped (paused or suspending).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Rate(pub u32);

impl Rate {
    /// Fixed-point scale: `Rate(SCALE)` is normal speed.
    pub const SCALE: u32 = 1_000_000;
    /// Stopped timeline.
    pub const ZERO: Rate = Rate(0);
    /// Normal speed (1.0×).
    pub const NORMAL: Rate = Rate(Self::SCALE);

    /// The rate as millionths.
    #[must_use]
    pub const fn as_micros(self) -> u32 {
        self.0
    }

    /// Whether the timeline is stopped.
    #[must_use]
    pub const fn is_stopped(self) -> bool {
        self.0 == 0
    }
}

impl Default for Rate {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl fmt::Debug for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / Self::SCALE;
        let frac = self.0 % Self::SCALE;
        write!(f, "Rate({whole}.{frac:06})")
    }
}
