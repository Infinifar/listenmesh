<!-- Title must follow Conventional Commits, e.g. `feat(core): add Probe message`. -->

## What

<!-- One concern per PR. Summarize the change. -->

## Why

<!-- Link the issue: "Closes #N" or "Refs #N". Link spec sections / ADRs. -->

## Checklist

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] No `unwrap()` / `expect()` in library code; no `unsafe`
- [ ] Wire changes: golden vectors updated **and** `docs/protocol.md` updated
- [ ] Architectural changes: ADR added or referenced
- [ ] `CHANGELOG.md` updated under `Unreleased`
- [ ] No wall-clock (`SystemTime`) on the timeline path (ADR-0005)
