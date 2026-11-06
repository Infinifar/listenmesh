//! Byte-exact golden vectors for the LMP wire format (version 0).
//!
//! The fixture file `tests/golden/v0.hex` is the normative reference for the
//! canonical encoding (`docs/protocol.md` §5.2). Every core message variant is
//! covered. To regenerate after an *intentional* wire change:
//!
//! ```sh
//! LMP_BLESS=1 cargo test -p lmp-core --test golden
//! ```
//!
//! and update `docs/protocol.md` in the same change.

// Test helpers outside `#[test]` functions are not covered by clippy's
// `allow-*-in-tests`; panicking is the desired failure mode here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use lmp_core::codec::{decode, encode};
use lmp_core::message::*;
use lmp_core::time::{Epoch, HostTime, PosMs, Rate, Rev};

const FIXTURE: &str = "tests/golden/v0.hex";

fn cid(b: u8) -> ClientId {
    let mut id = [0u8; 16];
    for (i, x) in id.iter_mut().enumerate() {
        *x = b.wrapping_add(i as u8);
    }
    ClientId(id)
}

fn content_full() -> Content {
    Content {
        provider: "local".into(),
        id: "Ünïcode/opaque id?#1".into(),
        duration_hint: Some(215_000),
        fingerprint: Some(Fingerprint {
            algorithm: "sha256".into(),
            value: (0u8..32).collect(),
        }),
    }
}

fn content_min() -> Content {
    Content {
        provider: "x".into(),
        id: String::new(),
        duration_hint: None,
        fingerprint: None,
    }
}

fn timeline() -> Timeline {
    Timeline {
        base_host_time: HostTime(86_400_123_456_789),
        base_pos_ms: PosMs(42_000),
        rate: Rate::NORMAL,
        rev: Rev(17),
    }
}

fn queue() -> Vec<QueueEntry> {
    vec![
        QueueEntry {
            entry_id: 1,
            content: content_full(),
            added_by: Some(cid(0x10)),
        },
        QueueEntry {
            entry_id: 300,
            content: content_min(),
            added_by: None,
        },
    ]
}

fn snapshot() -> Snapshot {
    Snapshot {
        room: RoomId("room-7f3a".into()),
        profile: Profile::ControlOnly,
        host_mode: HostMode::SingleHost,
        host: Some(cid(0x10)),
        state: PlaybackState::Playing,
        timeline: timeline(),
        queue: queue(),
        queue_rev: Rev(16),
        members: vec![
            Member {
                client_id: cid(0x10),
                ready: true,
                buffered_until: Some(PosMs(60_000)),
            },
            Member {
                client_id: cid(0x20),
                ready: false,
                buffered_until: None,
            },
        ],
    }
}

fn client_vectors() -> Vec<(&'static str, ClientEnvelope)> {
    let e = |epoch, rev, body| Envelope::new(Epoch(epoch), Rev(rev), body);
    vec![
        (
            "client.extension",
            e(
                3,
                5,
                ClientMessage::Extension(Extension {
                    name: "x.lmp.host.reclaim".into(),
                    payload: vec![],
                }),
            ),
        ),
        (
            "client.hello",
            e(
                0,
                0,
                ClientMessage::Hello(Hello {
                    client_id: cid(0xA0),
                    caps: Caps {
                        versions: VersionRange { min: 0, max: 0 },
                        profiles: vec![Profile::ControlOnly, Profile::ControlRelay],
                        host_modes: vec![HostMode::SingleHost],
                        extensions: vec!["x.lmp.host.reclaim".into()],
                    },
                    clock_req: ClockRequest {
                        transport: ProbeTransport::Datagram,
                        max_probe_hz: 10,
                    },
                }),
            ),
        ),
        (
            "client.hello.minimal",
            e(
                0,
                0,
                ClientMessage::Hello(Hello {
                    client_id: cid(0),
                    caps: Caps {
                        versions: VersionRange { min: 0, max: 0 },
                        profiles: vec![],
                        host_modes: vec![],
                        extensions: vec![],
                    },
                    clock_req: ClockRequest {
                        transport: ProbeTransport::Stream,
                        max_probe_hz: 1,
                    },
                }),
            ),
        ),
        (
            "client.probe",
            e(
                3,
                5,
                ClientMessage::Probe(Probe {
                    seq: 1,
                    t0: 1_000_000_007,
                }),
            ),
        ),
        (
            "client.probe.max",
            e(
                u64::MAX,
                u64::MAX,
                ClientMessage::Probe(Probe {
                    seq: u32::MAX,
                    t0: u64::MAX,
                }),
            ),
        ),
        (
            "client.set_ready",
            e(
                3,
                5,
                ClientMessage::SetReady(SetReady {
                    ready: true,
                    buffered_until: Some(PosMs(52_000)),
                }),
            ),
        ),
        (
            "client.set_ready.not",
            e(
                3,
                5,
                ClientMessage::SetReady(SetReady {
                    ready: false,
                    buffered_until: None,
                }),
            ),
        ),
        (
            "client.suggest",
            e(
                3,
                5,
                ClientMessage::Suggest(Suggest {
                    track: content_full(),
                    client_seq: 9,
                }),
            ),
        ),
        (
            "client.suggest.minimal",
            e(
                3,
                5,
                ClientMessage::Suggest(Suggest {
                    track: content_min(),
                    client_seq: 0,
                }),
            ),
        ),
        (
            "client.intent.play_now",
            e(
                3,
                5,
                ClientMessage::Intent(Intent {
                    action: PlayAction::Play,
                    at_host_time: None,
                }),
            ),
        ),
        (
            "client.intent.pause_at",
            e(
                3,
                5,
                ClientMessage::Intent(Intent {
                    action: PlayAction::Pause,
                    at_host_time: Some(HostTime(86_400_500_000_000)),
                }),
            ),
        ),
        (
            "client.intent.seek",
            e(
                3,
                5,
                ClientMessage::Intent(Intent {
                    action: PlayAction::Seek {
                        pos_ms: PosMs(90_500),
                    },
                    at_host_time: None,
                }),
            ),
        ),
        (
            "client.intent.skip_next",
            e(
                3,
                5,
                ClientMessage::Intent(Intent {
                    action: PlayAction::SkipNext,
                    at_host_time: None,
                }),
            ),
        ),
        (
            "client.intent.set_rate",
            e(
                3,
                5,
                ClientMessage::Intent(Intent {
                    action: PlayAction::SetRate {
                        rate: Rate(1_250_000),
                    },
                    at_host_time: Some(HostTime(1)),
                }),
            ),
        ),
        (
            "client.report_drift",
            e(
                3,
                5,
                ClientMessage::ReportDrift(ReportDrift {
                    seq: 44,
                    local_pos_ms: PosMs(61_012),
                    expected_pos_ms: PosMs(61_000),
                }),
            ),
        ),
    ]
}

fn server_vectors() -> Vec<(&'static str, ServerEnvelope)> {
    let e = |epoch, rev, body| Envelope::new(Epoch(epoch), Rev(rev), body);
    vec![
        (
            "server.extension",
            e(
                3,
                17,
                ServerMessage::Extension(Extension {
                    name: "x.acme.lyrics.line".into(),
                    payload: b"hello".to_vec(),
                }),
            ),
        ),
        (
            "server.welcome",
            e(
                3,
                17,
                ServerMessage::Welcome(Welcome {
                    epoch: Epoch(3),
                    snapshot: snapshot(),
                }),
            ),
        ),
        (
            "server.probe_reply",
            e(
                3,
                17,
                ServerMessage::ProbeReply(ProbeReply {
                    seq: 1,
                    t0: 1_000_000_007,
                    t1: HostTime(86_400_000_250_000),
                    t2: HostTime(86_400_000_260_000),
                }),
            ),
        ),
        (
            "server.timeline",
            e(3, 17, ServerMessage::Timeline(timeline())),
        ),
        (
            "server.timeline.paused",
            e(
                3,
                18,
                ServerMessage::Timeline(Timeline {
                    base_host_time: HostTime(0),
                    base_pos_ms: PosMs(0),
                    rate: Rate::ZERO,
                    rev: Rev(18),
                }),
            ),
        ),
        (
            "server.queue_changed",
            e(
                3,
                16,
                ServerMessage::QueueChanged(QueueChanged {
                    tracks: queue(),
                    rev: Rev(16),
                }),
            ),
        ),
        (
            "server.queue_changed.empty",
            e(
                3,
                19,
                ServerMessage::QueueChanged(QueueChanged {
                    tracks: vec![],
                    rev: Rev(19),
                }),
            ),
        ),
        (
            "server.host_changed.silent",
            e(
                4,
                20,
                ServerMessage::HostChanged(HostChanged {
                    epoch: Epoch(4),
                    new_host: cid(0x20),
                    reason: HostChangeReason::HostSilent,
                }),
            ),
        ),
        (
            "server.host_changed.reclaimed",
            e(
                5,
                21,
                ServerMessage::HostChanged(HostChanged {
                    epoch: Epoch(5),
                    new_host: cid(0x10),
                    reason: HostChangeReason::Reclaimed,
                }),
            ),
        ),
        (
            "server.resync",
            e(
                3,
                17,
                ServerMessage::Resync(Resync {
                    snapshot: snapshot(),
                }),
            ),
        ),
        (
            "server.server_restarting",
            e(
                3,
                17,
                ServerMessage::ServerRestarting(ServerRestarting {
                    retry_after_ms: 2_500,
                }),
            ),
        ),
    ]
}

fn client_tag(m: &ClientMessage) -> usize {
    match m {
        ClientMessage::Extension(_) => 0,
        ClientMessage::Hello(_) => 1,
        ClientMessage::Probe(_) => 2,
        ClientMessage::SetReady(_) => 3,
        ClientMessage::Suggest(_) => 4,
        ClientMessage::Intent(_) => 5,
        ClientMessage::ReportDrift(_) => 6,
    }
}

fn server_tag(m: &ServerMessage) -> usize {
    match m {
        ServerMessage::Extension(_) => 0,
        ServerMessage::Welcome(_) => 1,
        ServerMessage::ProbeReply(_) => 2,
        ServerMessage::Timeline(_) => 3,
        ServerMessage::QueueChanged(_) => 4,
        ServerMessage::HostChanged(_) => 5,
        ServerMessage::Resync(_) => 6,
        ServerMessage::ServerRestarting(_) => 7,
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

fn from_hex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "odd hex length");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

fn encoded_vectors() -> BTreeMap<&'static str, Vec<u8>> {
    let mut map = BTreeMap::new();
    for (name, env) in client_vectors() {
        assert!(
            map.insert(name, encode(&env).unwrap()).is_none(),
            "duplicate {name}"
        );
    }
    for (name, env) in server_vectors() {
        assert!(
            map.insert(name, encode(&env).unwrap()).is_none(),
            "duplicate {name}"
        );
    }
    map
}

fn read_fixture() -> BTreeMap<String, Vec<u8>> {
    let text =
        std::fs::read_to_string(fixture_path()).expect("fixture missing; run with LMP_BLESS=1");
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (name, hex) = l.split_once(' ').expect("line format: <name> <hex>");
            (name.to_owned(), from_hex(hex.trim()))
        })
        .collect()
}

#[test]
fn every_variant_is_covered() {
    let mut c = [false; 7];
    for (_, env) in client_vectors() {
        c[client_tag(&env.body)] = true;
    }
    let mut s = [false; 8];
    for (_, env) in server_vectors() {
        s[server_tag(&env.body)] = true;
    }
    assert!(c.iter().all(|x| *x), "client coverage: {c:?}");
    assert!(s.iter().all(|x| *x), "server coverage: {s:?}");
}

#[test]
fn golden_vectors_are_byte_exact() {
    let actual = encoded_vectors();
    if std::env::var_os("LMP_BLESS").is_some() {
        let mut out = String::from(
            "# ListenMesh Protocol (LMP) golden vectors — wire version 0 (draft).\n\
             # Normative for docs/protocol.md §5.2. Format: <name> <hex>\n\
             # Regenerate (intentional wire changes only):\n\
             #   LMP_BLESS=1 cargo test -p lmp-core --test golden\n",
        );
        for (name, bytes) in &actual {
            writeln!(out, "{name} {}", to_hex(bytes)).unwrap();
        }
        std::fs::create_dir_all(fixture_path().parent().unwrap()).unwrap();
        std::fs::write(fixture_path(), out).unwrap();
        return;
    }
    let expected = read_fixture();
    let expected_names: Vec<&str> = expected.keys().map(String::as_str).collect();
    let actual_names: Vec<&str> = actual.keys().copied().collect();
    assert_eq!(expected_names, actual_names, "vector set changed");
    for (name, bytes) in &actual {
        assert_eq!(
            to_hex(bytes),
            to_hex(&expected[*name]),
            "wire encoding of `{name}` changed; see docs/protocol.md §9 before blessing"
        );
    }
}

#[test]
fn golden_vectors_decode_to_values() {
    let fixture = read_fixture();
    for (name, env) in client_vectors() {
        assert_eq!(
            decode::<ClientMessage>(&fixture[name]).unwrap(),
            env,
            "{name}"
        );
    }
    for (name, env) in server_vectors() {
        assert_eq!(
            decode::<ServerMessage>(&fixture[name]).unwrap(),
            env,
            "{name}"
        );
    }
}

/// A hand-checked vector, independent of the fixture file, documenting the
/// encoding rules in `docs/protocol.md` §5.2.
#[test]
fn hand_decoded_timeline() {
    let env = Envelope::new(
        Epoch(3),
        Rev(17),
        ServerMessage::Timeline(Timeline {
            base_host_time: HostTime(300),
            base_pos_ms: PosMs(42_000),
            rate: Rate::NORMAL,
            rev: Rev(17),
        }),
    );
    let expected: &[u8] = &[
        0x00, // ver = 0
        0x03, // epoch = 3
        0x11, // rev = 17
        0x03, // tag 3 = Timeline
        0xac, 0x02, // base_host_time = 300 (varint)
        0x90, 0xc8, 0x02, // base_pos_ms = 42000
        0xc0, 0x84, 0x3d, // rate = 1_000_000
        0x11, // rev = 17
    ];
    assert_eq!(encode(&env).unwrap(), expected);
}
