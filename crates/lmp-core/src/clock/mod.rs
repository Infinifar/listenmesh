//! L1 clock layer: monotonic local time and room-clock estimation.
//!
//! See `docs/clock-sync.md` for the algorithm and its derivation, and
//! `docs/protocol.md` §4 for the normative requirements.
//!
//! - [`MonotonicClock`]: the only clock allowed on the timeline path
//!   (ADR-0005). Backed by [`quanta`].
//! - [`ClockSample`]: one NTP-style four-timestamp exchange.
//! - [`ClockEstimator`]: min-RTT filtering plus least-squares regression that
//!   yields the room-clock offset and the frequency skew (ppm).
//!
//! Every function in this module is pure with respect to time: callers pass
//! timestamps in, so the estimator is fully deterministic and testable with
//! synthetic sequences.

mod estimator;
mod monotonic;

#[cfg(test)]
mod tests;

pub use estimator::{ClockEstimator, ClockSample, Estimate, EstimatorConfig, SampleError};
pub use monotonic::MonotonicClock;
