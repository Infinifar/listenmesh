# Contributing to ListenMesh

Thanks for your interest! ListenMesh is a **protocol first** project: the
specification in [`docs/protocol.md`](docs/protocol.md) is the product, and the
Rust crates are its reference implementation. Changes are judged against the
project priorities, in order:

1. Correctness of time alignment
2. Media-source agnosticism
3. Resilience
4. Performance
5. Evolvability

## Build and test

Requirements: a Rust toolchain. The pinned development toolchain is declared in
[`rust-toolchain.toml`](rust-toolchain.toml) (rustup picks it up
automatically); the minimum supported Rust version (MSRV) is
`workspace.package.rust-version` in [`Cargo.toml`](Cargo.toml).

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

CI runs exactly these, plus an MSRV check, on Linux, macOS and Windows.

### Golden vectors

`crates/lmp-core/tests/golden/` holds byte-exact wire fixtures. If you change
the wire format intentionally, regenerate them with:

```sh
LMP_BLESS=1 cargo test -p lmp-core --test golden
```

and update `docs/protocol.md` in the **same** pull request. A wire change
without a spec change (or vice versa) will be rejected.

## Code rules

- `#![forbid(unsafe_code)]` in every crate (until an ADR says otherwise).
- No `unwrap()` / `expect()` in library code (enforced by clippy; tests may).
- **Never** use `SystemTime` / wall-clock time on the timeline path
  ([ADR-0005](docs/adr/0005-monotonic-clocks-only.md)).
- No new dependencies without updating [`docs/stack.md`](docs/stack.md);
  architectural dependencies (async runtime, storage, transport) need an ADR.
- Unimplemented code is `todo!()` with a marker that names the milestone and
  the owning issue, e.g. `// TODO(M1, #4): derive position from Timeline`.
- Public items are documented (`missing_docs` is on).

## Commits and pull requests

- [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):
  `feat(core): …`, `fix(server): …`, `docs(spec): …`, `build: …`, `ci: …`,
  `test(core): …`, `chore: …`. Scopes are crate short names (`core`, `server`,
  `client`, `conformance`, `cli`) or `spec`, `adr`.
- Small commits; **one pull request per concern**.
- Link the issue (`Closes #N` / `Refs #N`) and the spec section you touch.
- Update [`CHANGELOG.md`](CHANGELOG.md) under `Unreleased`.

## ADR process

Architecture Decision Records live in [`docs/adr/`](docs/adr/).

1. Open an issue with the **Architecture Decision Record** template to discuss.
2. Open a PR adding `docs/adr/NNNN-short-title.md` (next free number) with
   status `Proposed`, using the same structure as existing ADRs
   (Context / Decision / Consequences).
3. On merge the status becomes `Accepted`. Accepted ADRs are immutable except
   for typo fixes; to change a decision, write a new ADR that **supersedes** it
   and mark the old one `Superseded by ADR-NNNN`.
4. Normative consequences must be reflected in `docs/protocol.md` in the same PR.

## Protocol changes and extensions

- The core message set is closed. New behaviour starts life as an extension in
  the `x.<vendor>.` namespace (the project itself uses `x.lmp.`), and must be
  safely ignorable by peers that do not understand it.
- Promotion of an extension into the core set follows the versioning policy in
  [`docs/protocol.md` §9](docs/protocol.md#9-versioning-and-evolution).

## License

By contributing, you agree that your contributions are dual-licensed under
MIT OR Apache-2.0, without any additional terms or conditions, as described in
the README.

## Code of conduct

Participation is governed by the [Code of Conduct](CODE_OF_CONDUCT.md).
