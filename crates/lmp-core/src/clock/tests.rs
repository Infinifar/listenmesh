//! Deterministic tests for the clock estimator, driven by synthetic sample
//! sequences (seeded PRNG, simulated network). No real time is involved.

// The simulator converts between f64 nanoseconds and integer timestamps on
// purpose; values are always positive and far below 2^53.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use approx::assert_abs_diff_eq;

use super::*;
use crate::message::ProbeReply;
use crate::time::HostTime;

// ---------------------------------------------------------------------------
// Simulation harness
// ---------------------------------------------------------------------------

/// SplitMix64: tiny, deterministic, good enough for test noise.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Exponential with the given mean.
    fn exp(&mut self, mean: f64) -> f64 {
        -mean * (1.0 - self.uniform()).ln()
    }
}

/// A simulated client/server pair. True time τ is the client's local
/// monotonic clock (ns). The room clock is `S(τ) = s0 + (τ − τ0)·(1 + ε)`.
#[derive(Clone, Copy, Debug)]
struct Net {
    /// Room-clock skew ε in ppm (ground truth).
    skew_ppm: f64,
    /// Fixed one-way delays (ns).
    fwd_ns: f64,
    back_ns: f64,
    /// Mean of the exponential queuing jitter added to each direction (ns).
    jitter_mean_ns: f64,
    /// Probability that a direction suffers a large outlier delay.
    outlier_prob: f64,
    /// Outlier delays are uniform in [0, outlier_max_ns).
    outlier_max_ns: f64,
    /// Server processing time between t1 and t2 (ns).
    proc_ns: f64,
}

const TAU0: f64 = 5_000_000_000_000.0; // client uptime at start: ~83 min
const S0: f64 = 1_000_000_000.0; // server uptime at start: 1 s

impl Net {
    fn quiet(skew_ppm: f64) -> Self {
        Self {
            skew_ppm,
            fwd_ns: 5e6,
            back_ns: 5e6,
            jitter_mean_ns: 0.0,
            outlier_prob: 0.0,
            outlier_max_ns: 0.0,
            proc_ns: 20e3,
        }
    }

    fn typical(skew_ppm: f64) -> Self {
        Self {
            jitter_mean_ns: 2e6,
            outlier_prob: 0.05,
            outlier_max_ns: 200e6,
            ..Self::quiet(skew_ppm)
        }
    }

    fn room(&self, tau: f64) -> f64 {
        S0 + (tau - TAU0) * (1.0 + self.skew_ppm * 1e-6)
    }

    /// True `room − local` offset at local time `tau`.
    fn true_offset(&self, tau: f64) -> f64 {
        self.room(tau) - tau
    }

    fn one_way(&self, base: f64, rng: &mut Rng) -> f64 {
        let mut d = base + rng.exp(self.jitter_mean_ns.max(f64::MIN_POSITIVE));
        if self.jitter_mean_ns == 0.0 {
            d = base;
        }
        if rng.uniform() < self.outlier_prob {
            d += rng.uniform() * self.outlier_max_ns;
        }
        d
    }

    /// Performs one probe exchange sent at local time `t0` and returns the
    /// reply plus the local receive time `t3`.
    fn exchange(&self, seq: u32, t0: u64, rng: &mut Rng) -> (ProbeReply, u64) {
        let tau0 = t0 as f64;
        let tau1 = tau0 + self.one_way(self.fwd_ns, rng);
        let tau2 = tau1 + self.proc_ns;
        let tau3 = tau2 + self.one_way(self.back_ns, rng);
        let reply = ProbeReply {
            seq,
            t0,
            t1: HostTime(self.room(tau1).round() as u64),
            t2: HostTime(self.room(tau2).round() as u64),
        };
        (reply, tau3.round() as u64)
    }
}

/// Probe send times: a cold-start burst of `burst` probes at 10 Hz, then
/// `steady` probes at 1 Hz, each with up to ±20 ms scheduling jitter.
fn schedule(burst: usize, steady: usize, rng: &mut Rng) -> Vec<u64> {
    let mut t = TAU0;
    let mut out = Vec::with_capacity(burst + steady);
    for i in 0..burst + steady {
        let jitter = (rng.uniform() - 0.5) * 40e6;
        out.push((t + jitter) as u64);
        t += if i < burst { 100e6 } else { 1e9 };
    }
    out
}

/// Runs a full simulation and returns the final estimator.
fn simulate(net: &Net, seed: u64, burst: usize, steady: usize) -> ClockEstimator {
    let mut rng = Rng(seed);
    let mut est = ClockEstimator::new(EstimatorConfig::default());
    for (seq, t0) in schedule(burst, steady, &mut rng).into_iter().enumerate() {
        let (reply, t3) = net.exchange(seq as u32, t0, &mut rng);
        // Samples above max_rtt are legitimately rejected; everything else
        // must be accepted.
        match est.add_reply(&reply, t3) {
            Ok(_) | Err(SampleError::RttTooLarge { .. }) => {}
            Err(e) => panic!("unexpected rejection: {e}"),
        }
    }
    est
}

const SKEWS_PPM: [f64; 9] = [-100.0, -50.0, -21.7, -3.0, 0.0, 7.0, 12.5, 50.0, 100.0];
const SEEDS: [u64; 4] = [1, 2, 3, 0xC0FFEE];
/// Steady-state probes per simulation: 5 minutes at 1 Hz, enough to fill the
/// default 256-sample window.
const STEADY: usize = 300;
/// Acceptance threshold from the M0 definition of done.
const MAX_SKEW_ERR_PPM: f64 = 5.0;

// ---------------------------------------------------------------------------
// Sample construction
// ---------------------------------------------------------------------------

#[test]
fn sample_from_timestamps() {
    // Clocks equal, symmetric 500 ns paths, 100 ns processing.
    let s = ClockSample::from_timestamps(1_000, HostTime(1_500), HostTime(1_600), 2_100).unwrap();
    assert_eq!(
        s,
        ClockSample {
            local_mid: 1_550,
            offset_ns: 0,
            rtt_ns: 1_000
        }
    );

    // Room clock 9 µs ahead.
    let s = ClockSample::from_timestamps(0, HostTime(10_000), HostTime(10_000), 2_000).unwrap();
    assert_eq!(
        s,
        ClockSample {
            local_mid: 1_000,
            offset_ns: 9_000,
            rtt_ns: 2_000
        }
    );

    // Room clock behind (negative offset), odd sum rounds toward −∞.
    let s = ClockSample::from_timestamps(1_000_000, HostTime(0), HostTime(1), 1_000_010).unwrap();
    assert_eq!(s.offset_ns, -1_000_005);
    assert_eq!(s.rtt_ns, 9);

    // Extreme but valid values do not overflow.
    let max = i64::MAX.unsigned_abs();
    let s = ClockSample::from_timestamps(0, HostTime(max), HostTime(max), 0);
    assert_eq!(s.unwrap().offset_ns, i64::MAX);
    assert_eq!(
        ClockSample::from_timestamps(0, HostTime(max + 1), HostTime(max + 1), 0),
        Err(SampleError::OffsetOutOfRange)
    );
}

#[test]
fn sample_rejects_invalid() {
    assert_eq!(
        ClockSample::from_timestamps(10, HostTime(0), HostTime(0), 9),
        Err(SampleError::ClientNonCausal)
    );
    assert_eq!(
        ClockSample::from_timestamps(0, HostTime(5), HostTime(4), 10),
        Err(SampleError::ServerNonCausal)
    );
    assert_eq!(
        ClockSample::from_timestamps(0, HostTime(0), HostTime(11), 10),
        Err(SampleError::NegativeDelay)
    );
}

#[test]
fn rejects_rtt_above_limit_without_side_effects() {
    let mut est = ClockEstimator::new(EstimatorConfig::default());
    let s = ClockSample {
        local_mid: 0,
        offset_ns: 0,
        rtt_ns: 2_000_000_001,
    };
    assert!(matches!(
        est.add_sample(s),
        Err(SampleError::RttTooLarge { .. })
    ));
    assert!(est.is_empty());
    assert!(est.estimate().is_none());
}

// ---------------------------------------------------------------------------
// Golden sample sequence (hand-computed expectations)
// ---------------------------------------------------------------------------

/// Nine clean samples on an exact line `offset = 1 ms + 50 ppm · (t − t₀)`
/// with RTT 4 ms, interleaved with eighteen samples with large RTT and
/// garbage offsets. The lowest-RTT third (9 of 27) is exactly the clean set,
/// so the fit must recover the line exactly.
#[test]
fn golden_sequence_recovers_exact_line() {
    let t_start: u64 = 10_000_000_000;
    let mut est = ClockEstimator::new(EstimatorConfig::default());
    for i in 0..9u64 {
        let t = t_start + i * 1_000_000_000;
        let clean = 1_000_000 + 50 * i * 1_000; // 50 ppm × 1 s = 50 µs per step
        est.add_sample(ClockSample {
            local_mid: t,
            offset_ns: i64::try_from(clean).unwrap(),
            rtt_ns: 4_000_000,
        })
        .unwrap();
        for j in 1..=2u64 {
            est.add_sample(ClockSample {
                local_mid: t + j * 300_000_000,
                offset_ns: 7_777_777 * i64::try_from(j).unwrap(),
                rtt_ns: 4_000_000 + j * 10_000_000,
            })
            .unwrap();
        }
    }
    let e = est.estimate().unwrap();
    assert_eq!(e.samples_used, 9);
    assert_eq!(e.min_rtt_ns, 4_000_000);
    assert_eq!(e.reference_local, t_start + 4_000_000_000);
    assert!(e.skew_estimated);
    assert_abs_diff_eq!(e.skew_ppm, 50.0, epsilon = 1e-9);
    assert_abs_diff_eq!(e.offset_ns, 1_200_000.0, epsilon = 1e-6);
    assert_abs_diff_eq!(e.residual_rms_ns, 0.0, epsilon = 1e-6);
    assert_abs_diff_eq!(e.offset_at(t_start), 1_000_000.0, epsilon = 1e-6);
    assert_eq!(e.local_to_room(t_start), HostTime(t_start + 1_000_000));
}

#[test]
fn kept_fraction_is_lowest_third_with_floor() {
    let est = ClockEstimator::new(EstimatorConfig::default());
    assert_eq!(est.kept(1), 1);
    assert_eq!(est.kept(2), 2);
    assert_eq!(est.kept(3), 3);
    assert_eq!(est.kept(9), 3);
    assert_eq!(est.kept(10), 4);
    assert_eq!(est.kept(128), 43);
    assert_eq!(est.kept(256), 86);
}

#[test]
fn skew_requires_span_and_samples() {
    let mut est = ClockEstimator::new(EstimatorConfig::default());
    // 10 samples over 0.9 s: below the 2 s span → offset only.
    for i in 0..10u64 {
        let e = est
            .add_sample(ClockSample {
                local_mid: i * 100_000_000,
                offset_ns: i64::try_from(i * 10_000).unwrap(),
                rtt_ns: 1_000_000,
            })
            .unwrap();
        assert!(!e.skew_estimated);
        assert_eq!(e.skew_ppm, 0.0);
    }
}

#[test]
fn window_is_bounded_and_clear_resets() {
    let cfg = EstimatorConfig {
        capacity: 16,
        ..EstimatorConfig::default()
    };
    let mut est = ClockEstimator::new(cfg);
    for i in 0..100u64 {
        est.add_sample(ClockSample {
            local_mid: i,
            offset_ns: 0,
            rtt_ns: 1,
        })
        .unwrap();
    }
    assert_eq!(est.len(), 16);
    est.clear();
    assert!(est.is_empty());
    assert!(est.estimate().is_none());
}

// ---------------------------------------------------------------------------
// Synthetic network simulations (acceptance: skew error < 5 ppm)
// ---------------------------------------------------------------------------

#[test]
fn noise_free_is_exact() {
    for &skew in &SKEWS_PPM {
        let net = Net::quiet(skew);
        let est = simulate(&net, 7, 10, 60);
        let e = est.estimate().unwrap();
        assert!(e.skew_estimated);
        // Only nanosecond rounding of the timestamps remains.
        assert_abs_diff_eq!(e.skew_ppm, skew, epsilon = 0.01);
        let t = e.reference_local;
        assert_abs_diff_eq!(e.offset_at(t), net.true_offset(t as f64), epsilon = 2.0);
    }
}

#[test]
fn skew_error_below_5ppm_typical_network() {
    let mut worst: f64 = 0.0;
    for &skew in &SKEWS_PPM {
        for &seed in &SEEDS {
            let e = simulate(&Net::typical(skew), seed, 10, STEADY)
                .estimate()
                .unwrap();
            assert!(e.skew_estimated);
            let err = (e.skew_ppm - skew).abs();
            worst = worst.max(err);
            assert!(
                err < MAX_SKEW_ERR_PPM,
                "skew {skew} ppm, seed {seed}: estimated {} ppm (error {err})",
                e.skew_ppm
            );
        }
    }
    // Keep a margin so the test is meaningful, not borderline.
    assert!(worst < MAX_SKEW_ERR_PPM / 2.0, "worst error {worst} ppm");
}

#[test]
fn skew_error_below_5ppm_heavy_outliers() {
    for &skew in &SKEWS_PPM {
        for &seed in &SEEDS {
            let net = Net {
                outlier_prob: 0.30,
                outlier_max_ns: 500e6,
                ..Net::typical(skew)
            };
            let e = simulate(&net, seed, 10, STEADY).estimate().unwrap();
            let err = (e.skew_ppm - skew).abs();
            assert!(
                err < MAX_SKEW_ERR_PPM,
                "skew {skew}, seed {seed}: error {err}"
            );
        }
    }
}

#[test]
fn skew_error_below_5ppm_high_jitter() {
    for &skew in &SKEWS_PPM {
        for &seed in &SEEDS {
            let net = Net {
                jitter_mean_ns: 5e6,
                ..Net::typical(skew)
            };
            let e = simulate(&net, seed, 10, STEADY).estimate().unwrap();
            let err = (e.skew_ppm - skew).abs();
            assert!(
                err < MAX_SKEW_ERR_PPM,
                "skew {skew}, seed {seed}: error {err}"
            );
        }
    }
}

/// Skew error scales as σ / (T·√n) (`docs/clock-sync.md` §4.4). Under extreme
/// jitter (10 ms mean per direction) a 256-sample window is not enough; a
/// 10-minute window brings the error back under 5 ppm.
#[test]
fn extreme_jitter_needs_longer_window() {
    let cfg = EstimatorConfig {
        capacity: 600,
        ..EstimatorConfig::default()
    };
    for &skew in &SKEWS_PPM {
        for &seed in &SEEDS {
            let net = Net {
                jitter_mean_ns: 10e6,
                ..Net::typical(skew)
            };
            let mut rng = Rng(seed);
            let mut est = ClockEstimator::new(cfg);
            for (seq, t0) in schedule(10, 600, &mut rng).into_iter().enumerate() {
                let (reply, t3) = net.exchange(seq as u32, t0, &mut rng);
                let _ = est.add_reply(&reply, t3);
            }
            let err = (est.estimate().unwrap().skew_ppm - skew).abs();
            assert!(
                err < MAX_SKEW_ERR_PPM,
                "skew {skew}, seed {seed}: error {err}"
            );
        }
    }
}

/// Path asymmetry biases the offset by (fwd − back)/2 — this is unobservable
/// by any two-way protocol — but must not bias the skew.
#[test]
fn asymmetric_paths_bias_offset_not_skew() {
    for &skew in &SKEWS_PPM {
        let net = Net {
            fwd_ns: 9e6,
            back_ns: 3e6,
            ..Net::typical(skew)
        };
        let e = simulate(&net, 11, 10, STEADY).estimate().unwrap();
        assert!((e.skew_ppm - skew).abs() < MAX_SKEW_ERR_PPM);
        let bias = (net.fwd_ns - net.back_ns) / 2.0;
        let t = e.reference_local;
        let err = e.offset_at(t) - net.true_offset(t as f64);
        assert_abs_diff_eq!(err, bias, epsilon = 1.5e6);
    }
}

/// Cold start: 10 probes at 10 Hz (1 s) give a usable offset (< 2 ms error)
/// before any skew is estimated (`docs/protocol.md` §8: convergence < 3 s).
#[test]
fn cold_start_offset_converges_within_burst() {
    for &skew in &SKEWS_PPM {
        for &seed in &SEEDS {
            let net = Net::typical(skew);
            let e = simulate(&net, seed, 10, 0).estimate().unwrap();
            assert!(!e.skew_estimated);
            let t = e.reference_local;
            let err = (e.offset_at(t) - net.true_offset(t as f64)).abs();
            assert!(err < 2e6, "skew {skew}, seed {seed}: offset error {err} ns");
        }
    }
}

/// Extrapolating 30 s past the last sample stays well inside the 15 ms
/// steady-state budget.
#[test]
fn extrapolation_holds_offset() {
    for &skew in &SKEWS_PPM {
        let net = Net::typical(skew);
        let est = simulate(&net, 5, 10, STEADY);
        let e = est.estimate().unwrap();
        let t = (TAU0 + (STEADY as f64 + 31.0) * 1e9) as u64;
        let err = (e.offset_at(t) - net.true_offset(t as f64)).abs();
        assert!(err < 1e6, "skew {skew}: extrapolated offset error {err} ns");
    }
}

#[test]
fn local_room_mapping_roundtrips() {
    let net = Net::typical(-37.0);
    let e = simulate(&net, 9, 10, 60).estimate().unwrap();
    for k in 0..1_000u64 {
        let local = e.reference_local + k * 1_000_000_007;
        let room = e.local_to_room(local);
        let back = e.room_to_local(room);
        assert!(
            back.abs_diff(local) <= 1,
            "local {local} → {room:?} → {back}"
        );
    }
}

#[test]
fn mapping_saturates_instead_of_panicking() {
    let e = Estimate {
        reference_local: 0,
        offset_ns: -1e12,
        skew_ppm: 0.0,
        skew_estimated: false,
        samples_used: 1,
        min_rtt_ns: 0,
        residual_rms_ns: 0.0,
    };
    assert_eq!(e.local_to_room(0), HostTime(0));
    let e = Estimate {
        offset_ns: 1e30,
        ..e
    };
    assert_eq!(e.local_to_room(u64::MAX), HostTime(u64::MAX));
    assert_eq!(e.room_to_local(HostTime(0)), 0);
}

#[test]
fn deterministic_across_runs() {
    let a = simulate(&Net::typical(42.0), 99, 10, 100).estimate();
    let b = simulate(&Net::typical(42.0), 99, 10, 100).estimate();
    assert_eq!(a, b);
}

proptest::proptest! {
    /// The estimator is total: any accepted sample sequence yields finite numbers
    /// and mapping never panics.
    #[test]
    fn estimator_is_total(
        samples in proptest::collection::vec(
            (proptest::num::u64::ANY, proptest::num::i64::ANY, 0u64..=2_000_000_000),
            1..300,
        ),
        probe in proptest::num::u64::ANY,
    ) {
        let mut est = ClockEstimator::new(EstimatorConfig::default());
        for (local_mid, offset_ns, rtt_ns) in samples {
            let e = est.add_sample(ClockSample { local_mid, offset_ns, rtt_ns }).unwrap();
            proptest::prop_assert!(e.offset_ns.is_finite());
            proptest::prop_assert!(e.skew_ppm.is_finite());
            proptest::prop_assert!(e.residual_rms_ns.is_finite());
            let _ = e.room_to_local(e.local_to_room(probe));
        }
    }
}
