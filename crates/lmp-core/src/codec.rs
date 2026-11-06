//! Canonical encoding of LMP envelopes (`docs/protocol.md` §5.2).
//!
//! The canonical encoding is [postcard](https://docs.rs/postcard) 1.x: varint
//! integers, length-prefixed strings and sequences, enum variants tagged by
//! declaration index, and no field names or padding. One encoded envelope is
//! one message; framing is the transport binding's job.
//!
//! Decoding is strict: the version is checked before the body is decoded,
//! trailing bytes are rejected, and extension names are validated.

use alloc::sync::Arc;
use alloc::vec::Vec;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::message::{ClientMessage, Envelope, Extension, PROTOCOL_VERSION, ServerMessage};

/// Maximum encoded size of a single envelope, in bytes.
pub const MAX_MESSAGE_SIZE: usize = 256 * 1024;

/// Errors produced by [`encode`] and [`decode`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodecError {
    /// The message exceeds [`MAX_MESSAGE_SIZE`].
    #[error("message of {len} bytes exceeds the maximum of {max} bytes")]
    TooLarge {
        /// Actual size.
        len: usize,
        /// Allowed maximum.
        max: usize,
    },
    /// The envelope carries a protocol version this implementation does not
    /// speak.
    #[error("unsupported protocol version {got} (supported: {supported})")]
    UnsupportedVersion {
        /// Version found on the wire.
        got: u16,
        /// Version supported by this implementation.
        supported: u16,
    },
    /// Bytes remained after a complete envelope was decoded.
    #[error("{0} trailing bytes after envelope")]
    TrailingBytes(usize),
    /// An extension name does not match `x.<vendor>.<name>`.
    #[error("invalid extension name")]
    InvalidExtensionName,
    /// The bytes are not a valid envelope.
    #[error("malformed message: {0}")]
    Malformed(postcard::Error),
}

/// Message bodies that can be validated beyond their structure.
pub trait Body: Serialize + DeserializeOwned {
    /// Checks semantic constraints that the type system cannot express.
    fn validate(&self) -> Result<(), CodecError>;
}

fn validate_extension(ext: &Extension) -> Result<(), CodecError> {
    if Extension::is_valid_name(&ext.name) {
        Ok(())
    } else {
        Err(CodecError::InvalidExtensionName)
    }
}

impl Body for ClientMessage {
    fn validate(&self) -> Result<(), CodecError> {
        match self {
            Self::Extension(ext) => validate_extension(ext),
            _ => Ok(()),
        }
    }
}

impl Body for ServerMessage {
    fn validate(&self) -> Result<(), CodecError> {
        match self {
            Self::Extension(ext) => validate_extension(ext),
            _ => Ok(()),
        }
    }
}

/// Encodes an envelope into its canonical byte representation.
///
/// # Errors
///
/// Fails if the envelope does not validate or exceeds [`MAX_MESSAGE_SIZE`].
pub fn encode<B: Body>(envelope: &Envelope<B>) -> Result<Vec<u8>, CodecError> {
    envelope.body.validate()?;
    let bytes = postcard::to_allocvec(envelope).map_err(CodecError::Malformed)?;
    if bytes.len() > MAX_MESSAGE_SIZE {
        return Err(CodecError::TooLarge {
            len: bytes.len(),
            max: MAX_MESSAGE_SIZE,
        });
    }
    Ok(bytes)
}

/// Encodes an envelope once into a shared buffer suitable for zero-copy fanout
/// to many connections (ADR-0006).
///
/// # Errors
///
/// Same as [`encode`].
pub fn encode_shared<B: Body>(envelope: &Envelope<B>) -> Result<Arc<[u8]>, CodecError> {
    encode(envelope).map(Arc::from)
}

/// Decodes one envelope from `bytes`, which must contain exactly one message.
///
/// # Errors
///
/// Fails on oversized input, an unsupported version, malformed bytes, trailing
/// bytes, or an invalid extension name.
pub fn decode<B: Body>(bytes: &[u8]) -> Result<Envelope<B>, CodecError> {
    if bytes.len() > MAX_MESSAGE_SIZE {
        return Err(CodecError::TooLarge {
            len: bytes.len(),
            max: MAX_MESSAGE_SIZE,
        });
    }
    // `ver` is the first field: check it before interpreting the body, whose
    // layout may differ between versions.
    let (ver, _) = postcard::take_from_bytes::<u16>(bytes).map_err(CodecError::Malformed)?;
    if ver != PROTOCOL_VERSION {
        return Err(CodecError::UnsupportedVersion {
            got: ver,
            supported: PROTOCOL_VERSION,
        });
    }
    let (envelope, rest) =
        postcard::take_from_bytes::<Envelope<B>>(bytes).map_err(CodecError::Malformed)?;
    if !rest.is_empty() {
        return Err(CodecError::TrailingBytes(rest.len()));
    }
    envelope.body.validate()?;
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{ClientEnvelope, Probe, ServerEnvelope, ServerRestarting};
    use crate::time::{Epoch, Rev};

    fn probe() -> ClientEnvelope {
        Envelope::new(
            Epoch(1),
            Rev(2),
            ClientMessage::Probe(Probe { seq: 3, t0: 4 }),
        )
    }

    #[test]
    fn roundtrip_probe() {
        let bytes = encode(&probe()).unwrap();
        assert_eq!(bytes, [0x00, 0x01, 0x02, 0x02, 0x03, 0x04]);
        assert_eq!(decode::<ClientMessage>(&bytes).unwrap(), probe());
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = encode(&probe()).unwrap();
        bytes.push(0);
        assert_eq!(
            decode::<ClientMessage>(&bytes),
            Err(CodecError::TrailingBytes(1))
        );
    }

    #[test]
    fn rejects_unknown_version() {
        let mut env = probe();
        env.ver = 7;
        let bytes = postcard::to_allocvec(&env).unwrap();
        assert_eq!(
            decode::<ClientMessage>(&bytes),
            Err(CodecError::UnsupportedVersion {
                got: 7,
                supported: PROTOCOL_VERSION
            })
        );
    }

    #[test]
    fn rejects_truncated() {
        let bytes = encode(&probe()).unwrap();
        for n in 0..bytes.len() {
            assert!(matches!(
                decode::<ClientMessage>(&bytes[..n]),
                Err(CodecError::Malformed(_))
            ));
        }
    }

    #[test]
    fn rejects_unknown_tag() {
        // ver=0, epoch=0, rev=0, tag=99
        assert!(matches!(
            decode::<ServerMessage>(&[0, 0, 0, 99]),
            Err(CodecError::Malformed(_))
        ));
    }

    #[test]
    fn rejects_invalid_extension_name_both_ways() {
        let env: ServerEnvelope = Envelope::new(
            Epoch(0),
            Rev(0),
            ServerMessage::Extension(Extension {
                name: "lmp.bad".into(),
                payload: vec![],
            }),
        );
        assert_eq!(encode(&env), Err(CodecError::InvalidExtensionName));
        let raw = postcard::to_allocvec(&env).unwrap();
        assert_eq!(
            decode::<ServerMessage>(&raw),
            Err(CodecError::InvalidExtensionName)
        );
    }

    #[test]
    fn rejects_oversized() {
        let big = vec![0u8; MAX_MESSAGE_SIZE + 1];
        assert!(matches!(
            decode::<ClientMessage>(&big),
            Err(CodecError::TooLarge { .. })
        ));
        let env: ServerEnvelope = Envelope::new(
            Epoch(0),
            Rev(0),
            ServerMessage::Extension(Extension {
                name: "x.test.big".into(),
                payload: vec![0; MAX_MESSAGE_SIZE],
            }),
        );
        assert!(matches!(encode(&env), Err(CodecError::TooLarge { .. })));
    }

    #[test]
    fn shared_encoding_matches() {
        let env: ServerEnvelope = Envelope::new(
            Epoch(9),
            Rev(9),
            ServerMessage::ServerRestarting(ServerRestarting {
                retry_after_ms: 1500,
            }),
        );
        let shared = encode_shared(&env).unwrap();
        assert_eq!(&*shared, encode(&env).unwrap().as_slice());
    }
}
