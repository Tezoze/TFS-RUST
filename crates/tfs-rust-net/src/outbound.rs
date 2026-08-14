//! Per-connection outbound batch channel — bounded queue + byte accounting (GL-3).
//!
//! C++ reference: `src/connection.cpp` send path / output buffer.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::mpsc;

/// Max queued batches per connection before `try_send` returns [`OutboundSendError::Full`].
pub const OUTPUT_BATCH_CHANNEL_CAP: usize = 256;
/// Soft backpressure threshold once the writer is already behind (2 MiB).
///
/// A single floor-change / login `0x64` map description with dense creatures can exceed
/// a few hundred KiB in one flush. Soft-capping an *empty* queue rejects that burst and
/// the game loop sheds the client → OTClient desync. Soft cap applies only when
/// `queued_bytes > 0` (client already lagging).
pub const OUTPUT_QUEUED_BYTE_CAP: usize = 2 * 1024 * 1024;
/// Hard shed threshold — disconnect when queued bytes exceed this (8 MiB).
pub const OUTPUT_SLOW_CLIENT_DISCONNECT_BYTES: usize = 8 * 1024 * 1024;

pub type OutputBatch = Vec<Vec<u8>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboundSendError {
    Full,
    Closed,
    SlowClient { queued: usize, batch: usize },
}

/// Game-thread clone — cheap `Sender` handle with shared byte counter.
#[derive(Clone)]
pub struct OutboundTx {
    inner: mpsc::Sender<OutputBatch>,
    queued_bytes: Arc<AtomicUsize>,
    byte_cap: usize,
    slow_disconnect_bytes: usize,
}

pub struct OutboundRx {
    inner: mpsc::Receiver<OutputBatch>,
    queued_bytes: Arc<AtomicUsize>,
}

impl OutboundTx {
    pub fn pair() -> (Self, OutboundRx) {
        Self::pair_with_caps(
            OUTPUT_BATCH_CHANNEL_CAP,
            OUTPUT_QUEUED_BYTE_CAP,
            OUTPUT_SLOW_CLIENT_DISCONNECT_BYTES,
        )
    }

    pub fn pair_with_caps(
        batch_cap: usize,
        byte_cap: usize,
        slow_disconnect_bytes: usize,
    ) -> (Self, OutboundRx) {
        let (tx, rx) = mpsc::channel(batch_cap.max(1));
        let queued_bytes = Arc::new(AtomicUsize::new(0));
        (
            Self {
                inner: tx,
                queued_bytes: Arc::clone(&queued_bytes),
                byte_cap,
                slow_disconnect_bytes,
            },
            OutboundRx {
                inner: rx,
                queued_bytes,
            },
        )
    }

    pub fn queued_bytes(&self) -> usize {
        self.queued_bytes.load(Ordering::Relaxed)
    }

    /// Non-blocking send from the game thread. Updates shared byte accounting on success.
    ///
    /// On error the batch is returned so the caller can re-queue (never drop a floor-change
    /// `0x64`). Empty-queue bursts are always admitted up to the hard shed limit; soft byte
    /// cap only rejects when the writer already has queued data.
    pub fn try_send(&self, batch: OutputBatch) -> Result<(), (OutboundSendError, OutputBatch)> {
        let batch_bytes: usize = batch.iter().map(|b| b.len()).sum();
        let cur = self.queued_bytes.load(Ordering::Relaxed);
        let next = cur.saturating_add(batch_bytes);
        if next > self.slow_disconnect_bytes {
            return Err((
                OutboundSendError::SlowClient {
                    queued: cur,
                    batch: batch_bytes,
                },
                batch,
            ));
        }
        // Soft cap: only when already behind — never block the first batch of a flush.
        if cur > 0 && next > self.byte_cap {
            return Err((OutboundSendError::Full, batch));
        }
        match self.inner.try_send(batch) {
            Ok(()) => {
                self.queued_bytes.fetch_add(batch_bytes, Ordering::Relaxed);
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(batch)) => Err((OutboundSendError::Full, batch)),
            Err(mpsc::error::TrySendError::Closed(batch)) => {
                Err((OutboundSendError::Closed, batch))
            }
        }
    }
}

impl OutboundRx {
    pub async fn recv(&mut self) -> Option<OutputBatch> {
        let batch = self.inner.recv().await?;
        let batch_bytes: usize = batch.iter().map(|b| b.len()).sum();
        self.queued_bytes.fetch_sub(batch_bytes, Ordering::Relaxed);
        Some(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn byte_accounting_tracks_queued_payload() {
        let (tx, mut rx) = OutboundTx::pair_with_caps(4, 100, 200);
        tx.try_send(vec![vec![0u8; 40]]).unwrap();
        assert_eq!(tx.queued_bytes(), 40);
        let batch = rx.recv().await.unwrap();
        assert_eq!(batch[0].len(), 40);
        assert_eq!(tx.queued_bytes(), 0);
    }

    #[test]
    fn slow_client_threshold_rejects_before_channel_full() {
        let (tx, _rx) = OutboundTx::pair_with_caps(64, 10_000, 100);
        assert!(matches!(
            tx.try_send(vec![vec![0u8; 101]]),
            Err((OutboundSendError::SlowClient { .. }, _))
        ));
    }

    #[test]
    fn empty_queue_admits_burst_over_soft_cap() {
        // Soft cap 50, hard 10_000 — a 200-byte batch on an empty queue must succeed
        // (floor-change / login map description shape).
        let (tx, _rx) = OutboundTx::pair_with_caps(4, 50, 10_000);
        assert!(
            tx.try_send(vec![vec![0u8; 200]]).is_ok(),
            "empty-queue burst must not soft-cap reject"
        );
    }

    #[test]
    fn soft_cap_rejects_when_already_queued() {
        let (tx, _rx) = OutboundTx::pair_with_caps(4, 100, 10_000);
        tx.try_send(vec![vec![0u8; 80]]).unwrap();
        assert!(matches!(
            tx.try_send(vec![vec![0u8; 40]]),
            Err((OutboundSendError::Full, batch)) if batch[0].len() == 40
        ));
    }
}
