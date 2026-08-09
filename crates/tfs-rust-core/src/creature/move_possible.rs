//! Per-creature `MovePossible(Execute=true)` push predicates — 772 Gate B.
//!
//! Called from `check_push_destination` (`operate.cc:515-516`) on the **moving** creature
//! (not the actor) when pushing another creature. Each predicate mirrors the decompile but
//! is written as idiomatic Rust (no vtable, no OOP).
//!
//! - Domain: TFS `MovePossible` entry point (so `data/` contracts hold).
//! - 772 outcomes: `crplayer.cc:363` `TPlayer::MovePossible`, `crnonpl.cc:2141`
//!   `TMonster::MovePossible`, `crnonpl.cc:1672` `TNPC::MovePossible`, `crmain.cc:883`
//!   `TCreature::MovePossible`.
//!
//! **C2/C3:** signatures are `Result<bool, ReturnValue>` — `TPlayer::MovePossible` throws
//! `ENTERPROTECTIONZONE`/`NOTINVITED` and the monster kick loop throws `EXHAUSTED`; these
//! propagate out of `CheckMapDestination` unchanged (not remapped to `NOROOM`). `Ok(false)`
//! carries the plain `false` returns (which become `NOROOM` for pushing another creature).

use std::time::Instant;

use tfs_rust_common::Position;

use crate::creature::{CreatureKind, MonsterState};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::monster_push::MonsterKickOutcome;
use crate::player_flags::PLAYER_FLAG_CAN_EDIT_HOUSES;
use crate::return_value::ReturnValue;
use crate::tile::Tile;

impl GameWorld {
    /// 772 `TCreature::MovePossible(x, y, z, Execute, Jump)` — base predicate
    /// (`crmain.cc:883-898`).
    ///
    /// `Jump` → `JumpPossible(x, y, z, false)`; else `CoordinateFlag(BANK) && !CoordinateFlag(UNPASS)`.
    /// `!Execute && AVOID` → false — but here `Execute=true`, so AVOID is handled by Gate C
    /// (the `tile_has_avoid` check in `check_push_destination`).
    pub(crate) fn base_move_possible(&self, dest: Position, jump: bool) -> bool {
        if jump {
            // 772 `JumpPossible(x, y, z, false)` (`info.cc:702`).
            self.jump_possible(dest, false)
        } else {
            // 772 `CoordinateFlag(BANK) && !CoordinateFlag(UNPASS)`.
            self.tile_is_bank_and_passable(dest)
        }
    }

    /// 772 `TPlayer::MovePossible(x, y, z, Execute=true, Jump)` — `crplayer.cc:363-380`.
    ///
    /// Base (`JumpPossible`/`BANK&&!UNPASS`), PZ-enter gate (`EarliestProtectionZoneRound`),
    /// and house-invite gate. **C2:** the PZ-enter gate throws `ENTERPROTECTIONZONE` and the
    /// house-invite gate throws `NOTINVITED` — these are `Err(...)`, not `Ok(false)`.
    ///
    /// `origin` is the player's current position (for the PZ-enter `!IsProtectionZone(origin)`
    /// check). `jump` is `OrigZ != DestZ` from the `operate.cc:516` call.
    pub(crate) fn player_move_possible_push(
        &self,
        cid: CreatureId,
        dest: Position,
        origin: Position,
        jump: bool,
    ) -> Result<bool, ReturnValue> {
        // Base `TCreature::MovePossible` (`crplayer.cc:364`).
        if !self.base_move_possible(dest, jump) {
            return Ok(false);
        }

        // `crplayer.cc:365-378` — `if(Result && Execute)`.
        let (earliest_pz_round, round_nr, guid, group_flags) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => (
                p.earliest_protection_zone_round,
                self.round_nr,
                p.guid,
                self.player_group_flags(cid),
            ),
            _ => return Ok(false),
        };

        // PZ-enter gate (`crplayer.cc:366-369`):
        //   `EarliestProtectionZoneRound > RoundNr && IsProtectionZone(dest) && !IsProtectionZone(origin)`
        //   → throw ENTERPROTECTIONZONE.
        // Reuses the same `PlayerIsPzLocked` mapping as the self-walk PZ-enter gate
        // (`walk_tile.rs:639`).
        if earliest_pz_round > round_nr
            && self.tile_in_protection_zone(dest)
            && !self.tile_in_protection_zone(origin)
        {
            return Err(ReturnValue::PlayerIsPzLocked);
        }

        // House-invite gate (`crplayer.cc:372-377`):
        //   `HouseID != 0 && !IsInvited(HouseID, this, INT_MAX) && !CheckRight(this->ID, ENTER_HOUSES)`
        //   → throw NOTINVITED.
        // `GetHouseID(x, y, z)` — extract from `Tile::House`.
        let house_id = self
            .map
            .get_tile(dest)
            .and_then(|t| match t {
                Tile::House(h) => Some(h.house_id),
                _ => None,
            })
            .unwrap_or(0);
        if house_id != 0
            && !self.houses.is_invited(house_id, guid)
            && !crate::player_flags::has_player_flag(group_flags, PLAYER_FLAG_CAN_EDIT_HOUSES)
        {
            return Err(ReturnValue::PlayerIsNotInvited);
        }

        Ok(true)
    }

    /// 772 `TNPC::MovePossible(x, y, z, Execute, Jump)` — `crnonpl.cc:1672-1680`.
    ///
    /// `BANK && !UNPASS && !AVOID && z==startz && within Radius && !House`.
    /// Pure boolean — no throws — so `Ok(bool)` suffices but keeps the `Result` signature
    /// for uniformity with the player/monster arms.
    pub(crate) fn npc_move_possible_push(
        &self,
        cid: CreatureId,
        dest: Position,
    ) -> Result<bool, ReturnValue> {
        let (home, radius) = match self.creatures.get(cid) {
            Some(CreatureKind::Npc(n)) => (n.runtime.home_position, n.runtime.radius as i32),
            _ => return Ok(false),
        };
        // `CoordinateFlag(BANK) && !CoordinateFlag(UNPASS)`.
        if !self.tile_is_bank_and_passable(dest) {
            return Ok(false);
        }
        // `!CoordinateFlag(AVOID)`.
        if self.tile_has_avoid(dest) {
            return Ok(false);
        }
        // `z == startz`.
        if dest.z != home.z {
            return Ok(false);
        }
        // `abs(x - startx) <= Radius && abs(y - starty) <= Radius`.
        if (dest.x as i32 - home.x as i32).unsigned_abs() as i32 > radius
            || (dest.y as i32 - home.y as i32).unsigned_abs() as i32 > radius
        {
            return Ok(false);
        }
        // `!IsHouse(x, y, z)`.
        if self
            .map
            .get_tile(dest)
            .is_some_and(|t| matches!(t, Tile::House(_)))
        {
            return Ok(false);
        }
        Ok(true)
    }

    /// 772 `TMonster::MovePossible(x, y, z, Execute=true, Jump)` — `crnonpl.cc:2141-2293`.
    ///
    /// Pre-tile gates (same-z, home-range, per-creature `Radius`, `GO_STRENGTH`, `!PZ`,
    /// `!House`, summon anti-crowd C4) + the kick loop (`crnonpl.cc:2185-2288`). **C3:** the
    /// kick loop throws `EXHAUSTED` (`crnonpl.cc:2237` player blocker, `:2240` failed
    /// `KickCreature`) and sets `Target = 0` before throwing — this is `Err(YouAreExhausted)`
    /// with the `Target = 0` side effect applied by `monster_kick_before_step`.
    ///
    /// Reuses `monster_move_possible_planning` for the pre-tile + tile-stack planning gates
    /// (home range, PZ, house, creatures, items) and `monster_kick_before_step` for the
    /// execute-mode kick loop (`KickCreature`/`KickBoxes` with retry).
    pub(crate) fn monster_move_possible_push(
        &mut self,
        cid: CreatureId,
        dest: Position,
    ) -> Result<bool, ReturnValue> {
        // ── Pre-tile gates not covered by `monster_move_possible_planning` ──

        // `crnonpl.cc:2142-2146`: `posz != z` → false. C5 already gates this (only called
        // when `dz == 0` for monsters), but guard defensively.
        let (cur_pos, speed, state, master, radius) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.base.position,
                m.base.speed,
                m.state,
                m.base.master,
                m.radius,
            ),
            _ => return Ok(false),
        };
        if cur_pos.z != dest.z {
            return Ok(false);
        }

        // `crnonpl.cc:2154-2159`: per-creature `Radius` (unless ATTACKING/PANIC).
        // `Distance = max(abs(x - posx), abs(y - posy))`; `Distance > Radius` → false.
        // V2: `Radius` is per-creature (default `i32::MAX`), not a race flag.
        if state != MonsterState::Attacking && state != MonsterState::Panic {
            let distance = std::cmp::max(
                (dest.x as i32 - cur_pos.x as i32).unsigned_abs() as i32,
                (dest.y as i32 - cur_pos.y as i32).unsigned_abs() as i32,
            );
            if distance > radius {
                return Ok(false);
            }
        }

        // `crnonpl.cc:2162-2166`: `GO_STRENGTH` (`Skills[SKILL_GO_STRENGTH]->Act < 0`).
        // Q1: maps to `CreatureBase::speed` (`creature/base.rs:193`); check is `speed < 0`.
        if speed < 0 {
            return Ok(false);
        }

        // `crnonpl.cc:2171-2181`: summon anti-crowd (C4).
        // `Execute && Master != 0 && State ∉ {ATTACKING, PANIC}` and master on same z:
        // reject when summon currently **not** adjacent (manhattan > 1) and dest **would be**
        // adjacent (≤ 1). This is the inverse of a leash — stops a summon from snapping onto
        // the master's tile. Do NOT implement a distance cap.
        if master.is_some() && state != MonsterState::Attacking && state != MonsterState::Panic {
            if let Some(master_cid) = master {
                if let Some(master_k) = self.creatures.get(master_cid) {
                    let master_pos = master_k.position();
                    if master_pos.z == cur_pos.z {
                        let cur_dist = (master_pos.x as i32 - cur_pos.x as i32).unsigned_abs() as i32
                            + (master_pos.y as i32 - cur_pos.y as i32).unsigned_abs() as i32;
                        let dest_dist = (master_pos.x as i32 - dest.x as i32).unsigned_abs() as i32
                            + (master_pos.y as i32 - dest.y as i32).unsigned_abs() as i32;
                        // Anti-crowd: currently not adjacent (> 1) and dest would be adjacent (≤ 1).
                        if cur_dist > 1 && dest_dist <= 1 {
                            return Ok(false);
                        }
                    }
                }
            }
        }

        // ── Planning gate: home range, PZ, house, tile-stack (Execute=false) ──
        // `monster_move_possible_planning` covers `MonsterhomeInRange` (`crnonpl.cc:2149`),
        // `IsProtectionZone || IsHouse` (`crnonpl.cc:2168`), and the tile-stack creature/item
        // checks. Pushable creatures are plannable-through (the kick loop handles them).
        // D6: 772 `MovePossible` has no FLOORCHANGE|TELEPORT check (`crnonpl.cc:2141-2293`) —
        // a pushed monster can land on stairs/teleport. Pass `allow_floorchange_teleport = true`.
        if !self.monster_move_possible_planning(cid, dest, true) {
            return Ok(false);
        }

        // ── Kick loop (Execute=true) ──
        // `crnonpl.cc:2185-2288`: kicks blocking creatures/boxes on `dest` with retry.
        // Reuses `monster_kick_before_step` (already implements `KickCreature`/`KickBoxes`
        // for self-walk from `on_walk`). V1: do NOT skip the kick loop.
        let now = Instant::now();
        let outcome = self.monster_kick_before_step(cid, dest, now);
        match outcome {
            // C3: `EXHAUSTED` — player blocker (`Target = 0` applied at the throw site,
            // `crnonpl.cc:2237`) or failed `KickCreature` (`Target` preserved,
            // `crnonpl.cc:2241-2242`). The `Target = 0` side effect is applied here (the
            // throw site), not by `monster_kick_before_step` — it only returns the outcome.
            MonsterKickOutcome::ExhaustedDropTarget => {
                // `crnonpl.cc:2237`: `this->Target = 0; throw EXHAUSTED`.
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().clear_targets();
                }
                return Err(ReturnValue::YouAreExhausted);
            }
            MonsterKickOutcome::Exhausted => {
                // `crnonpl.cc:2241-2242`: `KickCreature` failed → `throw EXHAUSTED`
                // (Target preserved — the `Execute` catch `cract.cc:870-877` keeps it).
                return Err(ReturnValue::YouAreExhausted);
            }
            MonsterKickOutcome::Proceed => {}
        }

        // ── Post-kick tile check ──
        // After the kick loop, re-check if `dest` is now occupiable. The kick loop may have
        // cleared pushable creatures/boxes, or hit a hard block (unpushable, NPC, target,
        // master, invisible, summon-player, IGNORED). `monster_move_possible_planning` returns
        // `false` for hard blocks and `true` when the tile is clear (pushable creatures were
        // kicked or killed by the loop above).
        // D6: push path — allow FLOORCHANGE|TELEPORT (772 `MovePossible` has no such check).
        if self.monster_move_possible_planning(cid, dest, true) {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
