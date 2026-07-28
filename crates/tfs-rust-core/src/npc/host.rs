//! [`NpcActionHost`] implementation on [`GameWorld`] (NPC-5).

use slotmap::Key;
use tfs_rust_common::enums::{ConditionType, WorldType};
use tfs_rust_common::Position;

use super::actions::NpcActionHost;
use crate::combat::apply_condition;
use crate::condition::{ActiveCondition, ConditionData};
use crate::creature::vocation::VocationProfile;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

impl NpcActionHost for GameWorld {
    fn create_item(&mut self, player: CreatureId, item_id: i32, count: i32) -> Result<(), String> {
        if item_id <= 0 || count <= 0 {
            return Ok(());
        }
        let id = u16::try_from(item_id).map_err(|_| format!("invalid item id {item_id}"))?;
        self.npc_give_to(player, id, count as u32, -1)
    }

    fn delete_item(&mut self, player: CreatureId, item_id: i32, count: i32) -> Result<(), String> {
        if item_id <= 0 || count <= 0 {
            return Ok(());
        }
        let id = u16::try_from(item_id).map_err(|_| format!("invalid item id {item_id}"))?;
        self.npc_get_from(player, id, count as u32, -1)
    }

    fn create_money(&mut self, player: CreatureId, amount: i32) -> Result<(), String> {
        self.player_create_money(player, amount)
    }

    fn delete_money(&mut self, player: CreatureId, amount: i32) -> Result<(), String> {
        self.player_delete_money(player, amount)
    }

    fn set_hp(&mut self, player: CreatureId, value: i32) -> Result<(), String> {
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(player) else {
            return Err("set_hp: player not found".into());
        };
        let max = p.base.max_health.max(1);
        p.base.health = value.clamp(0, max);
        self.send_player_stats(player);
        Ok(())
    }

    fn set_poison(&mut self, player: CreatureId, cycles: i32, param: i32) -> Result<(), String> {
        set_dot_condition(self, player, ConditionType::Poison, cycles, param)
    }

    fn set_burning(&mut self, player: CreatureId, cycles: i32, param: i32) -> Result<(), String> {
        set_dot_condition(self, player, ConditionType::Fire, cycles, param)
    }

    fn effect_me(&mut self, npc: CreatureId, effect_id: u16) -> Result<(), String> {
        let pos = self
            .creatures
            .get(npc)
            .map(|k| k.position())
            .ok_or_else(|| "effect_me: npc not found".to_string())?;
        self.broadcast_magic_effect(pos, effect_id as u8);
        Ok(())
    }

    fn effect_opp(&mut self, player: CreatureId, effect_id: u16) -> Result<(), String> {
        let pos = self
            .creatures
            .get(player)
            .map(|k| k.position())
            .ok_or_else(|| "effect_opp: player not found".to_string())?;
        self.broadcast_magic_effect(pos, effect_id as u8);
        Ok(())
    }

    fn set_quest_value(&mut self, player: CreatureId, id: u32, value: i32) -> Result<(), String> {
        self.player_set_storage(player, id, value)
    }

    fn set_profession(&mut self, player: CreatureId, vocation: i32) -> Result<(), String> {
        let profile = self
            .vocations
            .get(vocation)
            .map(VocationProfile::from_def)
            .unwrap_or_else(|| VocationProfile {
                id: vocation,
                ..VocationProfile::default()
            });
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(player) else {
            return Err("set_profession: player not found".into());
        };
        p.vocation_id = vocation;
        p.vocation_profile = profile;
        if let Some(ref mut persist) = p.persist {
            persist.player_row.vocation = vocation;
        }
        self.send_player_stats(player);
        Ok(())
    }

    fn teach_spell(&mut self, player: CreatureId, spell: i32) -> Result<(), String> {
        let name = spell_learn_key(self, spell);
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(player) else {
            return Err("teach_spell: player not found".into());
        };
        let persist = p
            .persist
            .as_mut()
            .ok_or_else(|| "teach_spell: player has no persist baseline".to_string())?;
        if !persist.spells.iter().any(|s| s == &name) {
            persist.spells.push(name);
        }
        Ok(())
    }

    fn summon(&mut self, npc: CreatureId, monster: &str) -> Result<(), String> {
        let pos = self
            .creatures
            .get(npc)
            .map(|k| k.position())
            .ok_or_else(|| "summon: npc not found".to_string())?;
        let created = self.lua_script_create_monster(monster, pos.x, pos.y, pos.z, false, true)?;
        if created.is_none() {
            return Err(format!("summon: unknown monster {monster:?}"));
        }
        Ok(())
    }

    fn teleport(&mut self, player: CreatureId, x: i32, y: i32, z: i32) -> Result<(), String> {
        let dest = Position {
            x: u16::try_from(x).map_err(|_| format!("teleport: bad x {x}"))?,
            y: u16::try_from(y).map_err(|_| format!("teleport: bad y {y}"))?,
            z: u8::try_from(z).map_err(|_| format!("teleport: bad z {z}"))?,
        };
        let ffi = player.data().as_ffi();
        let ok = self.lua_script_creature_teleport(ffi, dest.x, dest.y, dest.z, false)?;
        if !ok {
            return Err("teleport failed".into());
        }
        Ok(())
    }

    fn set_start_position(
        &mut self,
        player: CreatureId,
        npc: CreatureId,
        pos: Option<(i32, i32, i32)>,
    ) -> Result<(i32, i32, i32), String> {
        let (x, y, z) = match pos {
            Some(p) => p,
            None => {
                let home = match self.creatures.get(npc) {
                    Some(CreatureKind::Npc(n)) => n.runtime.home_position,
                    _ => {
                        return Err("set_start_position: npc not found".into());
                    }
                };
                (i32::from(home.x), i32::from(home.y), i32::from(home.z))
            }
        };
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(player) else {
            return Err("set_start_position: player not found".into());
        };
        let persist = p
            .persist
            .as_mut()
            .ok_or_else(|| "set_start_position: player has no persist baseline".to_string())?;
        persist.player_row.posx = x;
        persist.player_row.posy = y;
        persist.player_row.posz = z;
        Ok((x, y, z))
    }

    fn invoke_custom_action(
        &mut self,
        npc: CreatureId,
        player: CreatureId,
        callback_id: tfs_rust_content::npcs::NpcCallbackId,
    ) -> Result<(), String> {
        if crate::lua_scope::fire_npc_custom_action(self, npc, player, callback_id) {
            Ok(())
        } else {
            Err("custom action failed or returned false".into())
        }
    }
}

impl GameWorld {
    /// 772 `TNPC::GiveTo` — `crnonpl.cc:1870-1895`.
    ///
    /// Stackable items are added in stacks of ≤100; non-stackable items spawn as `amount`
    /// separate objects, each created with `sub_type = data`. A hard cap prevents malformed
    /// behaviour files from allocating unbounded items.
    pub fn npc_give_to(
        &mut self,
        player: CreatureId,
        item_id: u16,
        amount: u32,
        data: i32,
    ) -> Result<(), String> {
        if amount == 0 {
            return Ok(());
        }
        const MAX_ITEMS: u32 = 500;
        let amount = if amount > MAX_ITEMS {
            tracing::warn!(
                player = ?player,
                item_id,
                requested = amount,
                "npc_give_to: amount exceeds cap of {MAX_ITEMS}"
            );
            MAX_ITEMS
        } else {
            amount
        };
        let stackable = self
            .items_db
            .items
            .get(&item_id)
            .map(|t| t.stackable())
            .unwrap_or(false);
        if stackable {
            let mut remaining = amount;
            while remaining > 0 {
                let chunk = remaining.min(100);
                self.player_add_item_count(player, item_id, chunk)?;
                remaining -= chunk;
            }
        } else {
            let ffi = player.data().as_ffi();
            for _ in 0..amount {
                self.lua_script_player_add_item_full(ffi, item_id, 1, data, true, 0)?
                    .ok_or_else(|| format!("failed to add item {item_id}"))?;
            }
        }
        Ok(())
    }

    /// 772 `TNPC::GetFrom` / `DeleteAtCreature` — `crnonpl.cc:1896-1908`, `operate.cc:2728-2751`.
    ///
    /// Stackable items remove a total of `amount` (partial-stack support). Non-stackable items
    /// remove `amount` whole objects. A hard cap prevents malformed behaviour files from deleting
    /// unbounded items. `data` is reserved for Phase 2 fluid/key subtype threading.
    pub fn npc_get_from(
        &mut self,
        player: CreatureId,
        item_id: u16,
        amount: u32,
        data: i32,
    ) -> Result<(), String> {
        if amount == 0 {
            return Ok(());
        }
        const MAX_ITEMS: u32 = 500;
        let amount = if amount > MAX_ITEMS {
            tracing::warn!(
                player = ?player,
                item_id,
                requested = amount,
                "npc_get_from: amount exceeds cap of {MAX_ITEMS}"
            );
            MAX_ITEMS
        } else {
            amount
        };
        let _stackable = self
            .items_db
            .items
            .get(&item_id)
            .map(|t| t.stackable())
            .unwrap_or(false);
        // `player_remove_item_of_type` already branches on CUMULATIVE internally, mirroring
        // `DeleteAtCreature`; the split is explicit here for parity with `npc_give_to` and to
        // prepare for Phase 2 `data` plumbing.
        if !self.player_remove_item_of_type(player, item_id, amount, data, false) {
            return Err(format!("failed to delete item {item_id} x{amount}"));
        }
        Ok(())
    }

    /// Read quest/storage value (missing → `-1`, matching 772 `GetQuestValue`).
    pub fn player_get_storage(&self, cid: CreatureId, storage_id: u32) -> i32 {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return -1;
        };
        p.persist
            .as_ref()
            .and_then(|b| {
                b.storage
                    .iter()
                    .find(|(k, _)| *k == storage_id)
                    .map(|(_, v)| *v)
            })
            .unwrap_or(-1)
    }

    /// Upsert quest/storage and mark persist dirty for save export.
    pub fn player_set_storage(
        &mut self,
        cid: CreatureId,
        storage_id: u32,
        value: i32,
    ) -> Result<(), String> {
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
            return Err("set_storage: player not found".into());
        };
        let persist = p
            .persist
            .as_mut()
            .ok_or_else(|| "set_storage: player has no persist baseline".to_string())?;
        if let Some(slot) = persist.storage.iter_mut().find(|(k, _)| *k == storage_id) {
            slot.1 = value;
        } else {
            persist.storage.push((storage_id, value));
        }
        Ok(())
    }

    /// Condition timer rounds for poison/fire (0 when absent).
    pub fn player_condition_cycles(&self, cid: CreatureId, ctype: ConditionType) -> i32 {
        let Some(kind) = self.creatures.get(cid) else {
            return 0;
        };
        kind.base()
            .active_conditions
            .iter()
            .find(|c| c.ctype == ctype)
            .map(|c| c.timer_rounds_left.unwrap_or(1).max(0))
            .unwrap_or(0)
    }

    pub(crate) fn npc_world_pvp_flags(&self) -> (bool, bool) {
        match self.pvp_config.world_type {
            WorldType::PvpEnforced => (true, false),
            WorldType::NoPvp => (false, true),
            _ => (false, false),
        }
    }
}

fn set_dot_condition(
    world: &mut GameWorld,
    player: CreatureId,
    ctype: ConditionType,
    cycles: i32,
    param: i32,
) -> Result<(), String> {
    if cycles <= 0 {
        let removed = if let Some(kind) = world.creatures.get_mut(player) {
            let base = kind.base_mut();
            let before = base.active_conditions.len();
            base.active_conditions.retain(|c| c.ctype != ctype);
            before != base.active_conditions.len()
        } else {
            false
        };
        if removed {
            world.on_condition_ended(player, ctype);
        }
        return Ok(());
    }
    let cond = ActiveCondition::new(
        0,
        0,
        ctype,
        ConditionData::Damage {
            total_rank: param.abs().max(1),
        },
        Some(cycles),
    );
    apply_condition(&mut world.creatures, player, cond);
    world.on_condition_started(player, ctype);
    Ok(())
}

fn spell_learn_key(world: &GameWorld, spell: i32) -> String {
    // Prefer a registered instant name when the numeric id matches nothing better;
    // 772 stores SpellNr — persist as decimal string for round-trip.
    let key = spell.to_string();
    for def in world.spells.instant_by_name.values() {
        if def.name.eq_ignore_ascii_case(&key) {
            return def.name.clone();
        }
    }
    key
}

impl GameWorld {
    /// NPC-7: Lua `npc:say` — enqueue Talk ToDo.
    pub fn npc_lua_say_u64(&mut self, npc_u64: u64, text: &str) -> Result<(), String> {
        let npc = self
            .resolve_creature_u64(npc_u64)
            .ok_or_else(|| "npc not found".to_string())?;
        self.enqueue_creature_talk(npc, text.to_string());
        Ok(())
    }

    /// NPC-7: Lua `npc:setFocus`.
    pub fn npc_lua_set_focus_u64(
        &mut self,
        npc_u64: u64,
        player_u64: Option<u64>,
    ) -> Result<(), String> {
        let npc = self
            .resolve_creature_u64(npc_u64)
            .ok_or_else(|| "npc not found".to_string())?;
        let player = match player_u64 {
            None => None,
            Some(id) => Some(
                self.resolve_creature_u64(id)
                    .ok_or_else(|| "player not found".to_string())?,
            ),
        };
        if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc) {
            n.runtime.focus = player;
            Ok(())
        } else {
            Err("not an npc".into())
        }
    }

    /// NPC-7: deposit inventory gold into bank balance.
    pub fn player_bank_deposit_u64(&mut self, player_u64: u64, amount: u64) -> Result<(), String> {
        if amount == 0 {
            return Ok(());
        }
        let cid = self
            .resolve_creature_u64(player_u64)
            .ok_or_else(|| "player not found".to_string())?;
        let amount_i = i32::try_from(amount).map_err(|_| "deposit amount too large".to_string())?;
        self.player_delete_money(cid, amount_i)?;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.economy.balance = p.economy.balance.saturating_add(amount);
            Ok(())
        } else {
            Err("not a player".into())
        }
    }

    /// NPC-7: withdraw bank balance into inventory gold.
    pub fn player_bank_withdraw_u64(&mut self, player_u64: u64, amount: u64) -> Result<(), String> {
        if amount == 0 {
            return Ok(());
        }
        let cid = self
            .resolve_creature_u64(player_u64)
            .ok_or_else(|| "player not found".to_string())?;
        let amount_i =
            i32::try_from(amount).map_err(|_| "withdraw amount too large".to_string())?;
        {
            let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
                return Err("not a player".into());
            };
            if p.economy.balance < amount {
                return Err("insufficient bank balance".into());
            }
            p.economy.balance -= amount;
        }
        self.player_create_money(cid, amount_i)
    }
}
