//! Transport binding abstraction (`docs/protocol.md` §2.2).
//!
//! No transport is implemented yet. QUIC (M1) and WebTransport/WebSocket
//! (M4, #14) bindings will implement [`Transport`].

/// Transport kinds in negotiation order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportKind {
    /// Native QUIC: datagrams for probes, streams for control.
    Quic,
    /// Browser WebTransport.
    WebTransport,
    /// Browser fallback: probes share the control channel.
    WebSocket,
}

/// Transport errors.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransportError {
    /// The connection closed.
    #[error("connection closed (code {code})")]
    Closed {
        /// Application close code (1012 = service restart).
        code: u32,
    },
}

/// A connected transport carrying encoded LMP envelopes.
pub trait Transport {
    /// Which binding this is.
    fn kind(&self) -> TransportKind;

    /// Sends one encoded envelope on the reliable control channel.
    ///
    /// # Errors
    ///
    /// The connection is closed.
    fn send_control(&mut self, bytes: &[u8]) -> Result<(), TransportError>;

    /// Sends one encoded probe, unreliably if the binding supports it.
    ///
    /// # Errors
    ///
    /// The connection is closed.
    fn send_probe(&mut self, bytes: &[u8]) -> Result<(), TransportError>;
}
