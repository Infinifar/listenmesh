//! Room → node placement with consistent hashing (`docs/roadmap.md` M4).

use lmp_core::message::RoomId;

/// Opaque node identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

/// A consistent-hash ring of nodes.
#[derive(Clone, Debug, Default)]
pub struct Ring {
    nodes: Vec<NodeId>,
}

impl Ring {
    /// Nodes currently on the ring.
    #[must_use]
    pub fn nodes(&self) -> &[NodeId] {
        &self.nodes
    }

    /// The node responsible for `room`.
    #[must_use]
    pub fn node_for(&self, _room: &RoomId) -> Option<&NodeId> {
        // TODO(M4, #13): consistent hashing with virtual nodes.
        todo!("sharding (M4, #13)")
    }
}
