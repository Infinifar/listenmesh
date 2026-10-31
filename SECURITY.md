# Security Policy

## Status

ListenMesh is at milestone **M0 (scaffolding)**. There are no releases and no
deployed reference server yet. Nothing in this repository should be exposed to
untrusted networks.

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

Report privately via GitHub's
[private vulnerability reporting](https://github.com/Infinifar/listenmesh/security/advisories/new).
Include:

- affected component (specification section or crate),
- a description of the issue and its impact,
- a reproduction (test case, byte sequence in hex, or message trace).

We aim to acknowledge reports within 7 days. Protocol-level issues may require a
specification change and coordinated disclosure with known implementers; we
will agree on a timeline with you.

## Scope

In scope: the protocol specification (`docs/`), and all crates in `crates/`.
The threat model is documented in [`docs/threat-model.md`](docs/threat-model.md);
issues it lists as "open" are known and tracked publicly.

## Supported versions

| Version | Supported |
|---|---|
| `main` (pre-release) | best effort |
