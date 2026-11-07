//! L3 timeline derivation (`docs/protocol.md` §6.4, ADR-0002).
//!
//! Clients never receive an authoritative "current position". They derive it
//! locally from the [`Timeline`] and the room clock:
//!
//! ```text
//! pos(t) = base_pos_ms + (t − base_host_time) · rate
//! ```
//!
//! **Status: skeleton.** Implementation is tracked in
//! [#4](https://github.com/Infinifar/listenmesh/issues/4).

use crate::message::Timeline;
use crate::time::{HostTime, PosMs};

/// Derives the media position at room-clock time `room_now`.
///
/// Before `base_host_time` the position is `base_pos_ms` (a scheduled
/// timeline has not started yet). The result saturates and is never clamped
/// to the content's `duration_hint` (ADR-0003).
#[must_use]
pub fn position_at(_timeline: &Timeline, _room_now: HostTime) -> PosMs {
    // TODO(M1, #4): fixed-point derivation, saturating, property-tested.
    todo!("timeline derivation (M1, #4)")
}
