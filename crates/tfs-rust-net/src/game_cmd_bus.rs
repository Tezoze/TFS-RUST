//! Dual-lane game command ingress (GL-2).
//!
//! Bounded lane carries per-connection `Game` packets with backpressure.
//! Unbounded control lane carries shutdown, disconnect, login completion, timers —
//! never dropped by channel fullness.
//!
//! Overload policy (documented production safety, not mechanics):
//! - channel capacity [`GAME_COMMAND_CHANNEL_CAP`]
//! - at most [`MAX_GAME_COMMANDS_PER_TURN`] game-lane commands before yielding to a ready beat
//! - when the game lane is full, the net reader **sheds the connection** (does not drop packets silently)

use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

use tfs_rust_common::GameCommand;

/// Shared game-lane capacity (packets + non-control traffic).
pub const GAME_COMMAND_CHANNEL_CAP: usize = 8192;

/// Max game-lane commands processed before the loop services a ready beat.
pub const MAX_GAME_COMMANDS_PER_TURN: usize = 64;

/// Cloneable sender pair used by net readers, scheduler, and login-load tasks.
#[derive(Clone, Debug)]
pub struct GameCmdTx {
    game: mpsc::Sender<GameCommand>,
    ctrl: mpsc::UnboundedSender<GameCommand>,
}

#[derive(Debug)]
pub enum GameCmdSendError {
    Closed,
    /// Game lane full — caller should shed the connection (stop reading / disconnect).
    GameLaneFull,
}

impl GameCmdTx {
    pub fn new(game: mpsc::Sender<GameCommand>, ctrl: mpsc::UnboundedSender<GameCommand>) -> Self {
        Self { game, ctrl }
    }

    /// Route `Game` packets to the bounded lane; everything else to control.
    pub fn send(&self, cmd: GameCommand) -> Result<(), GameCmdSendError> {
        match cmd {
            GameCommand::Game { .. } => match self.game.try_send(cmd) {
                Ok(()) => Ok(()),
                Err(TrySendError::Full(_)) => Err(GameCmdSendError::GameLaneFull),
                Err(TrySendError::Closed(_)) => Err(GameCmdSendError::Closed),
            },
            other => self.ctrl.send(other).map_err(|_| GameCmdSendError::Closed),
        }
    }

    /// Control-lane only (login load completion, disconnect, shutdown).
    pub fn send_ctrl(&self, cmd: GameCommand) -> Result<(), GameCmdSendError> {
        self.ctrl.send(cmd).map_err(|_| GameCmdSendError::Closed)
    }

    pub fn ctrl_sender(&self) -> mpsc::UnboundedSender<GameCommand> {
        self.ctrl.clone()
    }
}

/// Create the dual-lane channels used by the server / game loop.
pub fn open_game_command_channels() -> (
    GameCmdTx,
    mpsc::Receiver<GameCommand>,
    mpsc::UnboundedReceiver<GameCommand>,
) {
    let (game_tx, game_rx) = mpsc::channel(GAME_COMMAND_CHANNEL_CAP);
    let (ctrl_tx, ctrl_rx) = mpsc::unbounded_channel();
    (GameCmdTx::new(game_tx, ctrl_tx), game_rx, ctrl_rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_common::{ConnId, GamePacket, enums::Direction};

    #[tokio::test]
    async fn game_lane_reports_full_without_blocking() {
        let (game_tx, mut game_rx) = mpsc::channel(1);
        let (ctrl_tx, _ctrl_rx) = mpsc::unbounded_channel();
        let tx = GameCmdTx::new(game_tx, ctrl_tx);
        let pkt = || GameCommand::Game {
            conn_id: ConnId(1),
            packet: GamePacket::Move(Direction::North),
        };
        assert!(tx.send(pkt()).is_ok());
        assert!(matches!(
            tx.send(pkt()),
            Err(GameCmdSendError::GameLaneFull)
        ));
        let _ = game_rx.recv().await;
    }

    #[tokio::test]
    async fn control_lane_accepts_while_game_full() {
        let (game_tx, _game_rx) = mpsc::channel(1);
        let (ctrl_tx, mut ctrl_rx) = mpsc::unbounded_channel();
        let tx = GameCmdTx::new(game_tx, ctrl_tx);
        assert!(
            tx.send(GameCommand::Game {
                conn_id: ConnId(1),
                packet: GamePacket::Move(Direction::North),
            })
            .is_ok()
        );
        let _ = tx.send(GameCommand::Game {
            conn_id: ConnId(1),
            packet: GamePacket::Move(Direction::South),
        });
        assert!(tx.send(GameCommand::Shutdown).is_ok());
        assert!(matches!(ctrl_rx.try_recv(), Ok(GameCommand::Shutdown)));
    }

    /// Audit #2: game-lane full and outbound SlowClient both report without blocking.
    #[test]
    fn dual_fill_game_lane_full_and_outbound_slow_client() {
        use crate::outbound::{OutboundSendError, OutboundTx};

        let (game_tx, _game_rx) = mpsc::channel(1);
        let (ctrl_tx, _ctrl_rx) = mpsc::unbounded_channel();
        let cmd_tx = GameCmdTx::new(game_tx, ctrl_tx);
        let pkt = || GameCommand::Game {
            conn_id: ConnId(1),
            packet: GamePacket::Move(Direction::North),
        };
        assert!(cmd_tx.send(pkt()).is_ok());
        assert!(matches!(
            cmd_tx.send(pkt()),
            Err(GameCmdSendError::GameLaneFull)
        ));

        let (out_tx, _out_rx) = OutboundTx::pair_with_caps(4, 50, 100);
        assert!(matches!(
            out_tx.try_send(vec![vec![0u8; 200]]),
            Err((OutboundSendError::SlowClient { .. }, _))
        ));
    }
}
