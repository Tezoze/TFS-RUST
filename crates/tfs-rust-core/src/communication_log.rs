//! Speech statements and GM listeners, pruned every Other round.
//!
//! C++ reference: `operate.cc` `LogCommunication`, `LogListener`,
//! `ProcessCommunicationControl` (statements older than 1800 s).

use std::collections::VecDeque;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::player::flags::PLAYER_FLAG_LOG_COMMUNICATION;

/// Corpus `Statement->TimeStamp + 1800` (`operate.cc:3199`).
const STATEMENT_TTL_SECS: i64 = 1800;

struct Statement {
    id: u32,
    unix_time: i64,
    // Stored for a later report lookup (`GetCommunicationContext` is out of this pass).
    #[allow(dead_code)]
    character_id: u32,
    #[allow(dead_code)]
    mode: i32,
    #[allow(dead_code)]
    channel: i32,
    #[allow(dead_code)]
    text: String,
}

struct Listener {
    statement_id: u32,
    #[allow(dead_code)]
    character_id: u32,
}

/// Append-ordered statement and listener lists. Game thread only.
#[derive(Default)]
pub struct CommunicationLog {
    statements: VecDeque<Statement>,
    listeners: VecDeque<Listener>,
}

impl CommunicationLog {
    fn push_statement(
        &mut self,
        id: u32,
        unix_time: i64,
        character_id: u32,
        mode: i32,
        channel: i32,
        text: &str,
    ) {
        self.statements.push_back(Statement {
            id,
            unix_time,
            character_id,
            mode,
            channel,
            text: text.to_string(),
        });
    }

    fn push_listener(&mut self, statement_id: u32, character_id: u32) {
        self.listeners.push_back(Listener {
            statement_id,
            character_id,
        });
    }

    /// Drop statements older than 1800 s, then listeners below the oldest survivor.
    ///
    /// An empty statement list leaves `Limit == 0`, which drops no listeners
    /// (`operate.cc:3210-3214`).
    fn prune(&mut self, now_unix: i64) {
        while let Some(front) = self.statements.front() {
            if now_unix <= front.unix_time.saturating_add(STATEMENT_TTL_SECS) {
                break;
            }
            self.statements.pop_front();
        }
        let limit = self.statements.front().map(|s| s.id).unwrap_or(0);
        if limit == 0 {
            return;
        }
        while let Some(front) = self.listeners.front() {
            if front.statement_id >= limit {
                break;
            }
            self.listeners.pop_front();
        }
    }
}

impl GameWorld {
    /// `LogCommunication` — one statement id for the speech event (`operate.cc:3155`).
    ///
    /// Non-players still get a wire id and are not stored (`CharacterID` is a player id).
    pub(crate) fn log_player_speech(
        &mut self,
        speaker: CreatureId,
        mode: i32,
        channel: i32,
        text: &str,
    ) -> u32 {
        let id = self.alloc_statement_id();
        let Some(guid) = self.creatures.get(speaker).and_then(|k| match k {
            CreatureKind::Player(p) if p.guid != 0 => Some(p.guid),
            _ => None,
        }) else {
            return id;
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.communication_log
            .push_statement(id, now, guid, mode, channel, text);
        id
    }

    /// `LogListener` — only a player with the log-communication right is stored.
    /// The pack has no such group flag today, so this is a no-op until one is set.
    pub(crate) fn log_listener(&mut self, statement_id: u32, listener: CreatureId) {
        if statement_id == 0 || !self.player_has_flag(listener, PLAYER_FLAG_LOG_COMMUNICATION) {
            return;
        }
        let Some(guid) = self.creatures.get(listener).and_then(|k| match k {
            CreatureKind::Player(p) if p.guid != 0 => Some(p.guid),
            _ => None,
        }) else {
            return;
        };
        self.communication_log.push_listener(statement_id, guid);
    }

    /// `ProcessCommunicationControl` — every Other round (`main.cc:356`).
    pub(crate) fn process_communication_control(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.communication_log.prune(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn statement(id: u32, unix_time: i64) -> Statement {
        Statement {
            id,
            unix_time,
            character_id: 1,
            mode: 1,
            channel: 0,
            text: "hi".into(),
        }
    }

    #[test]
    fn prune_drops_statements_older_than_1800s_and_older_listeners() {
        let mut log = CommunicationLog::default();
        log.statements.push_back(statement(1, 1_000));
        log.statements.push_back(statement(2, 2_000));
        log.listeners.push_back(Listener {
            statement_id: 1,
            character_id: 9,
        });
        log.listeners.push_back(Listener {
            statement_id: 2,
            character_id: 9,
        });
        // now=2800 → 1000+1800 is not < now? now <= ts+1800 → 2800 <= 2800 keeps id 1.
        // now=2801 drops id 1 (2801 > 2800) and keeps id 2 (2801 <= 3800).
        log.prune(2_801);
        assert_eq!(log.statements.len(), 1);
        assert_eq!(log.statements[0].id, 2);
        assert_eq!(log.listeners.len(), 1);
        assert_eq!(log.listeners[0].statement_id, 2);
    }

    #[test]
    fn empty_statement_list_does_not_drop_listeners() {
        let mut log = CommunicationLog::default();
        log.listeners.push_back(Listener {
            statement_id: 1,
            character_id: 9,
        });
        log.prune(9_999);
        assert_eq!(log.listeners.len(), 1);
    }
}
