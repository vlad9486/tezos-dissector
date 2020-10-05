// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use wireshark_definitions::{TreePresenter, NetworkPacket};
use wireshark_epan_adapter::{Dissector, Tree};
use crypto::proof_of_work::ProofOfWork;
use tezos_conversation::{
    Conversation, Packet, ChunkInfo, ConsumeResult, Sender, ChunkInfoProvider, Identity,
    ChunkMetadata,
};
use std::{collections::BTreeMap, ops::Range};

pub struct TezosDissector {
    identity: Option<Identity>,
    // Each pair of endpoints has its own context.
    // The pair is unordered,
    // so A talk to B is the same conversation as B talks to A.
    // The key is just pointer in memory, so it is invalid when capturing session is closed.
    conversations: BTreeMap<usize, ConversationData>,
}

struct ConversationData {
    valid: bool,
    inner: Conversation,
    storage: Storage,
}

impl ConversationData {
    pub fn new() -> Self {
        ConversationData {
            valid: true,
            inner: Conversation::new(ProofOfWork::DEFAULT_TARGET),
            storage: Storage {
                from_initiator: Vec::new(),
                from_responder: Vec::new(),
                packet_ranges: BTreeMap::new(),
            }
        }
    }
}

struct Storage {
    from_initiator: Vec<ChunkInfo>,
    from_responder: Vec<ChunkInfo>,
    packet_ranges: BTreeMap<u64, Range<usize>>,
}

impl ChunkInfoProvider for Storage {
    fn packet_range(&self, packet_index: u64) -> Option<&Range<usize>> {
        self.packet_ranges.get(&packet_index)
    }

    fn chunks_after(&self, sender: &Sender, offset: usize) -> Option<(usize, &[ChunkInfo])> {
        let chunks: &[ChunkInfo] = match sender {
            &Sender::Initiator => self.from_initiator.as_ref(),
            &Sender::Responder => self.from_responder.as_ref(),
        };
        chunks
            .iter()
            .enumerate()
            .find(|&(_, info)| info.body().end > offset)
            .map(|(i, info)| (i, info.continuation()))
            .map(|(first_chunk, continuation)| {
                if continuation {
                    chunks[0..first_chunk]
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|&(_, info)| !info.continuation())
                        .unwrap()
                        .0
                } else {
                    if first_chunk == 0 {
                        first_chunk
                    } else {
                        let v = chunks[0..first_chunk]
                            .iter()
                            .enumerate()
                            .rev()
                            .take_while(|&(_, info)| info.incomplete())
                            .count();
                        first_chunk - v
                    }
                }
            })
            .map(|f| (f, &chunks[f..]))
    }
}

impl TezosDissector {
    pub fn new() -> Self {
        TezosDissector {
            identity: None,
            conversations: BTreeMap::new(),
        }
    }
}

impl Dissector for TezosDissector {
    // This method called by the wireshark when the user choose the identity file.
    fn prefs_update(&mut self, filenames: Vec<&str>) {
        if let Some(identity_path) = filenames.first().cloned() {
            if !identity_path.is_empty() {
                // read the identity from the file
                self.identity = Identity::from_path(identity_path.to_string())
                    .map_err(|e| {
                        log::error!("Identity: {}", e);
                        e
                    })
                    .ok();
            }
        }
    }

    // This method called by the wireshark when a new packet just arrive,
    // or when the user click on the packet.
    fn consume(&mut self, root: &mut Tree, packet: NetworkPacket, c_id: usize) -> usize {
        self.consume_polymorphic::<Tree>(root, packet, c_id)
    }

    // This method called by the wireshark when the user
    // closing current capturing session
    fn cleanup(&mut self) {
        self.conversations.clear();
    }
}

impl TezosDissector {
    /// needed for tests, to use moc instead of `Tree`
    fn consume_polymorphic<T>(
        &mut self,
        root: &mut Tree,
        packet: NetworkPacket,
        c_id: usize,
    ) -> usize
    where
        T: TreePresenter,
    {
        let packet = Packet::from(packet);

        // get the data
        // retrieve or create a new context for the conversation
        let data = self
            .conversations
            .entry(c_id)
            .or_insert_with(ConversationData::new);

        let id = self.identity.as_ref();
        if !data.storage.packet_ranges.contains_key(&packet.number) && data.valid {
            let (result, sender, packet_range) = data.inner.add(id, &packet);
            if let &ConsumeResult::InvalidConversation = &result {
                data.valid = false;
            } else {
                let mut chunks = match result {
                    ConsumeResult::ConnectionMessage(chunk) => vec![chunk],
                    ConsumeResult::Chunks { regular, .. } => {
                        regular.into_iter().map(|p| p.decrypted).collect()
                    },
                    _ => Vec::new(),
                };
                match sender {
                    Sender::Initiator => data.storage.from_initiator.append(&mut chunks),
                    Sender::Responder => data.storage.from_responder.append(&mut chunks),
                }
                data.storage.packet_ranges.insert(packet.number, packet_range);
            }
        }
        if data.inner.visualize(&packet, &data.storage, root) {
            packet.payload.len()
        } else {
            0
        }
    }
}
