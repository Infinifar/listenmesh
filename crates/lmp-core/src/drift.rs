//! Drift control ladder (`docs/protocol.md` §7.1, `docs/media-clock.md` §4).
//!
//! **Status: skeleton.** Implementation is tracked in
//! [#3](https://github.com/Infinifar/listenmesh/issues/3).

/// Thresholds of the drift ladder. Audio is tighter than video.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DriftThresholds {
    /// Below this absolute error (ms), correct by slewing the rate.
    pub slew_ms: u32,
    /// At or above this absolute error (ms), hard seek immediately.
    pub seek_ms: u32,
    /// Consecutive out-of-band measurements required before seeking in the
    /// band between `slew_ms` and `seek_ms`.
    pub strikes: u8,
    /// Maximum rate deviation, in millionths (5 000 = 0.5 %).
    pub max_rate_delta_micros: u32,
}

impl DriftThresholds {
    /// Defaults for audio: 12 ms / 220 ms, 3 strikes, ±0.5 %.
    pub const AUDIO: Self = Self {
        slew_ms: 12,
        seek_ms: 220,
        strikes: 3,
        max_rate_delta_micros: 5_000,
    };
    /// Defaults for video: 40 ms / 300 ms, 3 strikes, ±0.5 %.
    pub const VIDEO: Self = Self {
        slew_ms: 40,
        seek_ms: 300,
        strikes: 3,
        max_rate_delta_micros: 5_000,
    };
}

/// What the media pipeline should do after a drift measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Correction {
    /// Within tolerance; play at nominal rate.
    None,
    /// Adjust the playback rate by this many millionths (bounded by
    /// [`DriftThresholds::max_rate_delta_micros`]).
    Slew {
        /// Signed rate delta in millionths.
        rate_delta_micros: i32,
    },
    /// Jump to the expected position.
    Seek,
}

/// Drift controller state machine.
#[derive(Clone, Debug)]
pub struct DriftController {
    thresholds: DriftThresholds,
    strikes: u8,
}

impl DriftController {
    /// Creates a controller with the given thresholds.
    #[must_use]
    pub const fn new(thresholds: DriftThresholds) -> Self {
        Self {
            thresholds,
            strikes: 0,
        }
    }

    /// The thresholds in use.
    #[must_use]
    pub const fn thresholds(&self) -> &DriftThresholds {
        &self.thresholds
    }

    /// Consecutive strikes recorded so far.
    #[must_use]
    pub const fn strikes(&self) -> u8 {
        self.strikes
    }

    /// Feeds one measurement (`local − expected`, ms) and returns the correction.
    pub fn update(&mut self, _error_ms: i64) -> Correction {
        // TODO(M1, #3): slew band, strike counting, seek band; never seek while
        // the room is Suspending.
        todo!("drift controller (M1, #3)")
    }
}
