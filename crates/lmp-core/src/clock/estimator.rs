//! Room-clock offset and skew estimation (`docs/clock-sync.md` §2–§4).

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::message::ProbeReply;
use crate::time::HostTime;

/// One NTP-style four-timestamp exchange, reduced to the quantities the
/// estimator needs.
///
/// ```text
/// client  t0 ──Probe──▶ t1  server
///         t3 ◀──Reply── t2
///
/// offset θ = ((t1 − t0) + (t2 − t3)) / 2      (room − local)
/// rtt    δ = (t3 − t0) − (t2 − t1)
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockSample {
    /// Local monotonic time at the midpoint of the exchange, `t0 + (t3 − t0) / 2` (ns).
    pub local_mid: u64,
    /// Estimated `room − local` offset at `local_mid` (ns), rounded toward −∞.
    pub offset_ns: i64,
    /// Round-trip network delay, excluding server processing time (ns).
    pub rtt_ns: u64,
}

/// Why a four-timestamp exchange was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SampleError {
    /// `t3 < t0`: the reply was received before the probe was sent.
    #[error("client receive time precedes client transmit time")]
    ClientNonCausal,
    /// `t2 < t1`: the server replied before it received the probe.
    #[error("server transmit time precedes server receive time")]
    ServerNonCausal,
    /// The server claims to have spent longer than the whole round trip.
    #[error("negative round-trip delay")]
    NegativeDelay,
    /// The round trip exceeds [`EstimatorConfig::max_rtt_ns`].
    #[error("round-trip delay of {rtt_ns} ns exceeds the limit of {max_ns} ns")]
    RttTooLarge {
        /// Measured round-trip delay.
        rtt_ns: u64,
        /// Configured limit.
        max_ns: u64,
    },
    /// The offset does not fit in an `i64` (clocks ~292 years apart).
    #[error("offset out of range")]
    OffsetOutOfRange,
}

impl ClockSample {
    /// Builds a sample from the four timestamps of one probe exchange.
    ///
    /// `t0`/`t3` are local monotonic nanoseconds ([`super::MonotonicClock`]);
    /// `t1`/`t2` are room-clock times from the [`ProbeReply`].
    ///
    /// # Errors
    ///
    /// Rejects exchanges that violate causality or have a negative delay.
    pub fn from_timestamps(
        t0: u64,
        t1: HostTime,
        t2: HostTime,
        t3: u64,
    ) -> Result<Self, SampleError> {
        let client_span = t3.checked_sub(t0).ok_or(SampleError::ClientNonCausal)?;
        let server_span = t2.0.checked_sub(t1.0).ok_or(SampleError::ServerNonCausal)?;
        let rtt_ns = client_span
            .checked_sub(server_span)
            .ok_or(SampleError::NegativeDelay)?;
        let twice = (i128::from(t1.0) - i128::from(t0)) + (i128::from(t2.0) - i128::from(t3));
        let offset_ns =
            i64::try_from(twice.div_euclid(2)).map_err(|_| SampleError::OffsetOutOfRange)?;
        Ok(Self {
            local_mid: t0 + client_span / 2,
            offset_ns,
            rtt_ns,
        })
    }
}

/// Tuning of the [`ClockEstimator`]. The defaults are the values specified in
/// `docs/clock-sync.md` §4.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EstimatorConfig {
    /// Number of most recent samples kept (sliding window).
    pub capacity: usize,
    /// Keep the lowest-RTT `1 / keep_denominator` of the window (3 → one third).
    pub keep_denominator: usize,
    /// Never keep fewer than this many samples (if available).
    pub min_kept: usize,
    /// Minimum number of kept samples before skew is estimated.
    pub min_skew_samples: usize,
    /// Minimum local-time span of the kept samples before skew is estimated (ns).
    pub min_skew_span_ns: u64,
    /// Samples with a larger round trip are rejected outright (ns).
    pub max_rtt_ns: u64,
}

impl Default for EstimatorConfig {
    fn default() -> Self {
        Self {
            capacity: 256,
            keep_denominator: 3,
            min_kept: 3,
            min_skew_samples: 4,
            min_skew_span_ns: 2_000_000_000,
            max_rtt_ns: 2_000_000_000,
        }
    }
}

/// The current mapping between the local monotonic clock and the room clock:
///
/// ```text
/// offset(t) = offset_ns + skew_ppm · 1e-6 · (t − reference_local)
/// room(t)   = t + offset(t)
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    /// Local time at which [`Estimate::offset_ns`] applies (mean of the kept samples).
    pub reference_local: u64,
    /// `room − local` at `reference_local` (ns).
    pub offset_ns: f64,
    /// Frequency skew of the room clock relative to the local clock, in parts
    /// per million. Positive means the room clock runs faster. `0.0` until
    /// [`Estimate::skew_estimated`] is true.
    pub skew_ppm: f64,
    /// Whether enough span and samples were available to estimate skew.
    pub skew_estimated: bool,
    /// Number of samples that survived min-RTT filtering.
    pub samples_used: usize,
    /// Smallest round trip among the kept samples (ns).
    pub min_rtt_ns: u64,
    /// RMS of the regression residuals of the kept samples (ns). A jitter indicator.
    pub residual_rms_ns: f64,
}

impl Estimate {
    /// Estimated `room − local` offset at local time `local` (ns), extrapolated
    /// with the skew.
    #[must_use]
    pub fn offset_at(&self, local: u64) -> f64 {
        self.offset_ns + self.skew_ppm * 1e-6 * diff_f64(local, self.reference_local)
    }

    /// Maps a local monotonic time to the room clock (saturating).
    #[must_use]
    pub fn local_to_room(&self, local: u64) -> HostTime {
        let room = f64_to_i128(self.offset_at(local)) + i128::from(local);
        HostTime(saturate_u64(room))
    }

    /// Maps a room-clock time to local monotonic time (saturating). Inverse of
    /// [`Estimate::local_to_room`] up to rounding.
    #[must_use]
    pub fn room_to_local(&self, room: HostTime) -> u64 {
        // room − ref = u·(1 + s) + offset_ns, where u = local − ref.
        let s = self.skew_ppm * 1e-6;
        let u = (diff_f64(room.0, self.reference_local) - self.offset_ns) / (1.0 + s);
        saturate_u64(i128::from(self.reference_local) + f64_to_i128(u))
    }
}

/// Estimates the room clock from probe exchanges.
///
/// Keeps a sliding window of samples, retains the lowest-RTT third (the ones
/// least disturbed by queuing delay), and fits `offset = a + b·t` by least
/// squares. `b` is the frequency skew. The room clock is only guaranteed to be
/// continuous within one connection, so the window MUST be
/// [cleared](Self::clear) on every new connection (`docs/protocol.md` §4.1).
///
/// Allocation happens only in [`ClockEstimator::new`].
#[derive(Clone, Debug)]
pub struct ClockEstimator {
    config: EstimatorConfig,
    window: VecDeque<ClockSample>,
    scratch: Vec<ClockSample>,
    estimate: Option<Estimate>,
}

impl ClockEstimator {
    /// Creates an estimator. Degenerate configuration values are raised to
    /// their minimum (`capacity ≥ 1`, `keep_denominator ≥ 1`, `min_skew_samples ≥ 2`).
    #[must_use]
    pub fn new(mut config: EstimatorConfig) -> Self {
        config.capacity = config.capacity.max(1);
        config.keep_denominator = config.keep_denominator.max(1);
        config.min_skew_samples = config.min_skew_samples.max(2);
        Self {
            config,
            window: VecDeque::with_capacity(config.capacity),
            scratch: Vec::with_capacity(config.capacity),
            estimate: None,
        }
    }

    /// The configuration in use.
    #[must_use]
    pub fn config(&self) -> &EstimatorConfig {
        &self.config
    }

    /// Validates and adds a [`ProbeReply`] received at local time `t3`.
    ///
    /// # Errors
    ///
    /// See [`ClockSample::from_timestamps`]; additionally rejects round trips
    /// above [`EstimatorConfig::max_rtt_ns`]. Rejected samples leave the
    /// estimator unchanged.
    pub fn add_reply(&mut self, reply: &ProbeReply, t3: u64) -> Result<Estimate, SampleError> {
        let sample = ClockSample::from_timestamps(reply.t0, reply.t1, reply.t2, t3)?;
        self.add_sample(sample)
    }

    /// Adds a sample, evicting the oldest one if the window is full, and
    /// returns the updated estimate.
    ///
    /// # Errors
    ///
    /// Rejects round trips above [`EstimatorConfig::max_rtt_ns`].
    pub fn add_sample(&mut self, sample: ClockSample) -> Result<Estimate, SampleError> {
        if sample.rtt_ns > self.config.max_rtt_ns {
            return Err(SampleError::RttTooLarge {
                rtt_ns: sample.rtt_ns,
                max_ns: self.config.max_rtt_ns,
            });
        }
        if self.window.len() == self.config.capacity {
            self.window.pop_front();
        }
        self.window.push_back(sample);
        let estimate = self.recompute();
        self.estimate = Some(estimate);
        Ok(estimate)
    }

    /// The latest estimate, if at least one sample has been accepted.
    #[must_use]
    pub fn estimate(&self) -> Option<Estimate> {
        self.estimate
    }

    /// Number of samples in the window.
    #[must_use]
    pub fn len(&self) -> usize {
        self.window.len()
    }

    /// Whether the window is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }

    /// Drops all samples and the estimate (required on every new connection).
    pub fn clear(&mut self) {
        self.window.clear();
        self.estimate = None;
    }

    /// Number of samples kept by the min-RTT filter for a window of `n`.
    pub(super) fn kept(&self, n: usize) -> usize {
        n.div_ceil(self.config.keep_denominator)
            .max(self.config.min_kept)
            .min(n)
    }

    fn recompute(&mut self) -> Estimate {
        self.scratch.clear();
        self.scratch.extend(self.window.iter().copied());
        // Deterministic order: by RTT, ties broken by time.
        self.scratch
            .sort_unstable_by_key(|s| (s.rtt_ns, s.local_mid));
        let k = self.kept(self.scratch.len());
        let kept = &self.scratch[..k];
        fit(kept, &self.config)
    }
}

/// Least-squares fit of `offset = a + b·(t − t_ref)` over `kept` (non-empty).
fn fit(kept: &[ClockSample], config: &EstimatorConfig) -> Estimate {
    let n = kept.len();
    let mut min_t = u64::MAX;
    let mut max_t = 0u64;
    let mut sum_t: u128 = 0;
    let mut min_rtt = u64::MAX;
    for s in kept {
        min_t = min_t.min(s.local_mid);
        max_t = max_t.max(s.local_mid);
        sum_t += u128::from(s.local_mid);
        min_rtt = min_rtt.min(s.rtt_ns);
    }
    // Centering on the (integer) mean keeps the regression well conditioned
    // even though raw timestamps are ~1e13 ns.
    let reference_local = u64::try_from(sum_t / n.max(1) as u128).unwrap_or(u64::MAX);
    let nf = n as f64;

    let (mut mean_x, mut mean_y) = (0.0f64, 0.0f64);
    for s in kept {
        mean_x += diff_f64(s.local_mid, reference_local);
        mean_y += s.offset_ns as f64;
    }
    mean_x /= nf;
    mean_y /= nf;

    let (mut sxx, mut sxy) = (0.0f64, 0.0f64);
    for s in kept {
        let dx = diff_f64(s.local_mid, reference_local) - mean_x;
        let dy = s.offset_ns as f64 - mean_y;
        sxx += dx * dx;
        sxy += dx * dy;
    }

    let span = max_t.saturating_sub(min_t);
    let skew_estimated =
        n >= config.min_skew_samples && span >= config.min_skew_span_ns && sxx > 0.0;
    let slope = if skew_estimated { sxy / sxx } else { 0.0 };
    let offset_ns = mean_y - slope * mean_x;

    let mut sse = 0.0f64;
    for s in kept {
        let pred = offset_ns + slope * diff_f64(s.local_mid, reference_local);
        let r = s.offset_ns as f64 - pred;
        sse += r * r;
    }

    Estimate {
        reference_local,
        offset_ns,
        skew_ppm: slope * 1e6,
        skew_estimated,
        samples_used: n,
        min_rtt_ns: min_rtt,
        residual_rms_ns: (sse / nf).sqrt(),
    }
}

/// `a − b` as `f64` without intermediate overflow.
fn diff_f64(a: u64, b: u64) -> f64 {
    (i128::from(a) - i128::from(b)) as f64
}

/// Rounds a finite `f64` to the nearest `i128`, saturating; NaN maps to 0.
#[allow(clippy::cast_possible_truncation)]
fn f64_to_i128(x: f64) -> i128 {
    // `as` saturates for out-of-range values and maps NaN to 0.
    x.round() as i128
}

fn saturate_u64(x: i128) -> u64 {
    u64::try_from(x.max(0)).unwrap_or(u64::MAX)
}
