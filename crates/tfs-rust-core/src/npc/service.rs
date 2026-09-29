//! One requirement check for bless, promotion, spells, and travel.
//!
//! Pack surface: TFS `StdModule.bless` / `promote` / `learnSpell` / `travel`.
//! Default purse is inventory gold (`DeleteMoney`). `purse = "total"` also spends bank gold.

use tfs_rust_common::Position;
use tfs_rust_content::npcs::{ServiceKind, ServiceOffer, ServicePurse};

use super::actions::NpcActionHost;
use crate::creature::{CreatureKind, NpcActivity};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::player_money_lib::player_remove_total_money;
use crate::spell_learn::persist_knows_spell_nr;

const STORAGE_PROMOTION: u32 = 30018;
const TRAVEL_EFFECT: u8 = 11;

impl GameWorld {
    pub(crate) fn npc_say_destinations(&mut self, npc: CreatureId) -> Result<(), String> {
        let def_id = match self.creatures.get(npc) {
            Some(CreatureKind::Npc(n)) => n.definition,
            _ => return Err("destinations: npc not found".into()),
        };
        let names: Vec<String> = self
            .npcs_db
            .get(def_id)
            .map(|def| def.destinations.iter().map(|d| d.name.clone()).collect())
            .unwrap_or_default();
        let text = if names.is_empty() {
            "I do not travel.".to_string()
        } else {
            format!("I sail to {}.", names.join(", "))
        };
        self.enqueue_creature_talk(npc, text);
        Ok(())
    }

    pub(crate) fn apply_npc_service(
        &mut self,
        npc: CreatureId,
        player: CreatureId,
        offer: &ServiceOffer,
    ) -> Result<(), String> {
        let resolved = self.resolve_service(npc, offer)?;
        if resolved.premium && !self.player_is_premium(player) {
            return self.service_refuse(npc, "You need a premium account.");
        }
        let (level, blessings, pz_locked, vocation) = match self.creatures.get(player) {
            Some(CreatureKind::Player(p)) => (
                p.level,
                p.blessings,
                p.earliest_protection_zone_round > self.round_nr,
                p.vocation_id,
            ),
            _ => return Err("service: player not found".into()),
        };
        if resolved.level > 0 && level < i32::try_from(resolved.level).unwrap_or(i32::MAX) {
            return self.service_refuse(npc, "You do not have the required level.");
        }
        if resolved.pz_clear && pz_locked {
            return self.service_refuse(npc, "You cannot do this while fighting.");
        }
        if self.service_already(player, vocation, blessings, &resolved.kind) {
            return self.service_refuse(npc, "You already have this.");
        }
        if resolved.price > 0 && !self.service_can_pay(player, resolved.price, resolved.purse) {
            return self.service_refuse(npc, "You do not have enough money.");
        }
        self.service_charge(player, resolved.price, resolved.purse)?;
        self.service_grant(npc, player, &resolved)?;
        if !resolved.text.is_empty() {
            self.enqueue_creature_talk(npc, resolved.text);
        }
        Ok(())
    }

    fn resolve_service(
        &self,
        npc: CreatureId,
        offer: &ServiceOffer,
    ) -> Result<ServiceOffer, String> {
        let ServiceKind::Travel {
            destination: Some(name),
            ..
        } = &offer.kind
        else {
            return Ok(offer.clone());
        };
        let def_id = match self.creatures.get(npc) {
            Some(CreatureKind::Npc(n)) => n.definition,
            _ => return Err("travel: npc not found".into()),
        };
        let dest = self
            .npcs_db
            .get(def_id)
            .and_then(|def| {
                def.destinations
                    .iter()
                    .find(|d| d.name.eq_ignore_ascii_case(name))
            })
            .cloned()
            .ok_or_else(|| format!("travel: unknown destination {name}"))?;
        let mut resolved = offer.clone();
        resolved.kind = ServiceKind::Travel {
            x: dest.x,
            y: dest.y,
            z: dest.z,
            destination: Some(dest.name),
        };
        if dest.price > 0 {
            resolved.price = dest.price;
        }
        resolved.premium = resolved.premium || dest.premium;
        resolved.level = resolved.level.max(dest.level);
        Ok(resolved)
    }

    fn service_already(
        &self,
        player: CreatureId,
        vocation: i32,
        blessings: i8,
        kind: &ServiceKind,
    ) -> bool {
        match kind {
            ServiceKind::Bless { index } => {
                let bit = index.saturating_sub(1);
                (0..=5).contains(&bit) && (blessings & 1i8.wrapping_shl(bit as u32)) != 0
            }
            ServiceKind::Promote => self.player_get_storage(player, STORAGE_PROMOTION) == 1,
            ServiceKind::Spell { spell } => match self.creatures.get(player) {
                Some(CreatureKind::Player(p)) => p
                    .persist
                    .as_ref()
                    .is_some_and(|b| persist_knows_spell_nr(&b.spells, *spell)),
                _ => false,
            },
            ServiceKind::Travel { .. } => {
                let _ = vocation;
                false
            }
        }
    }

    fn service_can_pay(&self, player: CreatureId, price: u32, purse: ServicePurse) -> bool {
        let coins = self.player_count_money(player);
        let have = match purse {
            ServicePurse::Inventory => coins,
            ServicePurse::Total => {
                let bank = match self.creatures.get(player) {
                    Some(CreatureKind::Player(p)) => p.economy.balance,
                    _ => 0,
                };
                coins.saturating_add(bank)
            }
        };
        have >= u64::from(price)
    }

    fn service_charge(
        &mut self,
        player: CreatureId,
        price: u32,
        purse: ServicePurse,
    ) -> Result<(), String> {
        if price == 0 {
            return Ok(());
        }
        match purse {
            ServicePurse::Inventory => {
                self.player_delete_money(player, i32::try_from(price).unwrap_or(i32::MAX))
            }
            ServicePurse::Total => {
                if player_remove_total_money(self, player, u64::from(price)) {
                    Ok(())
                } else {
                    Err("You do not have enough money.".into())
                }
            }
        }
    }

    fn service_grant(
        &mut self,
        npc: CreatureId,
        player: CreatureId,
        offer: &ServiceOffer,
    ) -> Result<(), String> {
        match &offer.kind {
            ServiceKind::Bless { index } => self.add_blessing(player, *index),
            ServiceKind::Promote => self.promote(player),
            ServiceKind::Spell { spell } => self.teach_spell(player, *spell),
            ServiceKind::Travel { x, y, z, .. } => {
                let origin = self
                    .creatures
                    .get(player)
                    .map(|k| k.position())
                    .ok_or_else(|| "travel: player not found".to_string())?;
                let dest = Position {
                    x: u16::try_from(*x).map_err(|_| format!("travel: bad x {x}"))?,
                    y: u16::try_from(*y).map_err(|_| format!("travel: bad y {y}"))?,
                    z: u8::try_from(*z).map_err(|_| format!("travel: bad z {z}"))?,
                };
                self.teleport(player, *x, *y, *z)?;
                self.broadcast_magic_effect(origin, TRAVEL_EFFECT);
                self.broadcast_magic_effect(dest, TRAVEL_EFFECT);
                if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc) {
                    n.runtime.activity = NpcActivity::Idle;
                    n.runtime.focus = None;
                }
                Ok(())
            }
        }
    }

    fn service_refuse(&mut self, npc: CreatureId, text: &str) -> Result<(), String> {
        self.enqueue_creature_talk(npc, text);
        Err(text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        ensure_walkable_tile, insert_npc, insert_player, minimal_world, test_player,
    };
    use tfs_rust_common::Position;
    use tfs_rust_content::npcs::SourceSpan;

    fn offer(kind: ServiceKind, price: u32, purse: ServicePurse) -> ServiceOffer {
        ServiceOffer {
            kind,
            price,
            premium: false,
            level: 0,
            pz_clear: false,
            purse,
            text: String::new(),
            span: SourceSpan::default(),
        }
    }

    fn fixture() -> (GameWorld, CreatureId, CreatureId) {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let player = insert_player(&mut world, test_player("Pilgrim", pos));
        let npc = insert_npc(&mut world, "Monk", pos, 100);
        (world, player, npc)
    }

    #[test]
    fn not_enough_gold_leaves_blessings_unchanged() {
        let (mut world, player, npc) = fixture();
        let before = match world.creatures.get(player) {
            Some(CreatureKind::Player(p)) => p.blessings,
            _ => panic!("player"),
        };
        let err = world
            .apply_npc_service(
                npc,
                player,
                &offer(
                    ServiceKind::Bless { index: 1 },
                    10_000,
                    ServicePurse::Inventory,
                ),
            )
            .expect_err("short gold");
        assert!(err.contains("enough money"));
        match world.creatures.get(player) {
            Some(CreatureKind::Player(p)) => assert_eq!(p.blessings, before),
            _ => panic!("player"),
        }
        assert_eq!(world.player_count_money(player), 0);
    }

    #[test]
    fn total_purse_can_spend_bank() {
        let (mut world, player, npc) = fixture();
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.economy.balance = 10_000;
        }
        world
            .apply_npc_service(
                npc,
                player,
                &offer(ServiceKind::Bless { index: 1 }, 10_000, ServicePurse::Total),
            )
            .expect("bank pays");
        match world.creatures.get(player) {
            Some(CreatureKind::Player(p)) => {
                assert_ne!(p.blessings, 0);
                assert_eq!(p.economy.balance, 0);
            }
            _ => panic!("player"),
        }
    }
}
