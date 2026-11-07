//! Client session state machine (`docs/protocol.md` §6).

use lmp_core::message::{ServerEnvelope, Snapshot};
use lmp_core::time::{Epoch, Rev};

/// Connection-level session state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionState {
    /// Not connected; room state (if any) is retained.
    Disconnected,
    /// `Hello` sent, waiting for `Welcome`.
    Joining,
    /// Member of the room.
    Joined,
    /// Server announced a restart; reconnect after the hint.
    Restarting {
        /// Suggested delay before reconnecting (ms).
        retry_after_ms: u32,
    },
}

/// The client's view of the room.
#[derive(Debug)]
pub struct Session {
    state: SessionState,
    epoch: Epoch,
    rev: Rev,
    snapshot: Option<Snapshot>,
}

impl Session {
    /// A fresh, disconnected session.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: SessionState::Disconnected,
            epoch: Epoch(0),
            rev: Rev(0),
            snapshot: None,
        }
    }

    /// Current connection state.
    #[must_use]
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Current epoch and revision.
    #[must_use]
    pub fn version(&self) -> (Epoch, Rev) {
        (self.epoch, self.rev)
    }

    /// Last full snapshot, if any.
    #[must_use]
    pub fn snapshot(&self) -> Option<&Snapshot> {
        self.snapshot.as_ref()
    }

    /// Applies a server message. Stale-epoch messages are discarded.
    pub fn apply(&mut self, _envelope: ServerEnvelope) {
        // TODO(M1, #6): Welcome/Resync snapshot adoption; TODO(M3, #8): epoch checks.
        todo!("session state machine (M1, #6)")
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
