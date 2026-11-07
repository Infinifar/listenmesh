//! # lmp-client
//!
//! Reference client for the ListenMesh Protocol (LMP): transport, session state
//! machine and reconnect handling.
//!
//! **Status: M0 (scaffolding).** Nothing here is functional yet.
//!
//! - [`session`]: session state machine (epoch/rev handling, snapshots).
//! - [`probe`]: adaptive probe scheduler.
//! - [`reconnect`]: reconnect with backoff and supersede.
//! - [`transport`]: transport binding abstraction.
#![forbid(unsafe_code)]

pub mod probe;
pub mod reconnect;
pub mod session;
pub mod transport;
