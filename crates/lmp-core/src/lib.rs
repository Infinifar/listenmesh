//! # lmp-core
//!
//! Core building blocks of the ListenMesh Protocol (LMP): the wire message set,
//! its canonical encoding, clock estimation and drift control.
//!
//! **Status: M0 (scaffolding).** See `docs/protocol.md` for the specification
//! this crate implements and `docs/roadmap.md` for what is implemented.
//!
//! - [`message`]: the core message set and [`message::Envelope`].
//! - [`codec`]: canonical postcard encoding with strict decoding.
//! - [`time`]: monotonic time, position, epoch/revision and rate primitives.
#![forbid(unsafe_code)]

extern crate alloc;

pub mod codec;
pub mod message;
pub mod time;
