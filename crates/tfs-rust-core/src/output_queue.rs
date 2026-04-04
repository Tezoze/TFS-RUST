//! Per-connection outbound queues with backpressure (protocol packets).
// C++ reference: `OutputMessage`, `Connection::send` batching.

use std::collections::VecDeque;

/// Serialized protocol payload (builder lives in `tfs-rust-net` Phase 7).
pub type OutputMessage = Vec<u8>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketPriority {
    Critical,
    Visual,
}

#[derive(Debug)]
pub struct QueuedPacket {
    pub priority: PacketPriority,
    pub payload: OutputMessage,
}

#[derive(Debug)]
pub struct ConnectionSendQueue {
    pub queue: VecDeque<QueuedPacket>,
    pub max_packets: usize,
    pub max_bytes: usize,
    pub bytes_pending: usize,
    pub pending_chunks: VecDeque<OutputMessage>,
    pub full_streak: u8,
}

impl ConnectionSendQueue {
    pub fn new(max_packets: usize, max_bytes: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_packets,
            max_bytes,
            bytes_pending: 0,
            pending_chunks: VecDeque::new(),
            full_streak: 0,
        }
    }

    pub fn push(&mut self, priority: PacketPriority, payload: OutputMessage) -> bool {
        let n = payload.len();
        while self.bytes_pending + n > self.max_bytes && !self.queue.is_empty() {
            self.drop_visuals();
            if self.bytes_pending + n > self.max_bytes {
                break;
            }
        }
        if self.queue.len() >= self.max_packets {
            self.drop_visuals();
        }
        if self.queue.len() >= self.max_packets {
            self.full_streak = self.full_streak.saturating_add(1);
            return false;
        }
        self.bytes_pending += n;
        self.queue.push_back(QueuedPacket { priority, payload });
        self.full_streak = 0;
        true
    }

    fn drop_visuals(&mut self) {
        let mut dropped = 0usize;
        self.queue.retain(|q| {
            if q.priority == PacketPriority::Visual {
                dropped += q.payload.len();
                false
            } else {
                true
            }
        });
        self.bytes_pending = self.bytes_pending.saturating_sub(dropped);
    }

    pub fn pop_batch(&mut self, max_bytes: usize) -> Vec<OutputMessage> {
        let mut out = Vec::new();
        let mut used = 0usize;
        while let Some(front) = self.queue.front() {
            if used + front.payload.len() > max_bytes {
                break;
            }
            let q = self.queue.pop_front().unwrap();
            used += q.payload.len();
            self.bytes_pending = self.bytes_pending.saturating_sub(q.payload.len());
            out.push(q.payload);
        }
        out
    }

    pub fn should_disconnect(&self) -> bool {
        self.full_streak >= 3
    }
}
