//! Property-based round-trip tests: `decode(encode(m)) == m` for arbitrary
//! messages, and decoding arbitrary bytes never panics.

// Test helpers outside `#[test]` functions are not covered by clippy's
// `allow-*-in-tests`; panicking is the desired failure mode here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation
)]

use lmp_core::codec::{decode, encode};
use lmp_core::message::*;
use lmp_core::time::{Epoch, HostTime, PosMs, Rate, Rev};
use proptest::collection::vec;
use proptest::option;
use proptest::prelude::*;

fn client_id() -> impl Strategy<Value = ClientId> {
    any::<[u8; 16]>().prop_map(ClientId)
}

fn ext_name() -> impl Strategy<Value = String> {
    "x\\.[a-z0-9_-]{1,12}(\\.[a-z0-9_-]{1,12}){1,3}"
}

fn content() -> impl Strategy<Value = Content> {
    (
        ".{0,24}",
        ".{0,48}",
        option::of(any::<u64>()),
        option::of((".{0,12}", vec(any::<u8>(), 0..40))),
    )
        .prop_map(|(provider, id, duration_hint, fp)| Content {
            provider,
            id,
            duration_hint,
            fingerprint: fp.map(|(algorithm, value)| Fingerprint { algorithm, value }),
        })
}

fn timeline() -> impl Strategy<Value = Timeline> {
    (any::<u64>(), any::<u64>(), any::<u32>(), any::<u64>()).prop_map(|(t, p, r, rev)| Timeline {
        base_host_time: HostTime(t),
        base_pos_ms: PosMs(p),
        rate: Rate(r),
        rev: Rev(rev),
    })
}

fn queue_entry() -> impl Strategy<Value = QueueEntry> {
    (any::<u64>(), content(), option::of(client_id())).prop_map(|(entry_id, content, added_by)| {
        QueueEntry {
            entry_id,
            content,
            added_by,
        }
    })
}

fn profile() -> impl Strategy<Value = Profile> {
    prop_oneof![Just(Profile::ControlOnly), Just(Profile::ControlRelay)]
}

fn host_mode() -> impl Strategy<Value = HostMode> {
    prop_oneof![
        Just(HostMode::SingleHost),
        Just(HostMode::CoHost),
        Just(HostMode::Hostless)
    ]
}

fn snapshot() -> impl Strategy<Value = Snapshot> {
    (
        ".{0,16}",
        profile(),
        host_mode(),
        option::of(client_id()),
        prop_oneof![
            Just(PlaybackState::Paused),
            Just(PlaybackState::Playing),
            Just(PlaybackState::Suspending)
        ],
        timeline(),
        vec(queue_entry(), 0..4),
        any::<u64>(),
        vec(
            (client_id(), any::<bool>(), option::of(any::<u64>())).prop_map(|(c, r, b)| Member {
                client_id: c,
                ready: r,
                buffered_until: b.map(PosMs),
            }),
            0..6,
        ),
    )
        .prop_map(
            |(room, profile, host_mode, host, state, timeline, queue, qrev, members)| Snapshot {
                room: RoomId(room),
                profile,
                host_mode,
                host,
                state,
                timeline,
                queue,
                queue_rev: Rev(qrev),
                members,
            },
        )
}

fn play_action() -> impl Strategy<Value = PlayAction> {
    prop_oneof![
        Just(PlayAction::Play),
        Just(PlayAction::Pause),
        any::<u64>().prop_map(|p| PlayAction::Seek { pos_ms: PosMs(p) }),
        Just(PlayAction::SkipNext),
        any::<u32>().prop_map(|r| PlayAction::SetRate { rate: Rate(r) }),
    ]
}

fn extension() -> impl Strategy<Value = Extension> {
    (ext_name(), vec(any::<u8>(), 0..64)).prop_map(|(name, payload)| Extension { name, payload })
}

fn client_message() -> impl Strategy<Value = ClientMessage> {
    prop_oneof![
        extension().prop_map(ClientMessage::Extension),
        (
            client_id(),
            any::<(u16, u16)>(),
            vec(profile(), 0..3),
            vec(host_mode(), 0..3),
            vec(ext_name(), 0..3),
            any::<bool>(),
            any::<u8>()
        )
            .prop_map(
                |(client_id, (min, max), profiles, host_modes, extensions, dg, hz)| {
                    ClientMessage::Hello(Hello {
                        client_id,
                        caps: Caps {
                            versions: VersionRange { min, max },
                            profiles,
                            host_modes,
                            extensions,
                        },
                        clock_req: ClockRequest {
                            transport: if dg {
                                ProbeTransport::Datagram
                            } else {
                                ProbeTransport::Stream
                            },
                            max_probe_hz: hz,
                        },
                    })
                }
            ),
        (any::<u32>(), any::<u64>()).prop_map(|(seq, t0)| ClientMessage::Probe(Probe { seq, t0 })),
        (any::<bool>(), option::of(any::<u64>())).prop_map(|(ready, b)| {
            ClientMessage::SetReady(SetReady {
                ready,
                buffered_until: b.map(PosMs),
            })
        }),
        (content(), any::<u32>())
            .prop_map(|(track, client_seq)| ClientMessage::Suggest(Suggest { track, client_seq })),
        (play_action(), option::of(any::<u64>())).prop_map(|(action, at)| {
            ClientMessage::Intent(Intent {
                action,
                at_host_time: at.map(HostTime),
            })
        }),
        (any::<u32>(), any::<u64>(), any::<u64>()).prop_map(|(seq, l, e)| {
            ClientMessage::ReportDrift(ReportDrift {
                seq,
                local_pos_ms: PosMs(l),
                expected_pos_ms: PosMs(e),
            })
        }),
    ]
}

fn server_message() -> impl Strategy<Value = ServerMessage> {
    let reason = prop_oneof![
        Just(HostChangeReason::HostSilent),
        Just(HostChangeReason::HostLeft),
        Just(HostChangeReason::Handoff),
        Just(HostChangeReason::Reclaimed),
        Just(HostChangeReason::ServerDecision),
    ];
    prop_oneof![
        extension().prop_map(ServerMessage::Extension),
        (any::<u64>(), snapshot()).prop_map(|(e, snapshot)| ServerMessage::Welcome(Welcome {
            epoch: Epoch(e),
            snapshot
        })),
        (any::<u32>(), any::<u64>(), any::<u64>(), any::<u64>()).prop_map(|(seq, t0, t1, t2)| {
            ServerMessage::ProbeReply(ProbeReply {
                seq,
                t0,
                t1: HostTime(t1),
                t2: HostTime(t2),
            })
        }),
        timeline().prop_map(ServerMessage::Timeline),
        (vec(queue_entry(), 0..5), any::<u64>()).prop_map(|(tracks, r)| {
            ServerMessage::QueueChanged(QueueChanged {
                tracks,
                rev: Rev(r),
            })
        }),
        (any::<u64>(), client_id(), reason).prop_map(|(e, new_host, reason)| {
            ServerMessage::HostChanged(HostChanged {
                epoch: Epoch(e),
                new_host,
                reason,
            })
        }),
        snapshot().prop_map(|snapshot| ServerMessage::Resync(Resync { snapshot })),
        any::<u32>().prop_map(|retry_after_ms| {
            ServerMessage::ServerRestarting(ServerRestarting { retry_after_ms })
        }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    #[test]
    fn client_roundtrip(epoch: u64, rev: u64, body in client_message()) {
        let env = Envelope::new(Epoch(epoch), Rev(rev), body);
        let bytes = encode(&env).unwrap();
        prop_assert_eq!(decode::<ClientMessage>(&bytes).unwrap(), env);
    }

    #[test]
    fn server_roundtrip(epoch: u64, rev: u64, body in server_message()) {
        let env = Envelope::new(Epoch(epoch), Rev(rev), body);
        let bytes = encode(&env).unwrap();
        prop_assert_eq!(decode::<ServerMessage>(&bytes).unwrap(), env);
    }

    #[test]
    fn decode_never_panics(bytes in vec(any::<u8>(), 0..256)) {
        let _ = decode::<ClientMessage>(&bytes);
        let _ = decode::<ServerMessage>(&bytes);
    }
}
