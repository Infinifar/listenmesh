# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
The **protocol** is versioned separately via the envelope `ver` field; see
[`docs/protocol.md` §9](docs/protocol.md#9-versioning-and-evolution).

## [Unreleased]

### Added

- Cargo workspace with `lmp-core`, `lmp-server`, `lmp-client`, `lmp-conformance`
  and `lmp-cli`; pinned toolchain, shared lints, rustfmt and clippy configuration.
- CI: `fmt --check`, `clippy -D warnings`, tests on Linux/macOS/Windows,
  `rustdoc -D warnings`, MSRV check.
- Community files: contributing guide (incl. ADR process), code of conduct,
  security policy, issue and pull request templates.
- `lmp-core`: core message set (`Envelope`, client/server messages, `Content`,
  `Snapshot`, …) with canonical postcard codec, strict decoding (version check,
  trailing bytes, extension-name validation), property-based round-trip tests and
  byte-exact golden vectors (`crates/lmp-core/tests/golden/v0.hex`).
- `lmp-core`: `MonotonicClock` (quanta) and `ClockEstimator` — NTP four-timestamp
  sampling, lowest-RTT-third filtering, centered least-squares regression for
  offset and skew (ppm); deterministic synthetic-network tests asserting skew
  error < 5 ppm.
