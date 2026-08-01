//! Player request / change outfit (`0xD2` / `0xD3`).
//!
//! - Domain: TFS `Game::playerRequestOutfit` / `playerChangeOutfit` / `internalCreatureChangeOutfit`.
//! - Outcomes / wire 772: `gameserver/src/protocolgame.cpp` `sendOutfitWindow`, `parseSetOutfit`;
//!   `gameserver/src/game.cpp` `playerChangeOutfit` (no addons/mount).
//! - 1098: repo-root `src/game.cpp` / `protocolgame.cpp` (addons; mounts deferred).

use tfs_rust_common::enums::ConditionType;
use tfs_rust_common::game_packet::SetOutfitPayload;
use tfs_rust_common::{ConnId, PlayerSex, CLIENTOS_OTCLIENT_LINUX};
use tfs_rust_net::creature_encode::OutfitWire;
use tfs_rust_net::outgoing_extra::{
    send_outfit_window, send_outfit_window_772_classic, send_outfit_window_772_otclient,
};
use tfs_rust_net::Codec;

use crate::creature::{CreatureKind, Outfit};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::login_out::creature_wire_id;

impl GameWorld {
    /// `Game::playerRequestOutfit` — open outfit dialog (`0xC8`).
    pub fn player_request_outfit(&mut self, conn_id: ConnId, cid: CreatureId) {
        if !self.allow_change_outfit() {
            return;
        }
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };

        let sex = p.sex;
        let sex_byte = match sex {
            PlayerSex::Female => 0u8,
            PlayerSex::Male => 1u8,
        };
        let is_premium = self.player_is_premium(cid);
        let is_access = self.player_is_access_player(cid);
        let otclient = p.operating_system >= CLIENTOS_OTCLIENT_LINUX || p.otclient_v8 != 0;

        let current = {
            let o = &p.base.outfit;
            let mut look_type = o.look_type.max(0) as u16;
            if look_type == 0 {
                if let Some(first) = self.outfits_db.outfits_for_sex(sex_byte).first() {
                    look_type = first.looktype;
                }
            }
            OutfitWire {
                look_type,
                look_head: o.look_head.clamp(0, 255) as u8,
                look_body: o.look_body.clamp(0, 255) as u8,
                look_legs: o.look_legs.clamp(0, 255) as u8,
                look_feet: o.look_feet.clamp(0, 255) as u8,
                look_addons: o.look_addons.clamp(0, 255) as u8,
                look_mount: 0,
                look_type_ex: 0,
            }
        };

        let msg = match &self.codec {
            Codec::V772(_) => {
                if otclient {
                    let mut list: Vec<(u16, String)> = Vec::new();
                    if is_access {
                        list.push((75, "Gamemaster".into()));
                    }
                    for outfit in self.outfits_db.outfits_for_sex(sex_byte) {
                        if list.len() >= u8::MAX as usize {
                            break;
                        }
                        // OTClient list shows all enabled outfits for sex (gameserver loop).
                        list.push((outfit.looktype, outfit.name.clone()));
                    }
                    let refs: Vec<(u16, &str)> =
                        list.iter().map(|(lt, n)| (*lt, n.as_str())).collect();
                    send_outfit_window_772_otclient(&current, &refs)
                } else {
                    let (first, last) = classic_772_looktype_range(sex, is_premium);
                    send_outfit_window_772_classic(&current, first, last)
                }
            }
            Codec::V1098(_) => {
                let owned = match self.creatures.get(cid) {
                    Some(CreatureKind::Player(p)) => p.outfits.clone(),
                    _ => Vec::new(),
                };
                let mut list: Vec<(u16, String, u8)> = Vec::new();
                if is_access {
                    list.push((75, "Gamemaster".into(), 0));
                }
                for outfit in self.outfits_db.outfits_for_sex(sex_byte) {
                    if list.len() >= u8::MAX as usize {
                        break;
                    }
                    let Some(addons) =
                        self.outfit_addons_for_window(is_access, is_premium, outfit, &owned)
                    else {
                        continue;
                    };
                    list.push((outfit.looktype, outfit.name.clone(), addons));
                }
                let refs: Vec<(u16, &str, u8)> = list
                    .iter()
                    .map(|(lt, n, a)| (*lt, n.as_str(), *a))
                    .collect();
                send_outfit_window(&current, &refs, &[])
            }
        };
        self.enqueue_encoded(conn_id, msg);
    }

    /// `Game::playerChangeOutfit` — apply client outfit selection.
    pub fn player_change_outfit(
        &mut self,
        _conn_id: ConnId,
        cid: CreatureId,
        payload: SetOutfitPayload,
    ) {
        if !self.allow_change_outfit() {
            return;
        }
        let addons = match &self.codec {
            Codec::V772(_) => 0u8,
            Codec::V1098(_) => payload.look_addons,
        };
        // Mounts not wired yet — ignore client lookMount (same as missing mount entry).
        let _ = payload.look_mount;

        if !self.player_can_wear(cid, payload.look_type, addons) {
            return;
        }

        let new_outfit = Outfit {
            look_type: i32::from(payload.look_type),
            look_head: i32::from(payload.look_head),
            look_body: i32::from(payload.look_body),
            look_legs: i32::from(payload.look_legs),
            look_feet: i32::from(payload.look_feet),
            look_addons: i32::from(addons),
        };

        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
            return;
        };
        p.base.outfit = new_outfit.clone();

        let has_outfit_condition = p
            .base
            .active_conditions
            .iter()
            .any(|c| c.ctype == ConditionType::Outfit);
        if has_outfit_condition {
            return;
        }

        self.internal_creature_change_outfit(cid, &new_outfit);
    }

    /// `Game::internalCreatureChangeOutfit` — set current look + spectator `0x8E`.
    pub fn internal_creature_change_outfit(&mut self, cid: CreatureId, outfit: &Outfit) {
        // Lua `Creature:onChangeOutfit` veto deferred — default allow (events return true).
        if let Some(kind) = self.creatures.get_mut(cid) {
            kind.base_mut().outfit = outfit.clone();
        }

        let Some(kind) = self.creatures.get(cid) else {
            return;
        };
        if kind
            .base()
            .active_conditions
            .iter()
            .any(|c| c.ctype == ConditionType::Invisible)
        {
            return;
        }

        let pos = kind.position();
        let wire_id = creature_wire_id(cid, kind);
        let wire = OutfitWire {
            look_type: outfit.look_type.max(0) as u16,
            look_head: outfit.look_head.clamp(0, 255) as u8,
            look_body: outfit.look_body.clamp(0, 255) as u8,
            look_legs: outfit.look_legs.clamp(0, 255) as u8,
            look_feet: outfit.look_feet.clamp(0, 255) as u8,
            look_addons: outfit.look_addons.clamp(0, 255) as u8,
            look_mount: 0,
            look_type_ex: 0,
        };
        let msg = self.codec.encode_creature_outfit(wire_id, &wire);
        self.broadcast_to_spectators(pos, msg.into_bytes());
    }

    /// `Player::canWear` — access / premium / unlocked / owned addons.
    pub fn player_can_wear(&self, cid: CreatureId, look_type: u16, addons: u8) -> bool {
        if self.player_is_access_player(cid) {
            return true;
        }
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        let sex_byte = match p.sex {
            PlayerSex::Female => 0u8,
            PlayerSex::Male => 1u8,
        };
        let Some(def) = self.outfits_db.get_by_sex_looktype(sex_byte, look_type) else {
            return false;
        };
        if def.premium && !self.player_is_premium(cid) {
            return false;
        }

        match &self.codec {
            Codec::V772(_) => {
                if def.unlocked {
                    return true;
                }
                p.outfits.iter().any(|e| e.look_type == look_type)
            }
            Codec::V1098(_) => {
                if def.unlocked && addons == 0 {
                    return true;
                }
                for entry in &p.outfits {
                    if entry.look_type != look_type {
                        continue;
                    }
                    return entry.addons == addons || entry.addons == 3 || addons == 0;
                }
                false
            }
        }
    }

    fn allow_change_outfit(&self) -> bool {
        self.config
            .get_bool("allowChangeOutfit")
            .unwrap_or(true)
    }

    /// `Player::getOutfitAddons` — returns `None` when the outfit is not shown in the window.
    fn outfit_addons_for_window(
        &self,
        is_access: bool,
        is_premium: bool,
        outfit: &tfs_rust_content::outfits::Outfit,
        owned: &[crate::creature::OutfitEntry],
    ) -> Option<u8> {
        if is_access {
            return Some(3);
        }
        if outfit.premium && !is_premium {
            return None;
        }
        for entry in owned {
            if entry.look_type == outfit.looktype {
                return Some(entry.addons);
            }
        }
        if !outfit.unlocked {
            return None;
        }
        Some(0)
    }
}

/// Stock 7.72 client looktype range — `gameserver` `sendOutfitWindow` else-branch.
fn classic_772_looktype_range(sex: PlayerSex, premium: bool) -> (u16, u16) {
    match sex {
        PlayerSex::Male => {
            if premium {
                (128, 134)
            } else {
                (128, 131)
            }
        }
        PlayerSex::Female => {
            if premium {
                (136, 142)
            } else {
                (136, 139)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_harness::{
        beat_driven_test_world, insert_spectator_player, test_player,
    };
    use tfs_rust_common::{Position, ProtocolVersion};
    use tfs_rust_content::outfits::{Outfit, OutfitDatabase};
    use tfs_rust_net::NetworkMessage;

    fn load_test_outfits() -> OutfitDatabase {
        let mut outfits = std::collections::HashMap::new();
        for (sex, base) in [(1u8, 128u16), (0u8, 136u16)] {
            for i in 0..7u16 {
                let looktype = base + i;
                outfits.insert(
                    looktype,
                    Outfit {
                        looktype,
                        outfit_type: sex,
                        name: format!("Outfit{looktype}"),
                        premium: i >= 4,
                        unlocked: true,
                        enabled: true,
                    },
                );
            }
        }
        OutfitDatabase { outfits }
    }

    fn setup_player(world: &mut GameWorld, name: &str, conn: ConnId) -> CreatureId {
        let pos = Position::new(100, 100, 7);
        insert_spectator_player(world, conn, test_player(name, pos))
    }

    #[test]
    fn change_outfit_updates_look_and_broadcasts_0x8e() {
        let mut world = beat_driven_test_world();
        world.outfits_db = std::sync::Arc::new(load_test_outfits());
        world.codec = Codec::from_version(ProtocolVersion::V772).expect("772");
        let conn = ConnId(1);
        let cid = setup_player(&mut world, "Hero", conn);

        world.player_change_outfit(
            conn,
            cid,
            SetOutfitPayload {
                look_type: 130,
                look_head: 10,
                look_body: 20,
                look_legs: 30,
                look_feet: 40,
                look_addons: 0,
                look_mount: 0,
            },
        );

        let CreatureKind::Player(p) = world.creatures.get(cid).unwrap() else {
            panic!("player");
        };
        assert_eq!(p.base.outfit.look_type, 130);
        assert_eq!(p.base.outfit.look_head, 10);

        let pkts = world.pending_outgoing.get(&conn).cloned().unwrap_or_default();
        assert!(
            pkts.iter().any(|b| b.first() == Some(&0x8E)),
            "expected CREATURE_OUTFIT 0x8E broadcast, got {pkts:?}"
        );
    }

    #[test]
    fn request_outfit_772_classic_sends_0xc8_range() {
        let mut world = beat_driven_test_world();
        world.outfits_db = std::sync::Arc::new(load_test_outfits());
        world.codec = Codec::from_version(ProtocolVersion::V772).expect("772");
        let conn = ConnId(2);
        let cid = setup_player(&mut world, "Citizen", conn);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.operating_system = 1; // stock client
            p.sex = PlayerSex::Male;
            p.premium_ends_at = 0;
        }

        world.player_request_outfit(conn, cid);
        let pkts = world.pending_outgoing.remove(&conn).unwrap_or_default();
        let body = pkts
            .into_iter()
            .find(|b| b.first() == Some(&0xC8))
            .expect("0xC8");
        assert!(body.len() >= 1 + 6 + 4);
        let mut msg = NetworkMessage::from_bytes(&body[1..]);
        let look = msg.read_u16().unwrap();
        assert!(look != 0);
        let _h = msg.read_u8().unwrap();
        let _b = msg.read_u8().unwrap();
        let _l = msg.read_u8().unwrap();
        let _f = msg.read_u8().unwrap();
        assert_eq!(msg.read_u16().unwrap(), 128);
        assert_eq!(msg.read_u16().unwrap(), 131); // free account
    }

    #[test]
    fn parse_set_outfit_772_ignores_trailing_addons() {
        use tfs_rust_common::game_packet::GamePacket;
        use tfs_rust_net::game_parse::parse_game_opcode;

        let mut msg = NetworkMessage::new();
        msg.write_u16(128);
        msg.write_u8(1);
        msg.write_u8(2);
        msg.write_u8(3);
        msg.write_u8(4);
        let pkt = parse_game_opcode(0xD3, &mut msg, ProtocolVersion::V772).unwrap();
        match pkt {
            GamePacket::SetOutfit(p) => {
                assert_eq!(p.look_type, 128);
                assert_eq!(p.look_addons, 0);
                assert_eq!(p.look_mount, 0);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn premium_outfit_rejected_for_free_account() {
        let mut world = beat_driven_test_world();
        world.outfits_db = std::sync::Arc::new(load_test_outfits());
        world.codec = Codec::from_version(ProtocolVersion::V772).expect("772");
        let conn = ConnId(3);
        let cid = setup_player(&mut world, "Free", conn);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.premium_ends_at = 0;
            p.base.outfit.look_type = 128;
        }

        world.player_change_outfit(
            conn,
            cid,
            SetOutfitPayload {
                look_type: 134, // premium warrior
                look_head: 0,
                look_body: 0,
                look_legs: 0,
                look_feet: 0,
                look_addons: 0,
                look_mount: 0,
            },
        );
        let CreatureKind::Player(p) = world.creatures.get(cid).unwrap() else {
            panic!("player");
        };
        assert_eq!(p.base.outfit.look_type, 128);
    }
}
