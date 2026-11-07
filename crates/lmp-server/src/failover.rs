//! Host liveness and deputy election (`docs/protocol.md` §3.4–§3.5).

use lmp_core::message::ClientId;

/// Host silence after which authority moves (30 s).
pub const HOST_SILENCE_TIMEOUT_NS: u64 = 30_000_000_000;

/// A failover candidate as tracked by the room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// Member identity.
    pub client_id: ClientId,
    /// Whether the member is ready and buffered.
    pub buffered: bool,
    /// Smoothed round-trip time (ns).
    pub rtt_ns: u64,
}

/// Picks the deputy: buffered members only, lowest RTT, ties by `client_id`.
#[must_use]
pub fn elect_deputy(_candidates: &[Candidate]) -> Option<ClientId> {
    // TODO(M3, #8): deputy election, epoch bump, HostChanged, reclaim window.
    todo!("deputy election (M3, #8)")
}
