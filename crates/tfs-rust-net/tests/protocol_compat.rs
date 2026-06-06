//! Golden-byte checks for outgoing game packets against the C++ implementation in this repo.
//! Phase A1 regression gate — bytes must match pre-codec output.
// C++ reference: `src/protocolgame.cpp` (`ProtocolGame::send*`).

use tfs_rust_common::{Position, ProtocolVersion};
use tfs_rust_net::codec::{
    AddCreatureWire, Codec, Codec1098, ContainerOpenWire, ItemTemplateArgs, OutfitWire,
    PlayerSkillsWire, PlayerStatsWire,
};
use tfs_rust_net::creature_encode::write_add_creature;
use tfs_rust_net::map_description::send_map_description_stub;
use tfs_rust_net::outgoing::{
    send_creature_health, send_extended_opcode, send_magic_effect, send_otcv8_features, send_ping,
    send_ping_back, send_text_message,
};
use tfs_rust_net::outgoing_extra::send_unjustified_stats_stub;
use tfs_rust_net::{item_encode::write_item_template, NetworkMessage};

/// `GameFeature::GameExtendedOpcode` / `GameItemTooltip` (`src/const.h`) — same pair as `ProtocolGame::sendFeatures`.
const GAME_EXTENDED_OPCODE: u8 = 80;
const GAME_ITEM_TOOLTIP: u8 = 93;

fn codec() -> Codec {
    Codec::from_version(ProtocolVersion::V1098).expect("1098 codec")
}

#[test]
fn ping_and_ping_back() {
    assert_eq!(send_ping().as_bytes(), &[0x1D]);
    assert_eq!(send_ping_back().as_bytes(), &[0x1E]);
}

#[test]
fn magic_effect_encoding() {
    let pos = Position::new(0x0102, 0x0304, 5);
    let m = send_magic_effect(pos, 7);
    assert_eq!(m.as_bytes(), &[0x83, 0x02, 0x01, 0x04, 0x03, 0x05, 0x07]);
}

#[test]
fn creature_health_encoding() {
    let m = send_creature_health(0x11223344, 88);
    assert_eq!(m.as_bytes(), &[0x8C, 0x44, 0x33, 0x22, 0x11, 0x58]);
}

#[test]
fn text_message_encoding() {
    let m = send_text_message(0x16, "hello");
    assert_eq!(
        m.as_bytes(),
        &[0xB4, 0x16, 0x05, 0x00, b'h', b'e', b'l', b'l', b'o']
    );
}

#[test]
fn extended_opcode_encoding() {
    let m = send_extended_opcode(0xAB, "x");
    assert_eq!(m.as_bytes(), &[0x32, 0xAB, 0x01, 0x00, b'x']);
}

#[test]
fn otcv8_features_encoding_matches_send_features() {
    let m = send_otcv8_features(&[(GAME_EXTENDED_OPCODE, true), (GAME_ITEM_TOOLTIP, true)]);
    assert_eq!(
        m.as_bytes(),
        &[
            0x43,
            0x02,
            0x00,
            GAME_EXTENDED_OPCODE,
            0x01,
            GAME_ITEM_TOOLTIP,
            0x01
        ]
    );
}

#[test]
fn map_description_stub_encoding() {
    let p = Position::new(10, 20, 7);
    let m = send_map_description_stub(p, p);
    assert_eq!(m.as_bytes(), &[0x64, 10, 0, 20, 0, 7, 0xFF, 0xFF]);
}

/// `docs/OTCLIENT_INFO.md` §1 — `parseUnjustifiedStats`: opcode + 7× u8.
#[test]
fn unjustified_stats_stub_is_seven_payload_bytes() {
    let m = send_unjustified_stats_stub();
    let b = m.as_bytes();
    assert_eq!(b.len(), 1 + 7);
    assert_eq!(b[0], 0xB7);
    assert!(b[1..].iter().all(|&x| x == 0));
}

/// `docs/OTCLIENT_INFO.md` §2 — 13 skills with `GameAdditionalSkills`: 35 + 24 bytes after opcode.
#[test]
fn player_skills_1098_otc_thirteen_skill_layout_length() {
    let levels = [1u16, 2, 3, 4, 5, 6, 7];
    let bases = levels;
    let percents = [0u8; 7];
    let add_lv = [10u16; 6];
    let add_bs = [10u16; 6];
    let msg = codec().encode_player_skills(&PlayerSkillsWire {
        levels,
        bases,
        percents,
        additional_levels: add_lv,
        additional_bases: add_bs,
    });
    let b = msg.as_bytes();
    assert_eq!(b.len(), 1 + 35 + 24);
    assert_eq!(b[0], 0xA1);
}

#[test]
fn item_template_plain_via_codec_matches_legacy_bytes() {
    let mut legacy = NetworkMessage::new();
    write_item_template(&mut legacy, 0x1234, 1, false, false, false, false);
    let mut via_codec = NetworkMessage::new();
    codec().write_item_template(&mut via_codec, 0x1234, 1, false, false, false, false);
    assert_eq!(legacy.as_bytes(), via_codec.as_bytes());
    assert_eq!(legacy.as_bytes(), &[0x34, 0x12, 0xFF]);
}

#[test]
fn item_template_fluid_via_codec() {
    let mut m = NetworkMessage::new();
    codec().write_item_template(&mut m, 0x1234, 3, false, true, false, false);
    assert_eq!(m.as_bytes(), &[0x34, 0x12, 0xFF, 0x03]);
}

#[test]
fn outfit_looktype_via_codec() {
    let o = OutfitWire {
        look_type: 128,
        look_head: 1,
        look_body: 2,
        look_legs: 3,
        look_feet: 4,
        look_addons: 0,
        look_mount: 0,
        look_type_ex: 0,
    };
    let mut m = NetworkMessage::new();
    codec().write_outfit(&mut m, &o);
    assert_eq!(
        m.as_bytes(),
        &[128, 0, 1, 2, 3, 4, 0, 0, 0]
    );
}

#[test]
fn player_stats_packet_via_codec() {
    let stats = PlayerStatsWire {
        health: 100,
        max_health: 100,
        free_capacity: 40000,
        total_capacity: 40000,
        experience: 4200,
        level: 8,
        level_percent: 50,
        mana: 50,
        max_mana: 50,
        magic_level: 0,
        base_magic_level: 0,
        magic_level_percent: 0,
        soul: 100,
        stamina_minutes: 2520,
        base_speed_half: 110,
        regeneration_ticks_sec: 0,
        offline_training_time: 0,
    };
    let b = codec().encode_player_stats(&stats).as_bytes().to_vec();
    assert_eq!(b[0], 0xA0);
    assert_eq!(b.len(), 1 + 2 + 2 + 4 + 4 + 8 + 2 + 1 + 2 + 2 + 2 + 2 + 2 + 2 + 2 + 1 + 1 + 1 + 1 + 2 + 2 + 2 + 2 + 2 + 1);
}

#[test]
fn add_creature_known_header_via_codec() {
    let c = AddCreatureWire {
        id: 0x11223344,
        remove_known: 0,
        known: true,
        creature_type: 0,
        name: String::new(),
        health_percent: 100,
        direction: 2,
        outfit: OutfitWire::default(),
        light_level: 0,
        light_color: 0,
        step_speed: 220,
        skull: 0,
        party_shield: 0,
        guild_emblem: 0,
        speech_bubble: 0,
        helpers: 0,
        walkthrough_blocked: 1,
        access_player: false,
    };
    let mut legacy = NetworkMessage::new();
    write_add_creature(&mut legacy, &c);
    let mut via_codec = NetworkMessage::new();
    codec().write_add_creature(&mut via_codec, &c);
    assert_eq!(legacy.as_bytes(), via_codec.as_bytes());
}

#[test]
fn encode_add_tile_item_matches_deprecated_helper() {
    let pos = Position::new(10, 20, 7);
    let args = ItemTemplateArgs {
        client_id: 0x1234,
        count: 3,
        stackable: true,
        is_splash_or_fluid: false,
        is_animation: false,
        with_description: false,
    };
    let via_codec = codec().encode_add_tile_item(pos, 2, args, false).into_bytes();
    let via_legacy =
        Codec1098.encode_add_tile_item(pos, 2, args).into_bytes();
    assert_eq!(via_codec, via_legacy);
    assert_eq!(via_codec[0], 0x6A);
}

/// 1098 `sendContainer` (`0x6E`): cid + item + name + capacity + hasParent + unlocked + pagination
/// + `u16` size + `u16` firstIndex + count + items. Regression after routing through the codec.
#[test]
fn container_open_1098_layout() {
    let wire = ContainerOpenWire {
        cid: 3,
        header_item: ItemTemplateArgs {
            client_id: 0x0BBE,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        },
        name: "bag".to_string(),
        capacity: 8,
        has_parent: false,
        unlocked: true,
        pagination: false,
        total_size: 1,
        first_index: 0,
        items: vec![ItemTemplateArgs {
            client_id: 0x0C00,
            count: 5,
            stackable: true,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        }],
    };
    let b = codec().encode_container_open(&wire).into_bytes();
    assert_eq!(
        b,
        vec![
            0x6E, 3, // opcode + cid
            0xBE, 0x0B, 0xFF, // header item: clientId + MARK
            0x03, 0x00, b'b', b'a', b'g', // name
            8, // capacity
            0, // hasParent
            1, // unlocked
            0, // pagination
            1, 0, // total size
            0, 0, // first index
            1, // items to send
            0x00, 0x0C, 0xFF, 5, // child item: clientId + MARK + count
        ]
    );
}

/// 7.72 golden bytes (Phase A5). Reference: `gameserver/src/` ONLY — `protocolgame.cpp`,
/// `networkmessage.cpp`, `tools.cpp`. Every assertion cites the C++ field list it mirrors.
mod v772 {
    use super::*;

    fn codec() -> Codec {
        Codec::from_version(ProtocolVersion::V772).expect("772 codec")
    }

    /// `networkmessage.cpp` `addItem`: `u16 clientId` only for a plain non-stackable item — no MARK.
    #[test]
    fn item_template_plain_is_two_bytes_no_mark() {
        let mut m = NetworkMessage::new();
        codec().write_item_template(&mut m, 0x1234, 1, false, false, false, false);
        assert_eq!(m.as_bytes(), &[0x34, 0x12]);
    }

    /// Description / animation flags are ignored in 7.72 (still 2 bytes).
    #[test]
    fn item_template_ignores_animation_and_description() {
        let mut m = NetworkMessage::new();
        codec().write_item_template(&mut m, 0x1234, 1, false, false, true, true);
        assert_eq!(m.as_bytes(), &[0x34, 0x12]);
    }

    /// Stackable: `u16 clientId` + `u8 count`.
    #[test]
    fn item_template_stackable_writes_count() {
        let mut m = NetworkMessage::new();
        codec().write_item_template(&mut m, 0x1234, 7, true, false, false, false);
        assert_eq!(m.as_bytes(), &[0x34, 0x12, 0x07]);
    }

    /// Fluid: `u16 clientId` + `u8 getLiquidColor(count)`. `tools.cpp`: type 3 → 3, type 6 → 4.
    #[test]
    fn item_template_fluid_uses_getliquidcolor_not_fluidmap() {
        let mut m = NetworkMessage::new();
        codec().write_item_template(&mut m, 0x1234, 6, false, true, false, false);
        // getLiquidColor(6) == 4 (differs from 10.x FLUID_MAP[6] == 9).
        assert_eq!(m.as_bytes(), &[0x34, 0x12, 0x04]);
    }

    /// `item_template_wire_len` must stay in sync with `write_item_template`.
    #[test]
    fn item_template_wire_len_matches_write() {
        for &(cid, count, stack, splash) in &[
            (0x1234u16, 1u8, false, false),
            (0x1234u16, 7u8, true, false),
            (0x1234u16, 6u8, false, true),
        ] {
            let mut m = NetworkMessage::new();
            codec().write_item_template(&mut m, cid, count, stack, splash, false, false);
            assert_eq!(
                m.as_bytes().len(),
                codec().item_template_wire_len(cid, count, stack, splash, false, false)
            );
        }
    }

    /// `AddOutfit` lookType path: `u16 lookType` + head/body/legs/feet — no addons, no mount.
    #[test]
    fn outfit_looktype_no_addons_no_mount() {
        let o = OutfitWire {
            look_type: 128,
            look_head: 1,
            look_body: 2,
            look_legs: 3,
            look_feet: 4,
            look_addons: 5,
            look_mount: 9,
            look_type_ex: 0,
        };
        let mut m = NetworkMessage::new();
        codec().write_outfit(&mut m, &o);
        assert_eq!(m.as_bytes(), &[128, 0, 1, 2, 3, 4]);
    }

    /// `AddOutfit` lookType==0 path: `u16 0` + `u16 lookTypeEx` (`addItemId`).
    #[test]
    fn outfit_item_outfit_writes_looktypeex() {
        let o = OutfitWire {
            look_type: 0,
            look_type_ex: 0x0456,
            ..Default::default()
        };
        let mut m = NetworkMessage::new();
        codec().write_outfit(&mut m, &o);
        assert_eq!(m.as_bytes(), &[0x00, 0x00, 0x56, 0x04]);
    }

    /// `AddCreature` unknown header: `0x61` + removeId + id + name (no creature-type byte), health,
    /// direction, outfit, raw light, **full** step speed, skull, party shield. No emblem / 2nd
    /// type / bubble / MARK / helpers / walkthrough.
    #[test]
    fn add_creature_unknown_772_layout() {
        let c = AddCreatureWire {
            id: 0x11223344,
            remove_known: 0xAABBCCDD,
            known: false,
            creature_type: 1,
            name: "Rat".to_string(),
            health_percent: 80,
            direction: 2,
            outfit: OutfitWire {
                look_type: 21,
                look_head: 1,
                look_body: 2,
                look_legs: 3,
                look_feet: 4,
                ..Default::default()
            },
            light_level: 7,
            light_color: 215,
            step_speed: 220,
            skull: 0,
            party_shield: 0,
            guild_emblem: 0,
            speech_bubble: 0,
            helpers: 0,
            walkthrough_blocked: 1,
            access_player: false,
        };
        let mut m = NetworkMessage::new();
        codec().write_add_creature(&mut m, &c);
        assert_eq!(
            m.as_bytes(),
            &[
                0x61, 0x00, // 0x61
                0xDD, 0xCC, 0xBB, 0xAA, // removeId
                0x44, 0x33, 0x22, 0x11, // id
                0x03, 0x00, b'R', b'a', b't', // name (no creature-type byte before it)
                80,   // health %
                2,    // direction
                21, 0, 1, 2, 3, 4, // outfit (no addons / mount)
                7, 215, // light level + color (raw, no 0xFF substitution)
                220, 0, // full step speed (not halved)
                0,   // skull
                0,   // party shield
            ]
        );
        assert_eq!(
            m.as_bytes().len(),
            codec().add_creature_wire_len(&c),
            "add_creature_wire_len must match write_add_creature"
        );
    }

    /// `AddCreature` known header: `0x62` + id, then the common tail.
    #[test]
    fn add_creature_known_772_wire_len_matches() {
        let c = AddCreatureWire {
            id: 0x11223344,
            known: true,
            outfit: OutfitWire {
                look_type: 0,
                look_type_ex: 1234,
                ..Default::default()
            },
            step_speed: 300,
            ..Default::default()
        };
        let mut m = NetworkMessage::new();
        codec().write_add_creature(&mut m, &c);
        assert_eq!(m.as_bytes()[0], 0x62);
        assert_eq!(m.as_bytes().len(), codec().add_creature_wire_len(&c));
    }

    /// `AddPlayerStats` (`0xA0`): health/max u16, cap u16 (=free/100), exp u32, level u16 + %,
    /// mana/max u16, magic u8 + %, soul u8. 22 bytes after opcode.
    #[test]
    fn player_stats_772_layout() {
        let stats = PlayerStatsWire {
            health: 150,
            max_health: 150,
            free_capacity: 40000, // centi-oz → 400 on the wire
            total_capacity: 40000,
            experience: 4200,
            level: 8,
            level_percent: 50,
            mana: 35,
            max_mana: 35,
            magic_level: 3,
            base_magic_level: 3,
            magic_level_percent: 25,
            soul: 100,
            stamina_minutes: 2520,
            base_speed_half: 110,
            regeneration_ticks_sec: 0,
            offline_training_time: 0,
        };
        let b = codec().encode_player_stats(&stats).into_bytes();
        assert_eq!(
            b,
            vec![
                0xA0, //
                150, 0, // health
                150, 0, // max health
                0x90, 0x01, // capacity 400 (40000/100)
                0x68, 0x10, 0x00, 0x00, // experience 4200 u32
                8, 0, // level
                50,   // level %
                35, 0, // mana
                35, 0, // max mana
                3,  // magic level
                25, // magic level %
                100, // soul
            ]
        );
    }

    /// `AddPlayerStats` writes 0 for experience overflow.
    #[test]
    fn player_stats_772_experience_overflow_writes_zero() {
        let stats = PlayerStatsWire {
            health: 1,
            max_health: 1,
            free_capacity: 0,
            total_capacity: 0,
            experience: u32::MAX as u64, // >= u32::MAX - 1
            level: 1,
            level_percent: 0,
            mana: 0,
            max_mana: 0,
            magic_level: 0,
            base_magic_level: 0,
            magic_level_percent: 0,
            soul: 0,
            stamina_minutes: 0,
            base_speed_half: 0,
            regeneration_ticks_sec: 0,
            offline_training_time: 0,
        };
        let b = codec().encode_player_stats(&stats).into_bytes();
        // exp bytes are at offset 1+2+2+2 = 7..11
        assert_eq!(&b[7..11], &[0, 0, 0, 0]);
    }

    /// `AddPlayerSkills` (`0xA1`): 7 skills × (`u8` level + `u8`%) = 14 bytes after opcode.
    #[test]
    fn player_skills_772_layout() {
        let levels = [10u16, 11, 12, 13, 14, 15, 16];
        let percents = [1u8, 2, 3, 4, 5, 6, 7];
        let b = codec()
            .encode_player_skills(&PlayerSkillsWire {
                levels,
                bases: levels,
                percents,
                additional_levels: [0; 6],
                additional_bases: [0; 6],
            })
            .into_bytes();
        assert_eq!(
            b,
            vec![0xA1, 10, 1, 11, 2, 12, 3, 13, 4, 14, 5, 15, 6, 16, 7]
        );
    }

    /// Self-appear (`0x0A`): id + `u16` beat (0x32) + `u8` canReportBugs.
    #[test]
    fn self_appear_772_layout() {
        let b = codec().encode_self_appear_login(0x11223344).into_bytes();
        assert_eq!(
            b,
            vec![0x0A, 0x44, 0x33, 0x22, 0x11, 0x32, 0x00, 0x00]
        );
    }

    /// `sendAddContainerItem` (`0x70`): cid + item — no slot index (10.x adds `u16`).
    #[test]
    fn add_container_item_772_no_slot_index() {
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 5,
            stackable: true,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec().encode_add_container_item(3, 99, args).into_bytes();
        assert_eq!(b, vec![0x70, 3, 0x34, 0x12, 5]);
    }

    /// `sendUpdateContainerItem` (`0x71`): cid + `u8` slot + item.
    #[test]
    fn update_container_item_772_u8_slot() {
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec().encode_update_container_item(2, 6, args).into_bytes();
        assert_eq!(b, vec![0x71, 2, 6, 0x34, 0x12]);
    }

    /// `sendInventoryItem` (`0x78`): slot + item.
    #[test]
    fn inventory_item_772_layout() {
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec().encode_inventory_item(5, args).into_bytes();
        assert_eq!(b, vec![0x78, 5, 0x34, 0x12]);
    }

    /// `sendAddTileItem` (`0x6A`): position + item (no stackpos on 7.72).
    #[test]
    fn add_tile_item_772_no_stackpos() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 3,
            stackable: true,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec().encode_add_tile_item(pos, 2, args, false).into_bytes();
        assert_eq!(b, vec![0x6A, 0x02, 0x01, 0x04, 0x03, 0x05, 0x34, 0x12, 3]);
    }

    /// 772 ignores `otclient_stackpos` — OTCv8 772 does not read stackpos on `0x6A`.
    #[test]
    fn add_tile_item_772_ignores_otclient_stackpos_flag() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 3,
            stackable: true,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let without = codec()
            .encode_add_tile_item(pos, 2, args, false)
            .into_bytes();
        let with_flag = codec()
            .encode_add_tile_item(pos, 2, args, true)
            .into_bytes();
        assert_eq!(without, with_flag);
    }

    /// `sendAddCreature` non-self (`0x6A`): position + creature marker immediately (no stackpos).
    #[test]
    fn add_tile_creature_772_no_stackpos() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let wire = AddCreatureWire {
            id: 0x11223344,
            remove_known: 0,
            known: false,
            name: "Orc".to_string(),
            health_percent: 100,
            direction: 2,
            outfit: OutfitWire {
                look_type: 5,
                ..Default::default()
            },
            step_speed: 200,
            ..Default::default()
        };
        let b = codec()
            .encode_add_tile_creature(pos, 1, &wire, false)
            .into_bytes();
        assert_eq!(b[0], 0x6A);
        // opcode + position (5) → creature marker `0x0061`
        assert_eq!(b[6], 0x61);
        assert_eq!(b[7], 0x00);
        assert_eq!(
            b,
            codec()
                .encode_add_tile_creature(pos, 1, &wire, true)
                .into_bytes()
        );
    }

    /// `sendUpdateTileItem` (`0x6B`): position + `u8` stackpos + item.
    #[test]
    fn update_tile_item_772_layout() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec().encode_update_tile_item(pos, 2, args).into_bytes();
        assert_eq!(b, vec![0x6B, 0x02, 0x01, 0x04, 0x03, 0x05, 0x02, 0x34, 0x12]);
    }

    /// `RemoveTileThing` (`0x6C`): position + `u8` stackpos.
    #[test]
    fn remove_tile_thing_772_layout() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let b = codec().encode_remove_tile_thing(pos, 3).into_bytes();
        assert_eq!(b, vec![0x6C, 0x02, 0x01, 0x04, 0x03, 0x05, 0x03]);
    }

    /// `AddCreatureLight` (`0x8D`): id + raw level + color (no access-player `0xFF`).
    #[test]
    fn creature_light_772_layout() {
        let b = codec()
            .encode_creature_light(0x11223344, 7, 215, true)
            .into_bytes();
        assert_eq!(b, vec![0x8D, 0x44, 0x33, 0x22, 0x11, 7, 215]);
    }

    /// `sendCreatureTurn` (`0x6B`): position + stackpos + `u16 0x63` + id + direction. No `0xFFFF`
    /// by-id branch, no walkthrough byte (10.x only).
    #[test]
    fn creature_turn_772_layout() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let b = codec()
            .encode_creature_turn(0x11223344, 2, pos, 1, false)
            .into_bytes();
        assert_eq!(
            b,
            vec![0x6B, 0x02, 0x01, 0x04, 0x03, 0x05, 0x02, 0x63, 0x00, 0x44, 0x33, 0x22, 0x11, 1]
        );
    }

    /// `sendCancelWalk` (`0xB5`): direction.
    #[test]
    fn cancel_walk_772_layout() {
        let b = codec().encode_cancel_walk(3).into_bytes();
        assert_eq!(b, vec![0xB5, 3]);
    }

    /// `sendContainer` (`0x6E`): cid + item + name + `u8` capacity + `u8` hasParent + `u8` count +
    /// items. No unlocked / pagination / `u16` size / firstIndex (10.x additions).
    #[test]
    fn container_open_772_layout() {
        let wire = ContainerOpenWire {
            cid: 3,
            header_item: ItemTemplateArgs {
                client_id: 0x0BBE,
                count: 1,
                stackable: false,
                is_splash_or_fluid: false,
                is_animation: false,
                with_description: false,
            },
            name: "bag".to_string(),
            capacity: 8,
            has_parent: false,
            // 10.x-only fields are filled by core but must be ignored by the 772 codec.
            unlocked: true,
            pagination: true,
            total_size: 99,
            first_index: 7,
            items: vec![ItemTemplateArgs {
                client_id: 0x0C00,
                count: 5,
                stackable: true,
                is_splash_or_fluid: false,
                is_animation: false,
                with_description: false,
            }],
        };
        let b = codec().encode_container_open(&wire).into_bytes();
        assert_eq!(
            b,
            vec![
                0x6E, 3, // opcode + cid
                0xBE, 0x0B, // header item: clientId only (no MARK in 7.72)
                0x03, 0x00, b'b', b'a', b'g', // name
                8, // capacity
                0, // hasParent
                1, // items to send
                0x00, 0x0C, 5, // child: clientId + count (stackable, no MARK)
            ]
        );
    }

    /// 7.72 container window caps `count` at capacity (no pagination).
    #[test]
    fn container_open_772_caps_count_at_capacity() {
        let item = ItemTemplateArgs {
            client_id: 0x0C00,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let wire = ContainerOpenWire {
            cid: 0,
            header_item: item,
            name: String::new(),
            capacity: 2,
            has_parent: false,
            unlocked: false,
            pagination: false,
            total_size: 4,
            first_index: 0,
            items: vec![item, item, item, item],
        };
        let b = codec().encode_container_open(&wire).into_bytes();
        // capacity 2 → count byte = 2, then 2 item bodies (2 bytes each) = 4.
        // header: 0x6E + cid + clientId(2) + name len(2) = 6; + cap + hasParent + count = 9; + 4.
        let count_byte_idx = 1 + 1 + 2 + 2 + 1 + 1;
        assert_eq!(b[count_byte_idx], 2);
        assert_eq!(b.len(), count_byte_idx + 1 + 2 * 2);
    }

    /// 7.72 has no `sendBasicData` / by-id tile removal — encoders return empty (skipped by core).
    #[test]
    fn no_equivalent_packets_are_empty() {
        assert!(codec().encode_basic_data(true, 1234, 1).into_bytes().is_empty());
        assert!(codec()
            .encode_remove_tile_creature_by_id(42)
            .into_bytes()
            .is_empty());
    }
}
