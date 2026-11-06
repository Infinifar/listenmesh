//! `lmp` — demo binary for the ListenMesh Protocol (placeholder).
//!
//! **Status: M0 (scaffolding).** This binary does nothing useful yet.
#![forbid(unsafe_code)]

use std::io::Write as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    let msg = concat!(
        "lmp ",
        env!("CARGO_PKG_VERSION"),
        ": ListenMesh is at milestone M0 (scaffolding) and is not usable yet.\n",
        "See https://github.com/Infinifar/listenmesh/blob/main/docs/roadmap.md\n",
    );
    match std::io::stdout().write_all(msg.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
