//! PvP / world-type checks for attacking (`Combat::canTargetCreature` / `canDoCombat` subset).
// C++ reference: `combat.cpp` `Combat::canTargetCreature`, `isProtected`, `isInPvpZone`.

use tfs_rust_common::enums::{SkullType, WorldType, ZoneType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatDenyReason {
    /// Attacker cannot initiate PvP (secure mode vs unmarked).
    SecureModeUnmarkedTarget,
    /// `protectionLevel` or vocation disallows PvP.
    Protected,
    /// `WORLD_TYPE_NO_PVP` and not both in open PvP tiles.
    NoPvpWorld,
    /// Target in no-PvP zone (player vs player).
    TargetNoPvpZone,
    /// Attacker in no-PvP zone shooting out (player vs player).
    AttackerNoPvpZone,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerPvpSnapshot {
    pub level: u32,
    pub skull: SkullType,
    /// What the attacker sees on the target’s skull (unmarked = `SkullType::None`).
    pub target_skull_client: SkullType,
    pub zone: ZoneType,
    pub vocation_allows_pvp: bool,
    pub secure_mode: bool,
}

/// `protectionLevel` from `config.lua`; both players must be ≥ this level to fight (else protected).
#[inline]
pub fn is_protected(
    protection_level: u32,
    attacker: &PlayerPvpSnapshot,
    target: &PlayerPvpSnapshot,
) -> bool {
    if target.level < protection_level || attacker.level < protection_level {
        return true;
    }
    if !attacker.vocation_allows_pvp || !target.vocation_allows_pvp {
        return true;
    }
    if attacker.skull == SkullType::Black && target.target_skull_client == SkullType::None {
        return true;
    }
    false
}

#[inline]
pub fn is_in_pvp_zone(attacker_zone: ZoneType, target_zone: ZoneType) -> bool {
    attacker_zone == ZoneType::Pvp && target_zone == ZoneType::Pvp
}

/// Player attacking player (after tile / flag checks elsewhere).
pub fn can_player_attack_player(
    protection_level: u32,
    world_type: WorldType,
    attacker: &PlayerPvpSnapshot,
    target: &PlayerPvpSnapshot,
) -> Result<(), CombatDenyReason> {
    if is_protected(protection_level, attacker, target) {
        return Err(CombatDenyReason::Protected);
    }

    if attacker.secure_mode
        && !is_in_pvp_zone(attacker.zone, target.zone)
        && target.target_skull_client == SkullType::None
    {
        return Err(CombatDenyReason::SecureModeUnmarkedTarget);
    }

    if world_type == WorldType::NoPvp && !is_in_pvp_zone(attacker.zone, target.zone) {
        return Err(CombatDenyReason::NoPvpWorld);
    }

    if target.zone == ZoneType::NoPvp {
        return Err(CombatDenyReason::TargetNoPvpZone);
    }

    if attacker.zone == ZoneType::NoPvp
        && target.zone != ZoneType::NoPvp
        && target.zone != ZoneType::Protection
    {
        return Err(CombatDenyReason::AttackerNoPvpZone);
    }

    Ok(())
}
