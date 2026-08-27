//! Wall-clock daily save — replaces TFS `serversave.lua` `onTime`.
//!
//! Config keys (optional, TFS-shaped defaults):
//! - `serverSaveTime` `"04:30:00"`
//! - `serverSaveNotifyDuration` minutes (default 5)
//! - `serverSaveClose` (default true)
//! - `serverSaveShutdown` (default true)
//!
//! Game thread only. [`GameWorld::tick_server_save`] applies the poll result.
// C++ reference: `GlobalEvents::timer` + pack `serversave.lua`; `Game::setGameState`.

use chrono::{Local, NaiveTime, TimeZone, Timelike};

use crate::config::{ConfigManager, get_bool_or, get_i64_or, get_string_or};
use crate::game_state::GameState;
use crate::game_world::GameWorld;
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_net::outgoing_extra::send_text_message_simple;

/// `MESSAGE_STATUS_WARNING` (`const.h` / Lua `MESSAGE_STATUS_WARNING` = 0x12).
const MESSAGE_STATUS_WARNING: u8 = 0x12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerSaveConfig {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub notify_minutes: u32,
    pub close: bool,
    pub shutdown: bool,
}

impl ServerSaveConfig {
    pub fn from_config(cfg: &ConfigManager) -> Result<Self> {
        let raw = get_string_or(cfg, "serverSaveTime", "04:30:00")?;
        let (hour, minute, second) = parse_hms(&raw)?;
        let notify_minutes =
            get_i64_or(cfg, "serverSaveNotifyDuration", 5)?.clamp(0, 24 * 60) as u32;
        Ok(Self {
            hour,
            minute,
            second,
            notify_minutes,
            close: get_bool_or(cfg, "serverSaveClose", true)?,
            shutdown: get_bool_or(cfg, "serverSaveShutdown", true)?,
        })
    }
}

fn parse_hms(raw: &str) -> Result<(u32, u32, u32)> {
    let parts: Vec<&str> = raw.trim().split(':').collect();
    if parts.len() != 3 {
        return Err(TfsRustError::Config(format!(
            "serverSaveTime must be HH:MM:SS, got `{raw}`"
        )));
    }
    let hour: u32 = parts[0]
        .parse()
        .map_err(|_| TfsRustError::Config(format!("serverSaveTime hour in `{raw}`")))?;
    let minute: u32 = parts[1]
        .parse()
        .map_err(|_| TfsRustError::Config(format!("serverSaveTime minute in `{raw}`")))?;
    let second: u32 = parts[2]
        .parse()
        .map_err(|_| TfsRustError::Config(format!("serverSaveTime second in `{raw}`")))?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err(TfsRustError::Config(format!(
            "serverSaveTime out of range: `{raw}`"
        )));
    }
    Ok((hour, minute, second))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SavePhase {
    Idle,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerSavePoll {
    Idle,
    EnterWarning { close: bool, notify_minutes: u32 },
    Fire { shutdown: bool },
}

/// Result of [`GameWorld::tick_server_save`] for the game loop to apply (flush / exit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServerSaveTick {
    #[default]
    None,
    /// Persist online players, keep the process running (`serverSaveShutdown = false`).
    FlushStay,
    /// Persist online players, then exit (`serverSaveShutdown = true`).
    FlushShutdown,
}

/// Next-fire math + phase machine. Lives on [`GameWorld`].
#[derive(Debug, Clone)]
pub struct ServerSaveController {
    cfg: ServerSaveConfig,
    phase: SavePhase,
    next_save_unix: i64,
    pending: ServerSaveTick,
}

impl ServerSaveController {
    pub fn new(cfg: ServerSaveConfig, now_unix: i64) -> Self {
        Self {
            cfg,
            phase: SavePhase::Idle,
            next_save_unix: next_save_unix(cfg, now_unix),
            pending: ServerSaveTick::None,
        }
    }

    fn take_pending(&mut self) -> ServerSaveTick {
        std::mem::replace(&mut self.pending, ServerSaveTick::None)
    }

    pub(crate) fn request_flush_stay(&mut self) {
        self.pending = ServerSaveTick::FlushStay;
    }

    pub(crate) fn request_flush_shutdown(&mut self) {
        self.pending = ServerSaveTick::FlushShutdown;
    }

    pub fn from_world_config(cfg: &ConfigManager) -> Result<Self> {
        let save_cfg = ServerSaveConfig::from_config(cfg)?;
        Ok(Self::new(save_cfg, Local::now().timestamp()))
    }

    pub fn poll(&mut self, now_unix: i64) -> ServerSavePoll {
        let notify_secs = i64::from(self.cfg.notify_minutes) * 60;
        let warn_at = self.next_save_unix.saturating_sub(notify_secs);
        match self.phase {
            SavePhase::Idle => {
                if now_unix >= self.next_save_unix {
                    return self.take_fire(now_unix);
                }
                if now_unix >= warn_at {
                    self.phase = SavePhase::Warning;
                    return ServerSavePoll::EnterWarning {
                        close: self.cfg.close,
                        notify_minutes: self.cfg.notify_minutes.max(1),
                    };
                }
                ServerSavePoll::Idle
            }
            SavePhase::Warning => {
                if now_unix >= self.next_save_unix {
                    self.take_fire(now_unix)
                } else {
                    ServerSavePoll::Idle
                }
            }
        }
    }

    fn take_fire(&mut self, now_unix: i64) -> ServerSavePoll {
        self.phase = SavePhase::Idle;
        self.next_save_unix = next_save_unix(self.cfg, now_unix + 1);
        ServerSavePoll::Fire {
            shutdown: self.cfg.shutdown,
        }
    }
}

impl GameWorld {
    /// Apply the daily-save clock. Stores a pending flush/exit for [`Self::take_save_tick`].
    pub fn tick_server_save(&mut self, now_unix: i64) -> ServerSaveTick {
        let tick = match self.server_save.poll(now_unix) {
            ServerSavePoll::Idle => ServerSaveTick::None,
            ServerSavePoll::EnterWarning {
                close,
                notify_minutes,
            } => {
                let msg = format!(
                    "Server is saving game in {notify_minutes} minutes.\nPlease come back in 10 minutes."
                );
                broadcast_status(self, &msg);
                if close {
                    self.game_state = GameState::Closed;
                }
                ServerSaveTick::None
            }
            ServerSavePoll::Fire { shutdown } => {
                if shutdown {
                    self.game_state = GameState::Shutdown;
                    ServerSaveTick::FlushShutdown
                } else {
                    self.game_state = GameState::Normal;
                    ServerSaveTick::FlushStay
                }
            }
        };
        self.server_save.pending = tick;
        tick
    }

    pub fn take_save_tick(&mut self) -> ServerSaveTick {
        self.server_save.take_pending()
    }

    /// `/save` / `saveServer()` — TFS `luaSaveServer` queues `Game::saveGameState`
    /// without the daily-save close/shutdown side effects.
    pub fn lua_script_save_server(&mut self) {
        self.server_save.request_flush_stay();
    }
}

pub fn next_save_unix(cfg: ServerSaveConfig, now_unix: i64) -> i64 {
    let tz = Local::now().timezone();
    let now = tz
        .timestamp_opt(now_unix, 0)
        .single()
        .unwrap_or_else(|| Local::now());
    let t = NaiveTime::from_hms_opt(cfg.hour, cfg.minute, cfg.second)
        .unwrap_or_else(|| NaiveTime::from_hms_opt(4, 30, 0).expect("valid"));
    let today = now.date_naive();
    let candidate = today.and_time(t);
    let candidate_dt = candidate
        .and_local_timezone(tz)
        .earliest()
        .or_else(|| candidate.and_local_timezone(tz).latest())
        .unwrap_or(now);
    if candidate_dt.timestamp() > now_unix {
        candidate_dt.timestamp()
    } else {
        let tomorrow = today.succ_opt().unwrap_or(today);
        let next = tomorrow.and_time(t);
        next.and_local_timezone(tz)
            .earliest()
            .or_else(|| next.and_local_timezone(tz).latest())
            .map(|d| d.timestamp())
            .unwrap_or(now_unix + 86_400)
    }
}

fn broadcast_status(world: &mut GameWorld, text: &str) {
    let packet = send_text_message_simple(MESSAGE_STATUS_WARNING, text).into_bytes();
    let cids: Vec<_> = world.player_by_guid.values().copied().collect();
    for cid in cids {
        if let Some(conn) = world.creature_to_conn.get(&cid).copied() {
            world.enqueue_outgoing(conn, packet.clone());
        }
    }
}

#[allow(dead_code)]
pub fn seconds_since_midnight_local(now_unix: i64) -> u32 {
    let tz = Local::now().timezone();
    let dt = tz
        .timestamp_opt(now_unix, 0)
        .single()
        .unwrap_or_else(|| Local::now());
    dt.hour() * 3600 + dt.minute() * 60 + dt.second()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_at(hour: u32, minute: u32, notify: u32) -> ServerSaveConfig {
        ServerSaveConfig {
            hour,
            minute,
            second: 0,
            notify_minutes: notify,
            close: true,
            shutdown: true,
        }
    }

    #[test]
    fn parse_hms_ok() {
        assert_eq!(parse_hms("04:30:00").unwrap(), (4, 30, 0));
        assert_eq!(parse_hms("23:59:59").unwrap(), (23, 59, 59));
    }

    #[test]
    fn parse_hms_rejects_garbage() {
        assert!(parse_hms("4:30").is_err());
        assert!(parse_hms("25:00:00").is_err());
    }

    #[test]
    fn next_save_is_in_the_future() {
        let cfg = cfg_at(4, 30, 5);
        let now = 1_777_000_000;
        let next = next_save_unix(cfg, now);
        let delta = next - now;
        assert!(delta > 0, "next must be after now, got {delta}");
        assert!(delta <= 86_400, "next must be within a day, got {delta}");
    }

    #[test]
    fn poll_enters_warning_inside_notify_window() {
        let cfg = cfg_at(4, 30, 5);
        let save_at = 2_000_000_000;
        let mut ctl = ServerSaveController {
            cfg,
            phase: SavePhase::Idle,
            next_save_unix: save_at,
            pending: ServerSaveTick::None,
        };
        let poll = ctl.poll(save_at - 60);
        assert_eq!(
            poll,
            ServerSavePoll::EnterWarning {
                close: true,
                notify_minutes: 5
            }
        );
        assert_eq!(ctl.phase, SavePhase::Warning);
    }

    #[test]
    fn poll_fires_at_save_instant() {
        let cfg = cfg_at(4, 30, 5);
        let save_at = 2_000_000_000;
        let mut ctl = ServerSaveController {
            cfg,
            phase: SavePhase::Warning,
            next_save_unix: save_at,
            pending: ServerSaveTick::None,
        };
        let poll = ctl.poll(save_at);
        assert_eq!(poll, ServerSavePoll::Fire { shutdown: true });
        assert_eq!(ctl.phase, SavePhase::Idle);
        assert!(ctl.next_save_unix > save_at);
    }
}
