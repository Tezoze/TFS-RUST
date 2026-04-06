//! `PendingLogin` handling: queue/drop policy and oneshot discard semantics.
// Required for Phase 5 login flow when DB work spans ticks (`tasks.md` 1.6b).

use std::collections::VecDeque;

use tokio::sync::oneshot;

use tfs_rust_common::{ConnId, GameCommand, GamePacket};

/// Result delivered on the oneshot when async login/DB work finishes (placeholder until Phase 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginPendingResult {
    /// Character is ready to enter the game world.
    Ready,
    /// Login failed (wrong password, ban, etc.).
    Failed { reason: String },
}

/// What to do with an incoming game packet while in [`crate::protocol::ConnectionState::PendingLogin`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingLoginPacketAction {
    /// Packet is ignored (movement/combat during login).
    Dropped,
    /// Chat text was stored for delivery after login completes.
    QueuedChat,
}

/// While awaiting `LoginPendingResult` from the game/DB side across await points.
pub struct PendingLogin {
    pub conn_id: ConnId,
    pub char_name: String,
    chat_queue: VecDeque<String>,
    login_result_rx: oneshot::Receiver<LoginPendingResult>,
}

impl std::fmt::Debug for PendingLogin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingLogin")
            .field("conn_id", &self.conn_id)
            .field("char_name", &self.char_name)
            .field("chat_queue_len", &self.chat_queue.len())
            .finish_non_exhaustive()
    }
}

impl PendingLogin {
    pub fn new(
        conn_id: ConnId,
        char_name: String,
        login_result_rx: oneshot::Receiver<LoginPendingResult>,
    ) -> Self {
        Self {
            conn_id,
            char_name,
            chat_queue: VecDeque::new(),
            login_result_rx,
        }
    }

    /// Apply queue/drop rules from task 1.6b: movement/attack dropped; chat queued.
    pub fn dispatch_command(&mut self, cmd: GameCommand) -> PendingLoginPacketAction {
        match cmd {
            GameCommand::Game { conn_id, packet } if conn_id == self.conn_id => {
                dispatch_game_packet(&mut self.chat_queue, packet)
            }
            GameCommand::Game { .. } => PendingLoginPacketAction::Dropped,
            GameCommand::Shutdown
            | GameCommand::LuaCallback { .. }
            | GameCommand::LuaAsyncResult { .. }
            | GameCommand::PlayerLogin { .. }
            | GameCommand::PlayerDisconnect { .. } => PendingLoginPacketAction::Dropped,
        }
    }

    /// Drain queued chat (e.g. after transition to `Game`).
    pub fn drain_chat_queue(&mut self) -> VecDeque<String> {
        std::mem::take(&mut self.chat_queue)
    }

    /// Split for handing the receiver to an async task or awaiting in place.
    pub fn into_parts(
        self,
    ) -> (
        ConnId,
        String,
        VecDeque<String>,
        oneshot::Receiver<LoginPendingResult>,
    ) {
        (
            self.conn_id,
            self.char_name,
            self.chat_queue,
            self.login_result_rx,
        )
    }
}

fn dispatch_game_packet(
    chat_queue: &mut VecDeque<String>,
    packet: GamePacket,
) -> PendingLoginPacketAction {
    match packet {
        GamePacket::Say(p) => {
            chat_queue.push_back(p.text);
            PendingLoginPacketAction::QueuedChat
        }
        GamePacket::Move(_)
        | GamePacket::AutoWalk { .. }
        | GamePacket::StopAutoWalk
        | GamePacket::Turn(_)
        | GamePacket::Attack { .. }
        | GamePacket::Follow { .. }
        | GamePacket::CancelAttackAndFollow
        | GamePacket::EquipObject { .. } => PendingLoginPacketAction::Dropped,
        _ => PendingLoginPacketAction::Dropped,
    }
}

/// Drop pending state; if the connection closes first, the game thread’s `send` will fail — discard there.
pub fn disconnect_pending_login(pending: PendingLogin) {
    drop(pending);
}

/// Game thread: send login result; returns `false` if the connection already dropped the receiver (discard work).
pub fn send_login_result_or_discard(
    tx: oneshot::Sender<LoginPendingResult>,
    result: LoginPendingResult,
) -> bool {
    tx.send(result).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_common::enums::Direction;
    use tfs_rust_common::game_packet::SayPayload;

    #[test]
    fn drops_move_and_attack_queues_say() {
        let (_tx, rx) = oneshot::channel::<LoginPendingResult>();
        let mut p = PendingLogin::new(ConnId(1), "Test".to_string(), rx);
        assert_eq!(
            p.dispatch_command(GameCommand::Game {
                conn_id: ConnId(1),
                packet: GamePacket::Move(Direction::North),
            }),
            PendingLoginPacketAction::Dropped
        );
        assert_eq!(
            p.dispatch_command(GameCommand::Game {
                conn_id: ConnId(1),
                packet: GamePacket::Attack { creature_id: 0 },
            }),
            PendingLoginPacketAction::Dropped
        );
        assert_eq!(
            p.dispatch_command(GameCommand::Game {
                conn_id: ConnId(1),
                packet: GamePacket::Say(SayPayload {
                    speak_class: 0,
                    channel_id: 0,
                    receiver: String::new(),
                    text: "hi".to_string(),
                }),
            }),
            PendingLoginPacketAction::QueuedChat
        );
        assert_eq!(p.drain_chat_queue().len(), 1);
        drop(_tx);
    }

    #[test]
    fn mismatched_conn_id_dropped() {
        let (_tx, rx) = oneshot::channel::<LoginPendingResult>();
        let mut p = PendingLogin::new(ConnId(1), "Test".to_string(), rx);
        assert_eq!(
            p.dispatch_command(GameCommand::Game {
                conn_id: ConnId(99),
                packet: GamePacket::Say(SayPayload {
                    speak_class: 0,
                    channel_id: 0,
                    receiver: String::new(),
                    text: "nope".into(),
                }),
            }),
            PendingLoginPacketAction::Dropped
        );
        assert!(p.drain_chat_queue().is_empty());
    }

    #[test]
    fn producer_discards_when_receiver_dropped() {
        let (p, tx) = {
            let (tx, rx) = oneshot::channel();
            (PendingLogin::new(ConnId(2), "X".to_string(), rx), tx)
        };
        disconnect_pending_login(p);
        assert!(!send_login_result_or_discard(tx, LoginPendingResult::Ready));
    }

    #[tokio::test]
    async fn producer_delivers_when_receiver_alive() {
        let (tx, rx) = oneshot::channel();
        let p = PendingLogin::new(ConnId(3), "Y".to_string(), rx);
        let (_id, _name, _q, rx) = p.into_parts();
        assert!(send_login_result_or_discard(
            tx,
            LoginPendingResult::Failed {
                reason: "nope".into()
            }
        ));
        let got = rx.await.expect("oneshot");
        assert_eq!(
            got,
            LoginPendingResult::Failed {
                reason: "nope".into()
            }
        );
    }
}
