//! Login waiting list when `maxPlayers` is set.
//! C++ reference: `800src/waitlist.cpp` `WaitingList::clientLogin` /
//! `WaitingList::getTime`; `800src/protocolgame.cpp` opcode `0x16`.

use tfs_rust_db::LoadedPlayerData;
use tfs_rust_net::NetworkMessage;

use crate::config::get_i64_or;
use crate::game_world::GameWorld;
use crate::player::flags::{
    PLAYER_FLAG_CAN_ALWAYS_LOGIN, PLAYER_FLAG_IS_ALWAYS_PREMIUM, flags_for_group, has_player_flag,
};

#[derive(Debug, Clone, Copy)]
struct WaitEntry {
    guid: u32,
    timeout_ms: u64,
}

/// Premium queue, then the normal queue. Game-thread only.
#[derive(Debug, Default)]
pub struct WaitingList {
    priority: Vec<WaitEntry>,
    normal: Vec<WaitEntry>,
}

/// Outcome of one login attempt against the queues.
#[derive(Debug, PartialEq, Eq)]
pub enum Admit {
    Allow,
    Deny { slot: u32, retry_seconds: u8 },
}

pub(crate) struct AdmitInput {
    pub guid: u32,
    pub premium: bool,
    pub bypass: bool,
    pub max_players: u32,
    pub online: u32,
    pub now_ms: u64,
}

impl WaitingList {
    /// `WaitingList::getTime` — seconds the client should wait before retrying.
    pub fn retry_seconds(slot: u32) -> u8 {
        if slot < 5 {
            5
        } else if slot < 10 {
            10
        } else if slot < 20 {
            20
        } else if slot < 50 {
            60
        } else {
            120
        }
    }

    pub(crate) fn admit(&mut self, input: AdmitInput) -> Admit {
        if input.bypass || input.max_players == 0 {
            return Admit::Allow;
        }
        self.cleanup(input.now_ms);
        if self.priority.is_empty() && self.normal.is_empty() && input.online < input.max_players {
            return Admit::Allow;
        }
        if let Some((priority, index, slot)) = self.find(input.guid) {
            if input.online.saturating_add(slot) <= input.max_players {
                if priority {
                    self.priority.remove(index);
                } else {
                    self.normal.remove(index);
                }
                return Admit::Allow;
            }
            let timeout = input.now_ms.saturating_add(
                u64::from(u32::from(Self::retry_seconds(slot)).saturating_add(15)) * 1000,
            );
            let entry = if priority {
                &mut self.priority[index]
            } else {
                &mut self.normal[index]
            };
            entry.timeout_ms = timeout;
            return Admit::Deny {
                slot,
                retry_seconds: Self::retry_seconds(slot),
            };
        }

        let before = self.priority.len() as u32;
        let timeout_slot = if input.premium {
            before + 1
        } else {
            before + self.normal.len() as u32 + 1
        };
        let timeout = input.now_ms.saturating_add(
            u64::from(u32::from(Self::retry_seconds(timeout_slot)).saturating_add(15)) * 1000,
        );
        let entry = WaitEntry {
            guid: input.guid,
            timeout_ms: timeout,
        };
        if input.premium {
            self.priority.push(entry);
        } else {
            self.normal.push(entry);
        }
        let slot = self
            .find(input.guid)
            .map(|(_, _, slot)| slot)
            .unwrap_or(timeout_slot);
        Admit::Deny {
            slot,
            retry_seconds: Self::retry_seconds(slot),
        }
    }

    fn cleanup(&mut self, now_ms: u64) {
        self.priority.retain(|e| e.timeout_ms > now_ms);
        self.normal.retain(|e| e.timeout_ms > now_ms);
    }

    /// 1-based place, priority list first (`WaitingList::findClient`).
    fn find(&self, guid: u32) -> Option<(bool, usize, u32)> {
        let mut slot = 1u32;
        for (index, entry) in self.priority.iter().enumerate() {
            if entry.guid == guid {
                return Some((true, index, slot));
            }
            slot += 1;
        }
        for (index, entry) in self.normal.iter().enumerate() {
            if entry.guid == guid {
                return Some((false, index, slot));
            }
            slot += 1;
        }
        None
    }
}

/// `0x16` + message + retry byte. Sent only for client versions that read it.
pub fn encode_waiting_list(slot: u32, retry_seconds: u8) -> Vec<u8> {
    let text = format!("Too many players online.\nYou are at place {slot} on the waiting list.");
    let mut msg = NetworkMessage::new();
    msg.write_u8(0x16);
    msg.write_string(&text);
    msg.write_u8(retry_seconds);
    msg.into_bytes()
}

/// New logins only. `None` means the player may enter.
pub(crate) fn deny_login(
    world: &mut GameWorld,
    loaded: &LoadedPlayerData,
) -> Option<Option<Vec<u8>>> {
    let guid = u32::try_from(loaded.player.id).unwrap_or(0);
    let group_id = u16::try_from(loaded.player.group_id).unwrap_or(0);
    let flags = flags_for_group(&world.groups, group_id);
    let bypass = loaded.account_type >= 4 || has_player_flag(flags, PLAYER_FLAG_CAN_ALWAYS_LOGIN);
    let max_players = u32::try_from(
        get_i64_or(&world.config, "maxPlayers", 0)
            .unwrap_or(0)
            .max(0),
    )
    .unwrap_or(u32::MAX);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let premium = loaded_is_premium(world, loaded, flags, now_ms / 1000);
    let decision = world.waiting_list.admit(AdmitInput {
        guid,
        premium,
        bypass,
        max_players,
        online: world.player_by_guid.len() as u32,
        now_ms,
    });
    match decision {
        Admit::Allow => None,
        Admit::Deny {
            slot,
            retry_seconds,
        } => {
            let version = world.codec.version().raw();
            let notice = if (790..1098).contains(&version) {
                Some(encode_waiting_list(slot, retry_seconds))
            } else {
                None
            };
            Some(notice)
        }
    }
}

fn loaded_is_premium(
    world: &GameWorld,
    loaded: &LoadedPlayerData,
    flags: u64,
    now_secs: u64,
) -> bool {
    if world.config.get_bool("freePremium").unwrap_or(false) {
        return true;
    }
    if has_player_flag(flags, PLAYER_FLAG_IS_ALWAYS_PREMIUM) {
        return true;
    }
    u64::from(loaded.premium_ends_at) > now_secs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_seconds_match_waitlist_bands() {
        assert_eq!(WaitingList::retry_seconds(1), 5);
        assert_eq!(WaitingList::retry_seconds(4), 5);
        assert_eq!(WaitingList::retry_seconds(5), 10);
        assert_eq!(WaitingList::retry_seconds(9), 10);
        assert_eq!(WaitingList::retry_seconds(10), 20);
        assert_eq!(WaitingList::retry_seconds(20), 60);
        assert_eq!(WaitingList::retry_seconds(50), 120);
    }

    #[test]
    fn full_server_queues_then_admits_when_a_slot_opens() {
        let mut list = WaitingList::default();
        let denied = list.admit(AdmitInput {
            guid: 7,
            premium: false,
            bypass: false,
            max_players: 1,
            online: 1,
            now_ms: 1_000,
        });
        assert_eq!(
            denied,
            Admit::Deny {
                slot: 1,
                retry_seconds: 5
            }
        );
        let still = list.admit(AdmitInput {
            guid: 7,
            premium: false,
            bypass: false,
            max_players: 1,
            online: 1,
            now_ms: 2_000,
        });
        assert_eq!(
            still,
            Admit::Deny {
                slot: 1,
                retry_seconds: 5
            }
        );
        let allowed = list.admit(AdmitInput {
            guid: 7,
            premium: false,
            bypass: false,
            max_players: 1,
            online: 0,
            now_ms: 3_000,
        });
        assert_eq!(allowed, Admit::Allow);
        assert!(list.priority.is_empty() && list.normal.is_empty());
    }

    #[test]
    fn unlimited_and_bypass_skip_the_queue() {
        let mut list = WaitingList::default();
        assert_eq!(
            list.admit(AdmitInput {
                guid: 1,
                premium: false,
                bypass: false,
                max_players: 0,
                online: 50,
                now_ms: 0,
            }),
            Admit::Allow
        );
        assert_eq!(
            list.admit(AdmitInput {
                guid: 2,
                premium: false,
                bypass: true,
                max_players: 1,
                online: 9,
                now_ms: 0,
            }),
            Admit::Allow
        );
        assert!(list.normal.is_empty());
    }

    #[test]
    fn waiting_packet_starts_with_opcode() {
        let bytes = encode_waiting_list(3, 5);
        assert_eq!(bytes[0], 0x16);
        assert_eq!(*bytes.last().unwrap(), 5);
    }
}
