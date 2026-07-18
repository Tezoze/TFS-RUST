//! Player stats packets and group flag / capacity helpers.
//!
//! - `Player::sendStats` — `player.cpp`.
//! - `Player::sendSkills` — `player.cpp` / `ProtocolGame::AddPlayerSkills`.
//! - Group access flags — `groups.cpp`.

use tfs_rust_net::codec::{PlayerSkillsWire, PlayerStatsWire};

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::player::combat::SkillNr;

impl GameWorld {
    /// C++ `Player::sendStats` (`player.cpp` ~882) — builds a full `0xA0` stats packet and enqueues
    /// it for the player's connection. Must be called after any health/mana/soul/experience/capacity
    /// change (mirrors every `sendStats()` call site in TFS C++).
    pub fn send_player_stats(&mut self, cid: CreatureId) {
        let Some(conn_id) = self.conn_for_creature(cid) else {
            return;
        };
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        let flags = crate::player_flags::flags_for_group(&self.groups, p.group_id);
        let cannot = crate::player_flags::has_player_flag(
            flags,
            crate::player_flags::PLAYER_FLAG_CANNOT_PICKUP_ITEM,
        );
        let infinite = crate::player_flags::has_player_flag(
            flags,
            crate::player_flags::PLAYER_FLAG_HAS_INFINITE_CAPACITY,
        );
        let hl = p.base.health.max(0).min(u16::MAX as i32) as u16;
        let max_h = p.base.max_health.max(0).min(u16::MAX as i32) as u16;
        let level = p.level.max(0).min(u16::MAX as i32) as u16;
        let total_cap = p.get_capacity_u32_with_flags(cannot, infinite);
        let free_cap = p.get_free_capacity_u32_with_flags(cannot, infinite);

        // C++ `Player::getPercentLevel` (`player.cpp` ~1914).
        let level_percent = {
            let curr = crate::creature::vocation::total_experience_for_level(level as u32);
            let next = crate::creature::vocation::total_experience_for_level(level as u32 + 1);
            if next > curr && p.experience >= curr {
                crate::creature::Player::percent_level(p.experience - curr, next - curr)
            } else {
                0u8
            }
        };

        let magic_level_percent = p.magic_percent(&self.mechanics.profile, &self.mechanics.hooks);

        let stats = PlayerStatsWire {
            health: hl,
            max_health: max_h,
            free_capacity: free_cap,
            total_capacity: total_cap,
            experience: p.experience,
            level,
            level_percent,
            mana: p.mana.max(0).min(u16::MAX as i32) as u16,
            max_mana: p.max_mana.max(0).min(u16::MAX as i32) as u16,
            magic_level: p.skills.maglevel.clamp(0, 255) as u8,
            base_magic_level: p.skills.maglevel.clamp(0, 255) as u8,
            magic_level_percent,
            soul: p.economy.soul.clamp(0, 255) as u8,
            stamina_minutes: p.stamina_minutes,
            base_speed_half: (p.base.base_speed.max(0) as u32 / 2).min(0xFFFF) as u16,
            regeneration_ticks_sec: 0,
            offline_training_time: (p.offline_training_ms / 60 / 1000).min(65535) as u16,
        };

        self.enqueue_encoded(conn_id, self.codec.encode_player_stats(&stats));
    }

    /// C++ `Player::sendSkills` — `0xA1` with live levels + percent bars.
    ///
    /// Call after any `skill_increase` / `skill_decrease` (and at login).
    pub fn send_player_skills(&mut self, cid: CreatureId) {
        let Some(conn_id) = self.conn_for_creature(cid) else {
            return;
        };
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        let profile = &self.mechanics.profile;
        let hooks = &self.mechanics.hooks;
        let mut levels = [0u16; 7];
        let mut percents = [0u8; 7];
        for skill in SkillNr::COMBAT_ALL {
            let idx = skill.try_index();
            levels[idx] = skill.level(&p.skills).clamp(0, u16::MAX as i32) as u16;
            percents[idx] = p.skill_percent(skill, profile, hooks);
        }
        let wire = PlayerSkillsWire {
            levels,
            bases: levels,
            percents,
            additional_levels: [0u16; 6],
            additional_bases: [0u16; 6],
        };
        self.enqueue_encoded(conn_id, self.codec.encode_player_skills(&wire));
    }

    /// 772/1098 `MESSAGE_EVENT_ADVANCE` (0x13 / 19) — white console + game-window text.
    ///
    /// C++ `Player::sendTextMessage(MESSAGE_EVENT_ADVANCE, …)` — skill/magic/level advances.
    pub fn send_player_advance_message(&mut self, cid: CreatureId, text: &str) {
        let Some(conn_id) = self.conn_for_creature(cid) else {
            return;
        };
        use tfs_rust_net::outgoing_extra::send_text_message_simple;
        const MESSAGE_EVENT_ADVANCE: u8 = 0x13; // `const.h` / `gameserver/src/const.h`
        self.enqueue_outgoing(
            conn_id,
            send_text_message_simple(MESSAGE_EVENT_ADVANCE, text).into_bytes(),
        );
    }

    /// 772 `Creature::onGainExperience` — floating exp number (`TEXTCOLOR_WHITE_EXP` = 215).
    ///
    /// C++ `g_game.addAnimatedText(pos, TEXTCOLOR_WHITE_EXP, std::to_string(gainExp))`
    /// (`creature.cpp:771`). 1098 codec returns empty (no `sendAnimatedText`).
    pub fn broadcast_experience_popup(&mut self, pos: tfs_rust_common::Position, amount: u64) {
        if amount == 0 {
            return;
        }
        use tfs_rust_net::codec::wire::AnimatedTextWire;
        const TEXTCOLOR_WHITE_EXP: u8 = 215; // `gameserver/src/const.h`
        let animated = self.codec.encode_animated_text(&AnimatedTextWire {
            pos,
            color: TEXTCOLOR_WHITE_EXP,
            text: amount.to_string(),
        });
        if !animated.as_bytes().is_empty() {
            self.broadcast_to_spectators(pos, animated.into_bytes());
        }
    }

    /// Notify client after combat-skill tries were added (skills packet + optional advance lines).
    pub fn notify_skill_tries_gained(
        &mut self,
        cid: CreatureId,
        skill: SkillNr,
        levels_gained: u32,
    ) {
        self.send_player_skills(cid);
        for _ in 0..levels_gained {
            // 772 `addSkillAdvance` — `player.cpp:442`: "You advanced in {skill}."
            self.send_player_advance_message(
                cid,
                &format!("You advanced in {}.", skill.display_name()),
            );
        }
    }

    /// Notify client after mana-spent / magic tries (stats packet + optional advance lines).
    pub fn notify_magic_tries_gained(
        &mut self,
        cid: CreatureId,
        levels_gained: u32,
        new_maglevel: i32,
    ) {
        self.send_player_stats(cid);
        if levels_gained == 0 {
            return;
        }
        let first = new_maglevel - levels_gained as i32 + 1;
        for ml in first..=new_maglevel {
            // 772 `addManaSpent` — `player.cpp:1425`.
            self.send_player_advance_message(cid, &format!("You advanced to magic level {ml}."));
        }
    }

    pub fn player_is_access_player(&self, cid: CreatureId) -> bool {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        self.groups
            .groups
            .get(&p.group_id)
            .is_some_and(|g| g.access)
    }

    /// Resolved `PlayerFlag` bits for `players.group_id`.
    pub fn player_group_flags(&self, cid: CreatureId) -> u64 {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return 0;
        };
        crate::player_flags::flags_for_group(&self.groups, p.group_id)
    }

    pub fn player_has_flag(&self, cid: CreatureId, flag: u64) -> bool {
        crate::player_flags::has_player_flag(self.player_group_flags(cid), flag)
    }

    pub fn player_capacity_u32(&self, cid: CreatureId) -> Option<u32> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return None;
        };
        let cannot = self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_CANNOT_PICKUP_ITEM);
        let infinite =
            self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_HAS_INFINITE_CAPACITY);
        Some(p.get_capacity_u32_with_flags(cannot, infinite))
    }

    pub fn player_free_capacity_u32(&self, cid: CreatureId) -> Option<u32> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return None;
        };
        let cannot = self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_CANNOT_PICKUP_ITEM);
        let infinite =
            self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_HAS_INFINITE_CAPACITY);
        Some(p.get_free_capacity_u32_with_flags(cannot, infinite))
    }

    /// Ensure all worn containers are registered before inventory scans.
    pub(crate) fn hydrate_player_equipment_containers(&mut self, cid: CreatureId) {
        let roots: Vec<ItemId> = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.equipment_slots.iter().flatten().copied().collect(),
            _ => return,
        };
        let mut registry = std::mem::take(&mut self.container_registry);
        for root in roots {
            if self
                .items
                .get(root)
                .is_some_and(|i| self.items_db.is_container(i.item_type))
            {
                self.ensure_container_registered_simple(&mut registry, root, cid);
            }
        }
        self.container_registry = registry;
    }
}
