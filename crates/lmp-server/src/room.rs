//! Per-room actor (ADR-0006, `docs/protocol.md` §6).
//!
//! One lightweight task per room owns all room state and processes a bounded
//! inbox. Broadcasts are encoded once ([`lmp_core::codec::encode_shared`]) and
//! the resulting `Arc<[u8]>` is shared across member connections.

use lmp_core::message::{ClientEnvelope, ClientId, RoomId, Snapshot};

/// Bounded inbox capacity per room (messages).
pub const DEFAULT_INBOX_CAPACITY: usize = 1_024;

/// Configuration of a room actor.
#[derive(Clone, Debug)]
pub struct RoomConfig {
    /// Inbox capacity; senders observe backpressure when full.
    pub inbox_capacity: usize,
    /// Maximum number of members.
    pub max_members: usize,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            inbox_capacity: DEFAULT_INBOX_CAPACITY,
            max_members: 200,
        }
    }
}

/// Input to a room actor.
#[derive(Debug)]
pub enum RoomInput {
    /// A decoded message from a member connection.
    Message {
        /// Sender identity.
        from: ClientId,
        /// The message.
        envelope: ClientEnvelope,
    },
    /// A connection for `client_id` closed or was superseded.
    Disconnected {
        /// The member.
        client_id: ClientId,
    },
    /// Periodic tick driving timers (failover, canary, readiness deadlines).
    Tick {
        /// Server monotonic time (ns).
        now_ns: u64,
    },
}

/// The room actor state.
#[derive(Debug)]
pub struct Room {
    id: RoomId,
    config: RoomConfig,
}

impl Room {
    /// Creates an empty room.
    #[must_use]
    pub fn new(id: RoomId, config: RoomConfig) -> Self {
        Self { id, config }
    }

    /// The room identifier.
    #[must_use]
    pub fn id(&self) -> &RoomId {
        &self.id
    }

    /// The room configuration.
    #[must_use]
    pub fn config(&self) -> &RoomConfig {
        &self.config
    }

    /// Processes one input.
    pub fn handle(&mut self, _input: RoomInput) {
        // TODO(M1, #12): actor loop, bounded outboxes, encode-once fanout.
        todo!("room actor (M1, #12)")
    }

    /// Builds a complete, single-revision snapshot for a joining member.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        // TODO(M1, #6): late-join snapshot.
        todo!("snapshot (M1, #6)")
    }
}
