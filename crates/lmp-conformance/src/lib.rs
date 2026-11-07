//! # lmp-conformance
//!
//! Conformance tooling for the ListenMesh Protocol (LMP): byte-exact golden
//! vectors and an interoperability harness for third-party implementations.
//!
//! **Status: M0 (scaffolding).** The M0 golden vectors live in
//! `crates/lmp-core/tests/golden/` and will move here, in a language-neutral
//! form, under [#15](https://github.com/Infinifar/listenmesh/issues/15).
#![forbid(unsafe_code)]

/// A conformance scenario run against an implementation under test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    /// Codec: every golden vector decodes and re-encodes byte-exactly.
    Codec,
    /// Late join receives a complete snapshot.
    LateJoin,
    /// Group seek suspension resumes all members together.
    GroupSeek,
    /// Host failover and reclaim.
    Failover,
    /// Graceful server restart.
    Restart,
}

/// Outcome of a scenario.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The implementation conforms.
    Pass,
    /// The implementation violates the spec; the message cites the section.
    Fail(String),
}

/// Runs one scenario.
#[must_use]
pub fn run(_scenario: Scenario) -> Outcome {
    // TODO(M4, #15): scenario runner against a black-box implementation.
    todo!("conformance harness (M4, #15)")
}
