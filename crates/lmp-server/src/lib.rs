//! # lmp-server
//!
//! Reference server for the ListenMesh Protocol (LMP): one actor per room,
//! sharding and persistence.
//!
//! **Status: M0 (scaffolding).** Nothing here is functional yet. No async
//! runtime is chosen yet; that decision comes with M1 (#12).
//!
//! - [`room`]: per-room actor (ADR-0006).
//! - [`failover`]: host liveness and deputy election (ADR-0004).
//! - [`shard`]: room → node placement.
//! - [`persist`]: write-behind room persistence.
#![forbid(unsafe_code)]

pub mod failover;
pub mod persist;
pub mod room;
pub mod shard;
