//! The LMP core message set (`docs/protocol.md` §5).
//!
//! This module defines **exactly** the core messages of protocol version
//! [`PROTOCOL_VERSION`]. Everything else travels as an [`Extension`] in the
//! `x.<vendor>.` namespace and MUST be ignorable by peers that do not
//! understand it.
//!
//! # Wire stability
//!
//! The encoding is [postcard](https://docs.rs/postcard) (see
//! [`crate::codec`]). Postcard encodes enum variants by their **declaration
//! index**, and struct fields in **declaration order**. Therefore, within a
//! protocol version:
//!
//! - enum variants MUST NOT be reordered or removed; new variants are only
//!   appended, and only together with a version/capability bump;
//! - struct fields MUST NOT be reordered, removed, or retyped.
//!
//! The byte-exact golden vectors in `tests/golden/` enforce this.

use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::time::{Epoch, HostTime, PosMs, Rate, Rev};

/// Wire protocol version carried in [`Envelope::ver`].
///
/// `0` denotes the pre-1.0 draft; drafts MAY change incompatibly. See
/// `docs/protocol.md` §9.
pub const PROTOCOL_VERSION: u16 = 0;

/// Every LMP message is wrapped in an envelope.
///
/// `epoch` and `rev` are the sender's view of the session (ADR-0004). A
/// receiver MUST discard any message whose `epoch` is lower than its current
/// epoch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<B> {
    /// Protocol version, see [`PROTOCOL_VERSION`].
    pub ver: u16,
    /// Session epoch as known by the sender.
    pub epoch: Epoch,
    /// Session revision as known by the sender.
    pub rev: Rev,
    /// The message itself.
    pub body: B,
}

impl<B> Envelope<B> {
    /// Wraps `body` in an envelope for the current [`PROTOCOL_VERSION`].
    #[must_use]
    pub const fn new(epoch: Epoch, rev: Rev, body: B) -> Self {
        Self {
            ver: PROTOCOL_VERSION,
            epoch,
            rev,
            body,
        }
    }
}

/// An envelope sent from a client to the server.
pub type ClientEnvelope = Envelope<ClientMessage>;
/// An envelope sent from the server to a client.
pub type ServerEnvelope = Envelope<ServerMessage>;

// ---------------------------------------------------------------------------
// Client -> Server
// ---------------------------------------------------------------------------

/// Messages sent from a client to the server (`docs/protocol.md` §5.4).
///
/// Variant order is the wire tag; see the module docs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Tag 0: vendor extension (ignorable).
    Extension(Extension),
    /// Tag 1: first message on a connection.
    Hello(Hello),
    /// Tag 2: clock probe.
    Probe(Probe),
    /// Tag 3: readiness / buffering report.
    SetReady(SetReady),
    /// Tag 4: suggest a track for the queue.
    Suggest(Suggest),
    /// Tag 5: playback intent.
    Intent(Intent),
    /// Tag 6: drift report.
    ReportDrift(ReportDrift),
}

/// First message on every connection. Identifies the client and negotiates
/// capabilities. The room is selected by the transport binding, not here
/// (`docs/protocol.md` §2.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    /// Stable client identity; a second connection with the same id supersedes
    /// the first (`docs/protocol.md` §6.7).
    pub client_id: ClientId,
    /// Client capabilities.
    pub caps: Caps,
    /// Clock-sync preferences.
    pub clock_req: ClockRequest,
}

/// Clock probe. `t0` is the client's monotonic send time in nanoseconds; it is
/// opaque to the server and echoed back in [`ProbeReply`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Probe {
    /// Probe sequence number, chosen by the client.
    pub seq: u32,
    /// Client monotonic transmit timestamp (ns).
    pub t0: u64,
}

/// Readiness report used by group transitions (`docs/protocol.md` §6.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetReady {
    /// Whether the client can start playback at the current timeline position.
    pub ready: bool,
    /// Media position up to which the client has buffered, if known.
    pub buffered_until: Option<PosMs>,
}

/// Suggest a track to be appended to the queue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggest {
    /// The suggested content.
    pub track: Content,
    /// Client-chosen sequence number to correlate the resulting `QueueChanged`.
    pub client_seq: u32,
}

/// A playback intent (`docs/protocol.md` §6.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    /// What should happen.
    pub action: PlayAction,
    /// `None`: best effort, as soon as possible. `Some(t)`: take effect at
    /// room-clock time `t` so that all clients transition simultaneously.
    pub at_host_time: Option<HostTime>,
}

/// The action of an [`Intent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayAction {
    /// Tag 0: start or resume playback.
    Play,
    /// Tag 1: pause playback.
    Pause,
    /// Tag 2: group seek (`docs/protocol.md` §6.5).
    Seek {
        /// Target media position.
        pos_ms: PosMs,
    },
    /// Tag 3: advance to the next queue entry.
    SkipNext,
    /// Tag 4: change the playback rate.
    SetRate {
        /// New nominal rate.
        rate: Rate,
    },
}

/// Drift report: the client's measured position versus the position it
/// derived from the timeline at the same instant (`docs/protocol.md` §7.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportDrift {
    /// Report sequence number.
    pub seq: u32,
    /// Position actually rendered by the client's media clock.
    pub local_pos_ms: PosMs,
    /// Position derived from the timeline at the same instant.
    pub expected_pos_ms: PosMs,
}

// ---------------------------------------------------------------------------
// Server -> Client
// ---------------------------------------------------------------------------

/// Messages sent from the server to a client (`docs/protocol.md` §5.5).
///
/// Variant order is the wire tag; see the module docs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Tag 0: vendor extension (ignorable).
    Extension(Extension),
    /// Tag 1: reply to `Hello`, carrying the full room snapshot.
    Welcome(Welcome),
    /// Tag 2: reply to `Probe`.
    ProbeReply(ProbeReply),
    /// Tag 3: new timeline.
    Timeline(Timeline),
    /// Tag 4: new queue.
    QueueChanged(QueueChanged),
    /// Tag 5: authority moved.
    HostChanged(HostChanged),
    /// Tag 6: full state replacement.
    Resync(Resync),
    /// Tag 7: the server is about to restart.
    ServerRestarting(ServerRestarting),
}

/// Reply to [`Hello`]: the client is a member and receives a complete
/// snapshot (`docs/protocol.md` §6.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Welcome {
    /// Current session epoch.
    pub epoch: Epoch,
    /// Complete room state.
    pub snapshot: Snapshot,
}

/// Reply to [`Probe`]. `t1` / `t2` are the server's receive / transmit times on
/// the room clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeReply {
    /// Echo of [`Probe::seq`].
    pub seq: u32,
    /// Echo of [`Probe::t0`].
    pub t0: u64,
    /// Server receive time (room clock).
    pub t1: HostTime,
    /// Server transmit time (room clock).
    pub t2: HostTime,
}

/// The room timeline (ADR-0002). Clients derive the position locally:
/// `pos(t) = base_pos_ms + (t - base_host_time) * rate`. It always applies to
/// the head of the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timeline {
    /// Room-clock instant at which `base_pos_ms` is (or was) the position.
    pub base_host_time: HostTime,
    /// Media position at `base_host_time`.
    pub base_pos_ms: PosMs,
    /// Effective rate; `0` while paused or suspending.
    pub rate: Rate,
    /// Session revision at which this timeline was set.
    pub rev: Rev,
}

/// The queue was replaced. `tracks[0]` is the current track.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueChanged {
    /// The complete queue, current track first.
    pub tracks: Vec<QueueEntry>,
    /// Session revision at which this queue was set.
    pub rev: Rev,
}

/// Authority moved to another client (`docs/protocol.md` §3.4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostChanged {
    /// The new epoch (always greater than the previous one).
    pub epoch: Epoch,
    /// The client that now holds authority.
    pub new_host: ClientId,
    /// Why authority moved.
    pub reason: HostChangeReason,
}

/// Why authority moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostChangeReason {
    /// Tag 0: the host was silent for the failover timeout (30 s).
    HostSilent,
    /// Tag 1: the host left the room explicitly.
    HostLeft,
    /// Tag 2: the host handed authority over voluntarily.
    Handoff,
    /// Tag 3: a former host reclaimed authority after reconnecting.
    Reclaimed,
    /// Tag 4: server policy (moderation, rebalancing, restart recovery).
    ServerDecision,
}

/// Full state replacement. Clients MUST discard their session state and adopt
/// the snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resync {
    /// Complete room state.
    pub snapshot: Snapshot,
}

/// The server is about to restart. Clients MUST keep room state and reconnect
/// after `retry_after_ms` (`docs/protocol.md` §6.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRestarting {
    /// Suggested delay before reconnecting, in milliseconds.
    pub retry_after_ms: u32,
}

// ---------------------------------------------------------------------------
// Common types
// ---------------------------------------------------------------------------

/// Stable, client-generated identity (128 random bits). Opaque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClientId(pub [u8; 16]);

/// Opaque room identifier, assigned by the transport binding / server.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoomId(pub String);

/// Client capabilities advertised in [`Hello`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caps {
    /// Inclusive range of supported protocol versions.
    pub versions: VersionRange,
    /// Supported profiles.
    pub profiles: Vec<Profile>,
    /// Supported host modes.
    pub host_modes: Vec<HostMode>,
    /// Supported extension names (`x.<vendor>.<name>`).
    pub extensions: Vec<String>,
}

/// Inclusive protocol version range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRange {
    /// Lowest supported version.
    pub min: u16,
    /// Highest supported version.
    pub max: u16,
}

/// Protocol profile (ADR-0001).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Profile {
    /// Tag 0: timeline, queue and membership only (default).
    ControlOnly,
    /// Tag 1: control plus a self-hosted relay of time-stamped encoded chunks.
    ControlRelay,
}

/// Host model (`docs/protocol.md` §3.2). Only `SingleHost` is normative in v1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostMode {
    /// Tag 0: one client holds authority (normative).
    SingleHost,
    /// Tag 1: several clients share authority (reserved).
    CoHost,
    /// Tag 2: no client holds authority (reserved).
    Hostless,
}

/// Clock-sync preferences sent in [`Hello`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockRequest {
    /// How the client will send probes.
    pub transport: ProbeTransport,
    /// Maximum probe rate the client intends to use (Hz). Servers MAY rate
    /// limit; the protocol ceiling is 10 Hz.
    pub max_probe_hz: u8,
}

/// How probes are carried (`docs/protocol.md` §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeTransport {
    /// Tag 0: unreliable datagrams (preferred).
    Datagram,
    /// Tag 1: the reliable control stream (web fallback; lower quality).
    Stream,
}

/// Opaque content reference (ADR-0003).
///
/// `provider` and `id` are opaque: they MUST be compared byte-wise and MUST
/// NOT be parsed, normalized or required to follow any format.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Content {
    /// Opaque provider name (e.g. a service or `local`).
    pub provider: String,
    /// Opaque identifier within the provider.
    pub id: String,
    /// Advisory duration in milliseconds. The media's real length wins; never
    /// clamp positions to this value.
    pub duration_hint: Option<u64>,
    /// Optional content fingerprint to match local media.
    pub fingerprint: Option<Fingerprint>,
}

/// A content fingerprint.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fingerprint {
    /// Algorithm name (opaque, e.g. `chromaprint`, `sha256`).
    pub algorithm: String,
    /// Fingerprint bytes.
    pub value: Vec<u8>,
}

/// One entry of the room queue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueEntry {
    /// Server-assigned identifier, unique within the room.
    pub entry_id: u64,
    /// The content.
    pub content: Content,
    /// The client that suggested this entry, if known.
    pub added_by: Option<ClientId>,
}

/// Room playback state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    /// Tag 0: paused (`Timeline.rate == 0`).
    Paused,
    /// Tag 1: playing.
    Playing,
    /// Tag 2: group transition in progress (`Timeline.rate == 0`); playback
    /// resumes with a single scheduled `Play`.
    Suspending,
}

/// A room member as seen in a [`Snapshot`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    /// Member identity.
    pub client_id: ClientId,
    /// Last reported readiness.
    pub ready: bool,
    /// Last reported buffer horizon.
    pub buffered_until: Option<PosMs>,
}

/// Complete room state, sent on join and on resync. Late joiners MUST receive
/// one; incremental catch-up after join is prohibited (`docs/protocol.md` §6.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// The room.
    pub room: RoomId,
    /// Active profile.
    pub profile: Profile,
    /// Active host mode.
    pub host_mode: HostMode,
    /// Current authority, if any.
    pub host: Option<ClientId>,
    /// Playback state.
    pub state: PlaybackState,
    /// Current timeline (applies to `queue[0]`).
    pub timeline: Timeline,
    /// The complete queue, current track first.
    pub queue: Vec<QueueEntry>,
    /// Revision at which the queue was last changed.
    pub queue_rev: Rev,
    /// All members, including the receiver.
    pub members: Vec<Member>,
}

/// A vendor extension message (`docs/protocol.md` §5.7).
///
/// `name` MUST match `x.<vendor>.<name>`; see [`Extension::is_valid_name`].
/// Receivers that do not understand an extension MUST ignore it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extension {
    /// Namespaced extension name.
    pub name: String,
    /// Opaque, extension-defined payload.
    pub payload: Vec<u8>,
}

impl Extension {
    /// Maximum length of an extension name in bytes.
    pub const MAX_NAME_LEN: usize = 128;

    /// Validates an extension name: `x.<vendor>(.<segment>)+`, where every
    /// label is non-empty and made of ASCII lowercase letters, digits, `-` or
    /// `_`, and the whole name is at most [`Self::MAX_NAME_LEN`] bytes.
    #[must_use]
    pub fn is_valid_name(name: &str) -> bool {
        if name.len() > Self::MAX_NAME_LEN {
            return false;
        }
        let mut labels = name.split('.');
        if labels.next() != Some("x") {
            return false;
        }
        let mut count = 0usize;
        for label in labels {
            let ok = !label.is_empty()
                && label.bytes().all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'
                });
            if !ok {
                return false;
            }
            count += 1;
        }
        // vendor + at least one name segment
        count >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_names() {
        for ok in ["x.lmp.host.reclaim", "x.acme.lyrics", "x.a-b.c_d.e1"] {
            assert!(Extension::is_valid_name(ok), "{ok}");
        }
        for bad in [
            "",
            "x",
            "x.",
            "x.acme",
            "x.acme.",
            "y.acme.foo",
            "x..foo",
            "x.Acme.foo",
            "x.acme.fo o",
            "lmp.foo.bar",
        ] {
            assert!(!Extension::is_valid_name(bad), "{bad}");
        }
        let long = alloc::format!("x.v.{}", "a".repeat(Extension::MAX_NAME_LEN));
        assert!(!Extension::is_valid_name(&long));
    }
}
