// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use std::{ops::Range, collections::BTreeMap};
use bytes::Buf;
use failure::Fail;
use super::{addresses::Sender, chunk_info::ChunkInfo};
use crate::identity::{Decipher, NonceAddition};

pub struct DirectBuffer {
    data: Vec<u8>,
    chunks: Vec<ChunkInfo>,
    packets: BTreeMap<u64, Range<usize>>,
    processed: usize,
}

#[derive(Debug, Fail, Eq, PartialEq)]
#[fail(
    display = "MAC mismatch, sender: {:?}, number of chunk: {}",
    sender, chunk_number
)]
pub struct DecryptError {
    pub sender: Sender,
    pub chunk_number: usize,
}

impl DirectBuffer {
    pub fn new() -> Self {
        DirectBuffer {
            data: Vec::with_capacity(0x100000),
            chunks: Vec::with_capacity(0x1000),
            packets: BTreeMap::new(),
            // first message always decrypted
            processed: 1,
        }
    }

    pub fn consume(&mut self, payload: &[u8], frame_index: u64) {
        let start = self.data.len();
        self.data.extend_from_slice(payload);
        let end = self.data.len();
        self.packets.insert(frame_index, start..end);
        let mut position = self.chunks.last().map(|r| r.range().end).unwrap_or(0);

        loop {
            if position + 2 < end {
                let length = (&self.data[position..(position + 2)]).get_u16() as usize;
                let this_end = position + 2 + length;
                self.chunks.push(ChunkInfo::new(position, this_end));
                position = this_end;
            } else {
                break;
            }
        }
    }

    pub fn decrypt(&mut self, decipher: &Decipher, sender: Sender) -> Result<(), DecryptError> {
        if self.chunks().len() > self.processed {
            let chunks = (&self.chunks()[self.processed..]).to_vec();
            for chunk in chunks {
                let chunk = chunk.range();
                if self.data().len() >= chunk.end {
                    let nonce = match &sender {
                        &Sender::Initiator => NonceAddition::Initiator((self.processed - 1) as u64),
                        &Sender::Responder => NonceAddition::Responder((self.processed - 1) as u64),
                    };
                    let data = &self.data()[(chunk.start + 2)..chunk.end];
                    if let Ok(plain) = decipher.decrypt(data, nonce) {
                        self.processed += 1;
                        self.data_mut()[(chunk.start + 2)..(chunk.end - 16)]
                            .clone_from_slice(plain.as_ref());
                    } else {
                        return Err(DecryptError {
                            sender,
                            chunk_number: self.processed,
                        });
                    }
                } else {
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn decrypted(&self) -> usize {
        self.processed
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_ref()
    }

    fn data_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }

    pub fn chunks(&self) -> &[ChunkInfo] {
        self.chunks.as_ref()
    }

    pub fn packet(&self, index: u64) -> Range<usize> {
        self.packets
            .get(&index)
            .expect("expected already seen packet")
            .clone()
    }
}
