//! Talkaction Phase 5–6 mutation appliers (game state, master pos, sex, party XP).
//!
//! Pack: TFS `luascript.cpp` `luaGameSetGameState` / `luaNpcSetMasterPos` /
//! `luaPlayerSetSex` / `luaPartySetSharedExperience`.

use tfs_rust_common::Position;
use tfs_rust_common::enums::PlayerSex;

use crate::creature::CreatureKind;
use crate::game_state::GameState;
use crate::game_world::GameWorld;

impl GameWorld {
    /// `npc:setMasterPos(pos[, radius])` — TFS `luaNpcSetMasterPos`.
    pub fn lua_script_npc_set_master_pos(
        &mut self,
        npc_u64: u64,
        x: u16,
        y: u16,
        z: u8,
        radius: Option<u16>,
    ) -> bool {
        let Some(cid) = self.resolve_creature_u64(npc_u64) else {
            return false;
        };
        let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(cid) else {
            return false;
        };
        n.runtime.home_position = Position { x, y, z };
        if let Some(r) = radius {
            n.runtime.radius = r;
        }
        true
    }

    /// `Game.setGameState` — TFS `luaGameSetGameState`. Always true in Lua.
    pub fn lua_script_set_game_state(&mut self, lua_state: i32) {
        let next = match lua_state {
            2 => GameState::Normal,
            3 => GameState::Closed,
            4 => GameState::Shutdown,
            _ => {
                tracing::info!(lua_state, "Game.setGameState: unmapped state ignored");
                return;
            }
        };
        if self.game_state == GameState::Shutdown {
            return;
        }
        if self.game_state == next {
            return;
        }
        self.game_state = next;
        if next == GameState::Shutdown {
            self.server_save.request_flush_shutdown();
        }
    }

    /// `player:setSex(sex)` — TFS `luaPlayerSetSex` (no outfit rewrite).
    pub fn lua_script_player_set_sex(&mut self, creature_u64: u64, sex: u8) -> bool {
        let Some(cid) = self.resolve_creature_u64(creature_u64) else {
            return false;
        };
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
            return false;
        };
        p.sex = if sex == 0 {
            PlayerSex::Female
        } else {
            PlayerSex::Male
        };
        true
    }

    /// `party:setSharedExperience(bool)` — TFS `luaPartySetSharedExperience` (flag only).
    pub fn lua_script_party_set_shared_experience(&mut self, party_id: u32, enabled: bool) -> bool {
        let Some(party) = self.parties.get_mut(&party_id) else {
            return false;
        };
        party.shared_experience_enabled = enabled;
        true
    }
}
