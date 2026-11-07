//! Write-behind room persistence (`docs/roadmap.md` M4).
//!
//! The storage engine is not chosen yet; adding one requires an ADR (#13).

use lmp_core::message::{RoomId, Snapshot};

/// Errors from a [`RoomStore`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// The backing store failed.
    #[error("storage backend error: {0}")]
    Backend(String),
}

/// Durable storage for room snapshots.
pub trait RoomStore {
    /// Persists the latest snapshot of a room (write-behind; may be batched).
    ///
    /// # Errors
    ///
    /// Backend failure.
    fn save(&mut self, room: &RoomId, snapshot: &Snapshot) -> Result<(), StoreError>;

    /// Loads the last persisted snapshot of a room.
    ///
    /// # Errors
    ///
    /// Backend failure.
    fn load(&self, room: &RoomId) -> Result<Option<Snapshot>, StoreError>;
}

/// Placeholder store; every call is unimplemented.
#[derive(Debug, Default)]
pub struct UnimplementedStore;

impl RoomStore for UnimplementedStore {
    fn save(&mut self, _room: &RoomId, _snapshot: &Snapshot) -> Result<(), StoreError> {
        // TODO(M4, #13): write-behind persistence.
        todo!("persistence (M4, #13)")
    }

    fn load(&self, _room: &RoomId) -> Result<Option<Snapshot>, StoreError> {
        // TODO(M4, #13): recovery path (bumps epoch on load).
        todo!("persistence (M4, #13)")
    }
}
