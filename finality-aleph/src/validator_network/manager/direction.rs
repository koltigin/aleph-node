use std::{
    collections::{HashMap, HashSet},
    ops::BitXor,
};

use aleph_primitives::AuthorityId;

use crate::validator_network::Data;

/// Data about peers we know and whether we should connect to them or they to us. For the former
/// case also keeps the peers' addresses.
pub struct DirectedPeers<A: Data> {
    own_id: AuthorityId,
    outgoing: HashMap<AuthorityId, Vec<A>>,
    incoming: HashSet<AuthorityId>,
}

/// Whether we should call the remote or the other way around. We xor the peer ids and based on the
/// parity of the sum of bits of the result decide whether the caller should be the smaller or
/// greated lexicographically. They are never equal, because cryptography.
fn should_we_call(own_id: &[u8], remote_id: &[u8]) -> bool {
    let xor_sum_parity = (own_id.iter().fold(0u8, BitXor::bitxor)
        ^ remote_id.iter().fold(0u8, BitXor::bitxor))
    .count_ones()
        % 2;
    match xor_sum_parity == 0 {
        true => own_id < remote_id,
        false => own_id > remote_id,
    }
}

impl<A: Data> DirectedPeers<A> {
    /// Create a new set of peers directed using our own peer id.
    pub fn new(own_id: AuthorityId) -> Self {
        DirectedPeers {
            own_id,
            outgoing: HashMap::new(),
            incoming: HashSet::new(),
        }
    }

    /// Add a peer to the list of peers we want to stay connected to, or
    /// update the list of addresses if the peer was already added.
    /// Returns whether we should start attempts at connecting with the peer, which is the case
    /// exactly when the peer is one with which we should attempt connections AND it was added for
    /// the first time.
    pub fn add_peer(&mut self, peer_id: AuthorityId, addresses: Vec<A>) -> bool {
        match should_we_call(self.own_id.as_ref(), peer_id.as_ref()) {
            true => self.outgoing.insert(peer_id, addresses).is_none(),
            false => {
                // We discard the addresses here, as we will never want to call this peer anyway,
                // so we don't need them.
                self.incoming.insert(peer_id);
                false
            }
        }
    }

    /// Return the addresses of the given peer, or None if we shouldn't attempt connecting with the peer.
    pub fn peer_addresses(&self, peer_id: &AuthorityId) -> Option<Vec<A>> {
        self.outgoing.get(peer_id).cloned()
    }

    /// Whether we should be maintaining a connection with this peer.
    pub fn interested(&self, peer_id: &AuthorityId) -> bool {
        self.incoming.contains(peer_id) || self.outgoing.contains_key(peer_id)
    }

    /// Iterator over the peers we want connections from.
    pub fn incoming_peers(&self) -> impl Iterator<Item = &AuthorityId> {
        self.incoming.iter()
    }

    /// Iterator over the peers we want to connect to.
    pub fn outgoing_peers(&self) -> impl Iterator<Item = &AuthorityId> {
        self.outgoing.keys()
    }

    /// Remove a peer from the list of peers that we want to stay connected with, whether the
    /// connection was supposed to be incoming or outgoing.
    pub fn remove_peer(&mut self, peer_id: &AuthorityId) {
        self.incoming.remove(peer_id);
        self.outgoing.remove(peer_id);
    }
}

#[cfg(test)]
mod tests {
    use aleph_primitives::AuthorityId;

    use super::DirectedPeers;
    use crate::validator_network::mock::key;

    type Address = String;

    async fn container_with_id() -> (DirectedPeers<Address>, AuthorityId) {
        let (id, _) = key().await;
        let container = DirectedPeers::new(id.clone());
        (container, id)
    }

    fn some_addresses() -> Vec<Address> {
        vec![
            String::from(""),
            String::from("a/b/c"),
            String::from("43.43.43.43:43000"),
        ]
    }

    #[tokio::test]
    async fn exactly_one_direction_attempts_connections() {
        let (mut container0, id0) = container_with_id().await;
        let (mut container1, id1) = container_with_id().await;
        let addresses = some_addresses();
        assert!(
            container0.add_peer(id1, addresses.clone())
                != container1.add_peer(id0, addresses.clone())
        );
    }

    async fn container_with_added_connecting_peer() -> (DirectedPeers<Address>, AuthorityId) {
        let (mut container0, id0) = container_with_id().await;
        let (mut container1, id1) = container_with_id().await;
        let addresses = some_addresses();
        match container0.add_peer(id1.clone(), addresses.clone()) {
            true => (container0, id1),
            false => {
                container1.add_peer(id0.clone(), addresses);
                (container1, id0)
            }
        }
    }

    async fn container_with_added_nonconnecting_peer() -> (DirectedPeers<Address>, AuthorityId) {
        let (mut container0, id0) = container_with_id().await;
        let (mut container1, id1) = container_with_id().await;
        let addresses = some_addresses();
        match container0.add_peer(id1.clone(), addresses.clone()) {
            false => (container0, id1),
            true => {
                container1.add_peer(id0.clone(), addresses);
                (container1, id0)
            }
        }
    }

    #[tokio::test]
    async fn no_connecting_on_subsequent_add() {
        let (mut container0, id1) = container_with_added_connecting_peer().await;
        let addresses = some_addresses();
        assert!(!container0.add_peer(id1, addresses));
    }

    #[tokio::test]
    async fn peer_addresses_when_connecting() {
        let (container0, id1) = container_with_added_connecting_peer().await;
        assert!(container0.peer_addresses(&id1).is_some());
    }

    #[tokio::test]
    async fn no_peer_addresses_when_nonconnecting() {
        let (container0, id1) = container_with_added_nonconnecting_peer().await;
        assert!(container0.peer_addresses(&id1).is_none());
    }

    #[tokio::test]
    async fn interested_in_connecting() {
        let (container0, id1) = container_with_added_connecting_peer().await;
        assert!(container0.interested(&id1));
    }

    #[tokio::test]
    async fn interested_in_nonconnecting() {
        let (container0, id1) = container_with_added_nonconnecting_peer().await;
        assert!(container0.interested(&id1));
    }

    #[tokio::test]
    async fn uninterested_in_unknown() {
        let (container0, _) = container_with_id().await;
        let (_, id1) = container_with_id().await;
        assert!(!container0.interested(&id1));
    }

    #[tokio::test]
    async fn connecting_are_outgoing() {
        let (container0, id1) = container_with_added_connecting_peer().await;
        assert_eq!(container0.outgoing_peers().collect::<Vec<_>>(), vec![&id1]);
        assert_eq!(container0.incoming_peers().next(), None);
    }

    #[tokio::test]
    async fn nonconnecting_are_incoming() {
        let (container0, id1) = container_with_added_nonconnecting_peer().await;
        assert_eq!(container0.incoming_peers().collect::<Vec<_>>(), vec![&id1]);
        assert_eq!(container0.outgoing_peers().next(), None);
    }

    #[tokio::test]
    async fn uninterested_in_removed() {
        let (mut container0, id1) = container_with_added_connecting_peer().await;
        assert!(container0.interested(&id1));
        container0.remove_peer(&id1);
        assert!(!container0.interested(&id1));
    }
}
