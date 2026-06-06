//! Player stats packets and group flag / capacity helpers.
//!
//! - `Player::sendStats` — `player.cpp`.
//! - Group access flags — `groups.cpp`.

use tfs_rust_net::codec::PlayerStatsWire;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};

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
        let cannot =
            crate::player_flags::has_player_flag(flags, crate::player_flags::PLAYER_FLAG_CANNOT_PICKUP_ITEM);
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
                (((p.experience - curr) * 100) / (next - curr)).min(100) as u8
            } else {
                0u8
            }
        };

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
            magic_level_percent: 0,
            soul: p.economy.soul.clamp(0, 255) as u8,
            stamina_minutes: p.stamina_minutes,
            base_speed_half: (p.base.base_speed.max(0) as u32 / 2).min(0xFFFF) as u16,
            regeneration_ticks_sec: 0,
            offline_training_time: (p.offline_training_ms / 60 / 1000).min(65535) as u16,
        };

        self.enqueue_encoded(conn_id, self.codec.encode_player_stats(&stats));
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
        let infinite = self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_HAS_INFINITE_CAPACITY);
        Some(p.get_capacity_u32_with_flags(cannot, infinite))
    }

    pub fn player_free_capacity_u32(&self, cid: CreatureId) -> Option<u32> {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return None;
        };
        let cannot = self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_CANNOT_PICKUP_ITEM);
        let infinite = self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_HAS_INFINITE_CAPACITY);
        Some(p.get_free_capacity_u32_with_flags(cannot, infinite))
    }

    /// Ensure all worn containers are registered before inventory scans.
    pub(crate) fn hydrate_player_equipment_containers(&mut self, cid: CreatureId) {
        let roots: Vec<ItemId> = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => {
                p.equipment_slots.iter().flatten().copied().collect()
            }
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
