//! First game-protocol burst after `Player` is placed (`ProtocolGame::sendAddCreature` self branch + map).
// C++ reference: `src/protocolgame.cpp` `ProtocolGame::login` / `sendAddCreature`;
// corpus `sending.cc` `SendFullScreen` / `SendMapPoint`, `crplayer.cc` ctor / `TakeOver`.

use slotmap::Key;
use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_common::enums::{ConditionType, SkullType};
use tracing::warn;

use crate::creature::CreatureKind;
use crate::creature::LightInfo;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::walk::{WalkSpeedRole, wire_step_speed};
use crate::{Monster, Npc, Outfit, Player};

use tfs_rust_net::codec::ItemTemplateArgs;
use tfs_rust_net::creature_encode::{AddCreatureWire, OutfitWire};
use tfs_rust_net::creature_known::KnownCreatureTable;
use tfs_rust_net::map_description::{ItemStack, TileContent, send_map_description_stub};
use tfs_rust_net::outgoing::{send_extended_opcode, send_magic_effect, send_otcv8_features};
use tfs_rust_net::outgoing_extra::{
    send_enter_world, send_fight_modes, send_icons, send_icons_classic, send_inventory_slot_empty,
    send_otc_features_raw, send_pending_state_entered, send_unjustified_stats_stub,
    send_world_light,
};

/// `GameFeature` (`src/const.h`) — `ProtocolGame::sendFeatures` (OTCv8), not `sendOTCFeatures`.
const OTC_FEATURE_EXTENDED_OPCODE: u8 = 80;
const OTC_FEATURE_ITEM_TOOLTIP: u8 = 93;

fn skull_byte(s: SkullType) -> u8 {
    match s {
        SkullType::None => 0,
        SkullType::Yellow => 1,
        SkullType::Green => 2,
        SkullType::White => 3,
        SkullType::Red => 4,
        SkullType::Black => 5,
        SkullType::Orange => 6,
    }
}

fn outfit_to_wire(o: &Outfit) -> OutfitWire {
    OutfitWire {
        look_type: o.look_type.max(0) as u16,
        look_head: o.look_head.clamp(0, 255) as u8,
        look_body: o.look_body.clamp(0, 255) as u8,
        look_legs: o.look_legs.clamp(0, 255) as u8,
        look_feet: o.look_feet.clamp(0, 255) as u8,
        look_addons: o.look_addons.clamp(0, 255) as u8,
        look_mount: 0,
        look_type_ex: 0,
    }
}

/// Empty outfit for `CONDITION_INVISIBLE` — TFS `Outfit_t{}` / 772 `TOutfit::Invisible()`.
/// Used on AddCreature so floor-change map refresh does not reappear a full lookType.
fn outfit_wire_visible(base: &crate::creature::CreatureBase) -> OutfitWire {
    if base.is_invisible() {
        OutfitWire::default()
    } else {
        outfit_to_wire(&base.outfit)
    }
}

fn health_percent(cur: i32, max_hp: i32) -> u8 {
    if max_hp <= 0 {
        return 100;
    }
    ((cur.max(0) as u64 * 100) / max_hp as u64).min(100) as u8
}

/// Non-player creatures: use slot key index (low 32 bits of `KeyData::as_ffi`) as protocol id until a global id allocator exists.
fn non_player_wire_id(cid: CreatureId) -> u32 {
    (cid.data().as_ffi() & 0xFFFF_FFFF) as u32
}

/// C++ `Monster::setID` — assigns an auto-incrementing wire id to a newly-inserted
/// monster or npc (`monster.h:43-46`, `monster.cpp:18`). Must be called immediately
/// after `creatures.insert(CreatureKind::Monster(...))` / `Npc(...)`. The id starts
/// at `0x40000000` and never wraps or reuses, preventing wire-id collisions when
/// SlotMap slots are recycled (the root cause of the "dead dragon sprite shows for
/// respawned skeleton" bug — the client caches outfit/name by wire id).
pub(crate) fn assign_creature_wire_id(world: &mut GameWorld, cid: CreatureId) {
    let id = world.next_monster_wire_id;
    world.next_monster_wire_id = world.next_monster_wire_id.wrapping_add(1);
    match world.creatures.get_mut(cid) {
        Some(CreatureKind::Monster(m)) => m.wire_id = id,
        Some(CreatureKind::Npc(n)) => n.wire_id = id,
        _ => return,
    }
    world.creature_by_wire.insert(id, cid);
}

impl GameWorld {
    /// C++ `Monster::setID` — assigns an auto-incrementing wire id after insert.
    ///
    /// See [`assign_creature_wire_id`].
    pub fn assign_creature_wire_id(&mut self, cid: CreatureId) {
        assign_creature_wire_id(self, cid);
    }

    /// Drop `creature_by_wire` before `creatures.remove` on a failed place.
    pub(crate) fn unindex_creature_wire(&mut self, cid: CreatureId) {
        let wire = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) if m.wire_id != 0 => Some(m.wire_id),
            Some(CreatureKind::Npc(n)) if n.wire_id != 0 => Some(n.wire_id),
            _ => None,
        };
        if let Some(w) = wire {
            self.creature_by_wire.remove(&w);
        }
    }
}

/// Protocol creature id for move/turn packets (`protocolgame.cpp` `sendMoveCreature`).
/// Players use `guid`; monsters/npcs use the auto-incrementing `wire_id` assigned at
/// spawn (C++ `Monster::setID`, `monster.h:43-46`). Falls back to the SlotMap idx for
/// unassigned ids (test harness monsters that skip `assign_creature_wire_id`).
pub(crate) fn creature_wire_id(cid: CreatureId, kind: &CreatureKind) -> u32 {
    match kind {
        CreatureKind::Player(p) => p.guid,
        CreatureKind::Monster(m) => {
            if m.wire_id != 0 {
                m.wire_id
            } else {
                non_player_wire_id(cid)
            }
        }
        CreatureKind::Npc(n) => {
            if n.wire_id != 0 {
                n.wire_id
            } else {
                non_player_wire_id(cid)
            }
        }
    }
}

/// C++ `ProtocolGame::AddCreature` — `protocolgame.cpp` ~3206 (`getCreatureLight`, viewer `isAccessPlayer`).
/// Build `AddCreatureWire` for map description and tile appear packets.
pub(crate) fn build_add_creature_wire(
    world: &GameWorld,
    cid: CreatureId,
    viewer: CreatureId,
) -> AddCreatureWire {
    let viewer_access = world.player_is_access_player(viewer);
    match world.creatures.get(cid).map(|k| match k {
        CreatureKind::Player(_) => 0u8,
        CreatureKind::Monster(_) => 1,
        CreatureKind::Npc(_) => 2,
    }) {
        Some(0) => {
            let light = world.player_creature_light(cid);
            let skull = world.player_get_killing_mark(cid, viewer);
            let party_shield = world.player_get_party_mark(cid, viewer);
            let subject_guid = match world.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.guid,
                _ => return AddCreatureWire::default(),
            };
            let is_self = world
                .creatures
                .get(viewer)
                .and_then(|k| match k {
                    CreatureKind::Player(vp) => Some(vp.guid == subject_guid),
                    _ => None,
                })
                .unwrap_or(false);
            let Some(CreatureKind::Player(p)) = world.creatures.get(cid) else {
                return AddCreatureWire::default();
            };
            player_to_add_creature_wire(
                p,
                is_self,
                light,
                viewer_access,
                &world.mechanics,
                skull,
                party_shield,
            )
        }
        Some(1) => match world.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                monster_to_add_creature_wire(cid, m, &world.mechanics)
            }
            _ => AddCreatureWire::default(),
        },
        Some(2) => match world.creatures.get(cid) {
            Some(CreatureKind::Npc(n)) => npc_to_add_creature_wire(cid, n, &world.mechanics),
            _ => AddCreatureWire::default(),
        },
        _ => AddCreatureWire::default(),
    }
}

pub(crate) fn fill_player_wire(
    out: &mut AddCreatureWire,
    p: &Player,
    is_self: bool,
    light: LightInfo,
    viewer_is_access: bool,
    mech: &crate::formulas::Mechanics,
    skull: SkullType,
    party_shield: u8,
) {
    let hp = if !is_self && p.health_hidden {
        0
    } else {
        health_percent(p.base.health, p.base.max_health)
    };
    out.id = p.guid;
    out.remove_known = 0;
    out.known = false;
    out.uptodate = false;
    out.creature_type = 0;
    out.name.clear();
    out.name.push_str(&p.base.name);
    out.health_percent = hp;
    out.direction = p.base.direction as u8;
    out.outfit = if p.ghost_mode {
        OutfitWire::default()
    } else {
        outfit_wire_visible(&p.base)
    };
    out.light_level = light.level;
    out.light_color = light.color;
    out.step_speed = wire_step_speed(WalkSpeedRole::Player, &p.base, mech);
    out.skull = skull_byte(skull);
    out.party_shield = party_shield;
    out.guild_emblem = 0;
    out.speech_bubble = 0;
    out.helpers = 0;
    out.walkthrough_blocked = 1;
    out.access_player = viewer_is_access;
}

fn player_to_add_creature_wire(
    p: &Player,
    is_self: bool,
    light: LightInfo,
    viewer_is_access: bool,
    mech: &crate::formulas::Mechanics,
    skull: SkullType,
    party_shield: u8,
) -> AddCreatureWire {
    let mut out = AddCreatureWire::default();
    fill_player_wire(
        &mut out,
        p,
        is_self,
        light,
        viewer_is_access,
        mech,
        skull,
        party_shield,
    );
    out
}

pub(crate) fn fill_monster_wire(
    out: &mut AddCreatureWire,
    cid: CreatureId,
    m: &Monster,
    mech: &crate::formulas::Mechanics,
) {
    out.id = if m.wire_id != 0 {
        m.wire_id
    } else {
        non_player_wire_id(cid)
    };
    out.remove_known = 0;
    out.known = false;
    out.uptodate = false;
    out.creature_type = 1;
    out.name.clear();
    out.name.push_str(&m.base.name);
    out.health_percent = health_percent(m.base.health, m.base.max_health);
    out.direction = m.base.direction as u8;
    out.outfit = outfit_wire_visible(&m.base);
    out.light_level = 0;
    out.light_color = 0;
    out.step_speed = wire_step_speed(WalkSpeedRole::MonsterOrNpc, &m.base, mech);
    out.skull = skull_byte(m.base.skull);
    out.party_shield = 0;
    out.guild_emblem = 0;
    out.speech_bubble = 0;
    out.helpers = 0;
    out.walkthrough_blocked = 1;
    out.access_player = false;
}

fn monster_to_add_creature_wire(
    cid: CreatureId,
    m: &Monster,
    mech: &crate::formulas::Mechanics,
) -> AddCreatureWire {
    let mut out = AddCreatureWire::default();
    fill_monster_wire(&mut out, cid, m, mech);
    out
}

pub(crate) fn fill_npc_wire(
    out: &mut AddCreatureWire,
    cid: CreatureId,
    n: &Npc,
    mech: &crate::formulas::Mechanics,
) {
    out.id = if n.wire_id != 0 {
        n.wire_id
    } else {
        non_player_wire_id(cid)
    };
    out.remove_known = 0;
    out.known = false;
    out.uptodate = false;
    out.creature_type = 2;
    out.name.clear();
    out.name.push_str(&n.base.name);
    out.health_percent = health_percent(n.base.health, n.base.max_health);
    out.direction = n.base.direction as u8;
    out.outfit = outfit_wire_visible(&n.base);
    out.light_level = 0;
    out.light_color = 0;
    out.step_speed = wire_step_speed(WalkSpeedRole::MonsterOrNpc, &n.base, mech);
    out.skull = skull_byte(n.base.skull);
    out.party_shield = 0;
    out.guild_emblem = 0;
    out.speech_bubble = n.speech_bubble;
    out.helpers = 0;
    out.walkthrough_blocked = 1;
    out.access_player = false;
}

fn npc_to_add_creature_wire(
    cid: CreatureId,
    n: &Npc,
    mech: &crate::formulas::Mechanics,
) -> AddCreatureWire {
    let mut out = AddCreatureWire::default();
    fill_npc_wire(&mut out, cid, n, mech);
    out
}

pub(crate) fn item_stack_from_server_id(
    world: &GameWorld,
    iid: u16,
    count: u8,
) -> Option<ItemStack> {
    if iid == 0 {
        return None;
    }
    let cid = world.items_db.client_id_for_server(iid);
    if cid == 0 {
        return None;
    }
    let stackable = world.item_wire_has_count(iid);
    let splash_fluid = world.items_db.is_splash_or_fluid_for_server(iid);
    Some(ItemStack {
        client_id: cid,
        count,
        stackable,
        is_splash_or_fluid: splash_fluid && !stackable,
        is_animation: world.items_db.is_animation_for_server(iid),
    })
}

/// Invisible and ghost creatures are left off other clients' tiles
/// (`map_tile_content_into` / [`creature_hidden_from_map`]). The spectator stack
/// byte must skip them the same way, or `0x6D`/`0x6C`/`0x6B` hits the next object.
pub(crate) fn creature_omitted_from_other_clients(kind: &CreatureKind) -> bool {
    let invisible = kind
        .base()
        .active_conditions
        .iter()
        .any(|c| c.ctype == ConditionType::Invisible);
    let ghost = matches!(kind, CreatureKind::Player(p) if p.ghost_mode);
    ghost || invisible
}

pub(crate) fn creature_hidden_from_map(
    kind: &CreatureKind,
    ocid: CreatureId,
    self_cid: CreatureId,
) -> bool {
    if ocid == self_cid {
        return false;
    }
    creature_omitted_from_other_clients(kind)
}

/// Viewer-side map encode context. Built once per `SendFullScreen` / NotifyGo strip
/// so `self_wire` is not rebuilt for every viewport tile (`sending.cc` `SendMapPoint`).
pub(crate) struct MapDescribeCtx {
    pub(crate) self_cid: CreatureId,
    pub(crate) self_guid: u32,
    pub(crate) player_pos: Position,
    pub(crate) viewer_access: bool,
    pub(crate) cip_map_order: bool,
    pub(crate) self_wire: AddCreatureWire,
}

impl MapDescribeCtx {
    pub(crate) fn from_world(
        world: &GameWorld,
        self_cid: CreatureId,
        player_pos: Position,
    ) -> Option<Self> {
        let Some(CreatureKind::Player(_)) = world.creatures.get(self_cid) else {
            return None;
        };
        let viewer_access = world.player_is_access_player(self_cid);
        let self_light = world.player_creature_light(self_cid);
        let self_skull = world.player_get_killing_mark(self_cid, self_cid);
        let self_party_shield = world.player_get_party_mark(self_cid, self_cid);
        let Some(CreatureKind::Player(self_player)) = world.creatures.get(self_cid) else {
            return None;
        };
        let is_772 = !world.codec.caps().move_creature_self_packet;
        Some(Self {
            self_cid,
            self_guid: self_player.guid,
            player_pos,
            viewer_access,
            cip_map_order: is_772 && !self_player.is_otclient(),
            self_wire: player_to_add_creature_wire(
                self_player,
                true,
                self_light,
                viewer_access,
                &world.mechanics,
                self_skull,
                self_party_shield,
            ),
        })
    }
}

/// Fill `out` for one viewport tile. Returns true when the tile has wire content
/// (decompile `SendMapPoint` non-empty). Reuses `out` capacity across tiles.
pub(crate) fn map_tile_content_into(
    world: &GameWorld,
    ctx: &MapDescribeCtx,
    tx: i32,
    ty: i32,
    tz: i32,
    out: &mut TileContent,
) -> bool {
    out.clear();
    if tx < 0 || ty < 0 || !(0..=15).contains(&tz) {
        return false;
    }

    let px = ctx.player_pos.x as i32;
    let py = ctx.player_pos.y as i32;
    let pz = ctx.player_pos.z as i32;
    let pos = Position::new(tx as u16, ty as u16, tz as u8);
    let on_self = tx == px && ty == py && tz == pz;
    // Real 772 client stores tiles in Cip map-container order
    // (Bank → Bottom → Top → Creature → Low). Matching `GetObjectRNum` /
    // spectator `0x6D` stackpos — treating PRIORITY_LOW downs as Bottom leaves
    // creatures at the wrong index → bug0000017 MoveCreature assert.
    out.cip_map_order = ctx.cip_map_order;

    if let Some(tile) = world.map.get_tile(pos) {
        let body = tile.body();
        if let Some(gid) = body.ground
            && gid != 0
        {
            out.ground = item_stack_from_server_id(world, gid, 1);
        }
        for &item_id in body.top_items() {
            let Some(item) = world.items.get(item_id) else {
                continue;
            };
            let Some(stack) = item_stack_from_server_id(world, item.item_type, item.client_count())
            else {
                continue;
            };
            out.top_items.push(stack);
        }
        for &ocid in body.creatures() {
            let skip = match world.creatures.get(ocid) {
                Some(kind) => creature_hidden_from_map(kind, ocid, ctx.self_cid),
                None => true,
            };
            if skip {
                continue;
            }
            if ocid == ctx.self_cid {
                let slot = out.creature_slot();
                slot.copy_from(&ctx.self_wire);
                continue;
            }
            let skull = match world.creatures.get(ocid) {
                Some(CreatureKind::Player(_)) => world.player_get_killing_mark(ocid, ctx.self_cid),
                _ => SkullType::None,
            };
            let party_shield = match world.creatures.get(ocid) {
                Some(CreatureKind::Player(_)) => world.player_get_party_mark(ocid, ctx.self_cid),
                _ => 0,
            };
            let light = match world.creatures.get(ocid) {
                Some(CreatureKind::Player(_)) => world.player_creature_light(ocid),
                _ => LightInfo::default(),
            };
            match world.creatures.get(ocid) {
                Some(CreatureKind::Player(p)) => {
                    let slot = out.creature_slot();
                    fill_player_wire(
                        slot,
                        p,
                        p.guid == ctx.self_guid,
                        light,
                        ctx.viewer_access,
                        &world.mechanics,
                        skull,
                        party_shield,
                    );
                }
                Some(CreatureKind::Monster(m)) => {
                    let slot = out.creature_slot();
                    fill_monster_wire(slot, ocid, m, &world.mechanics);
                }
                Some(CreatureKind::Npc(n)) => {
                    let slot = out.creature_slot();
                    fill_npc_wire(slot, ocid, n, &world.mechanics);
                }
                None => {}
            }
        }
        for &item_id in body.down_items() {
            let Some(item) = world.items.get(item_id) else {
                continue;
            };
            let iid = item.item_type;
            if iid == 0 {
                continue;
            }
            let Some(itype) = world.items_db.items.get(&iid) else {
                continue;
            };
            let cid = itype.client_id;
            if cid == 0 {
                continue;
            }
            let stackable = world.item_wire_has_count(iid);
            let splash_fluid = (itype.is_splash() || itype.is_fluid_container()) && !stackable;
            let stack = ItemStack {
                client_id: cid,
                count: item.client_count(),
                stackable,
                is_splash_or_fluid: splash_fluid,
                is_animation: itype.is_animation(),
            };
            // Real 772: only PRIORITY_BOTTOM before creatures; LOW after.
            // TVP/OTC: all downs fold into bottom_items (emitted after creatures).
            if out.cip_map_order && itype.is_cip_priority_bottom() {
                out.bottom_items.push(stack);
            } else if out.cip_map_order {
                out.low_items.push(stack);
            } else {
                out.bottom_items.push(stack);
            }
        }
    }

    if on_self && !out.creatures.iter().any(|c| c.id == ctx.self_guid) {
        tracing::debug!(
            self_cid = ?ctx.self_cid,
            pos = ?pos,
            "map_tile_content: self not on tile, injecting self_wire (ghost check)"
        );
        let slot = out.creature_slot();
        slot.copy_from(&ctx.self_wire);
    }

    !out.is_blank()
}

/// Full `0x64` map around `center` from loaded OTBM tiles and creature indices.
fn build_initial_map_packet(
    world: &GameWorld,
    self_cid: CreatureId,
    center: Position,
    known: &mut KnownCreatureTable,
) -> Vec<u8> {
    if world.creatures.get(self_cid).is_none() {
        return send_map_description_stub(center, center).into_bytes();
    }

    let with_description = matches!(
        world.creatures.get(self_cid),
        Some(CreatureKind::Player(p)) if p.item_with_description()
    );

    let Some(ctx) = MapDescribeCtx::from_world(world, self_cid, center) else {
        return send_map_description_stub(center, center).into_bytes();
    };
    crate::map_point::encode_fullscreen(world, &ctx, known, with_description)
}

/// Enqueue the initial login burst for a freshly placed player, selecting the version-specific
/// sequence by the active wire codec. 1098 (`ProtocolGame::login` repo-root `src/protocolgame.cpp`)
/// sends the OTCv8/10.98 preamble; 772 (`gameserver/src/protocolgame.cpp` `login` + `sendAddCreature`
/// self branch) sends the lean 772 sequence. Mixing them desyncs the client (e.g. 1098's
/// pending-state `0x0A` collides with 772's self-appear opcode → black screen).
///
/// Queues only (`FinishSendData`). 772 `TPlayer` ctor / `TakeOver` (`crplayer.cc:197-209`,
/// `721-773`) never `SendAll`; that is `AdvanceGame` (`main.cc:455`). Callers must not flush.
pub fn enqueue_initial_login_packets(
    world: &mut GameWorld,
    conn_id: ConnId,
    creature_id: CreatureId,
) {
    match world.codec {
        // 800 uses the same login burst as 772. Icons width follows caps.
        tfs_rust_net::Codec::V772(_) | tfs_rust_net::Codec::V800(_) => {
            enqueue_initial_login_packets_classic(world, conn_id, creature_id)
        }
        tfs_rust_net::Codec::V1098(_) => {
            enqueue_initial_login_packets_1098(world, conn_id, creature_id)
        }
    }
}

/// 7.72 login burst — `gameserver/src/protocolgame.cpp` `ProtocolGame::login` (OTCv8 extended-opcode
/// preamble, OTClient only) + `sendAddCreature` self branch (~L1733):
/// `[0x32 ext-opcode init if OTClient]` → `0x0A` self-appear → `0x64` map → inventory (`0x78`/`0x79`)
/// → `0xA0` stats → `0xA1` skills → `0x82` world light → `0x8D` creature light → VIP → `0xA2` icons
/// → `0x83` `CONST_ME_TELEPORT` (TVP `Game::placeCreature` after `sendAddCreature`, `game.cpp:537-540`).
/// No `0x17`/`0x43`/`0x0F`/pending-state/enter-world/unjustified/basic-data/fight-modes (10.98-only).
fn enqueue_initial_login_packets_classic(
    world: &mut GameWorld,
    conn_id: ConnId,
    creature_id: CreatureId,
) {
    let (pid, pos, equipment_slots, vip_list, with_desc_inv, is_otclient) = {
        let Some(CreatureKind::Player(p)) = world.creatures.get(creature_id) else {
            return;
        };
        let is_otclient =
            p.otclient_v8 != 0 || p.operating_system >= tfs_rust_common::CLIENTOS_OTCLIENT_LINUX;
        (
            p.guid,
            p.base.position,
            p.equipment_slots,
            p.vip_list.clone(),
            p.item_with_description(),
            is_otclient,
        )
    };

    // OTCv8 extended-opcode preamble — `gameserver/src/protocolgame.cpp` `login` ~L142: only when the
    // client is OTClient. Raw `0x32 0x00 0x0000` (extended opcode 0, empty buffer).
    if is_otclient {
        world.enqueue_outgoing(conn_id, send_extended_opcode(0, "").into_bytes());
    }

    // Self-appear (`0x0A` in 772 via the codec) then the full map description (`0x64`).
    // 772 beat duration comes from `MechanicsProfile::beat_ms` (`data/formulas/772.lua` `beatMs`,
    // default 200 per `tibia-game-master/src/config.cc:102`) — the same value the game loop ticks at.
    let server_beat = world.mechanics.profile.beat_ms.min(u16::MAX as u32) as u16;
    world.enqueue_encoded(
        conn_id,
        world.codec.encode_self_appear_login(pid, server_beat),
    );

    let mut known = world.take_known_creatures_for_send(conn_id);
    world.reconcile_known_creatures_for_send(conn_id, &mut known);
    let map_bytes = build_initial_map_packet(world, creature_id, pos, &mut known);
    let map_0x64_len = map_bytes.len();
    world.commit_known_creatures_after_send(conn_id, &known);
    world.enqueue_outgoing(conn_id, map_bytes);

    // Inventory slots 1..=10 (`sendInventoryItem`: `0x78` item / `0x79` empty). 772 has no slot 11 store.
    for slot in 1u8..=10 {
        let idx = (slot - 1) as usize;
        if let Some(item_id) = equipment_slots[idx] {
            let Some(item) = world.items.get(item_id) else {
                world.enqueue_outgoing(conn_id, send_inventory_slot_empty(slot).into_bytes());
                continue;
            };
            let sid = item.item_type;
            let cid = world.items_db.client_id_for_server(sid);
            if cid == 0 {
                world.enqueue_outgoing(conn_id, send_inventory_slot_empty(slot).into_bytes());
                continue;
            }
            let cnt = world.item_wire_count(item);
            let stackable = world.item_wire_has_count(sid);
            let splash = world.items_db.is_splash_or_fluid_for_server(sid);
            let anim = world.items_db.is_animation_for_server(sid);
            world.enqueue_encoded(
                conn_id,
                world.codec.encode_inventory_item(
                    slot,
                    ItemTemplateArgs {
                        client_id: cid,
                        count: cnt,
                        stackable,
                        is_splash_or_fluid: splash && !stackable,
                        is_animation: anim,
                        with_description: with_desc_inv,
                    },
                ),
            );
        } else {
            world.enqueue_outgoing(conn_id, send_inventory_slot_empty(slot).into_bytes());
        }
    }

    // Stats (`0xA0`) + skills (`0xA1`) via the codec (772 widths).
    world.send_player_stats(creature_id);
    world.send_player_skills(creature_id);

    // World light (`0x82`) + this player's creature light (`0x8D`).
    let (wl_level, wl_color) = world.current_world_light();
    world.enqueue_outgoing(
        conn_id,
        send_world_light(wl_level, wl_color, false).into_bytes(),
    );
    let pl = world.player_creature_light(creature_id);
    world.enqueue_encoded(
        conn_id,
        world
            .codec
            .encode_creature_light(pid, pl.level, pl.color, false),
    );

    // VIP entries, then status icons (`0xA2` + `u8` in 772 — `sendIcons(uint16_t)` truncates to a byte).
    for e in &vip_list {
        let online = world.player_by_guid.contains_key(&e.player_id);
        let status = u8::from(online);
        let pkt = world.codec.encode_vip_entry(
            e.player_id,
            &e.name,
            &e.description,
            e.icon,
            e.notify,
            status,
        );
        world.enqueue_encoded(conn_id, pkt);
    }
    let icons_zero = if world.codec.caps().icons_u16 {
        send_icons(0)
    } else {
        send_icons_classic(0)
    };
    world.enqueue_outgoing(conn_id, icons_zero.into_bytes());
    // Overwrite the zeroed icons with live condition icons (mana shield / swords / …).
    // TFS `Player::sendIcons` after stored conditions are applied (`player.cpp:1142-1145`).
    world.send_player_icons(creature_id);
    // Re-announce haste / invis / light / outfit from persisted conditions.
    world.reapply_persisted_condition_effects(creature_id);

    // TVP `addMagicEffect(..., CONST_ME_TELEPORT)` after self `sendAddCreature` (`game.cpp:537-540`).
    world.enqueue_outgoing(conn_id, send_magic_effect(pos, 11).into_bytes());

    world.auto_open_containers_on_login(conn_id, creature_id);

    if map_0x64_len < 32 {
        warn!(
            conn_id = conn_id.0,
            player_id = pid,
            ?pos,
            map_0x64_len,
            "initial 772 0x64 map packet is very small — possible stub (creature/world not ready)"
        );
    }
}

/// 10.98 / OTCv8 login burst (repo-root `src/protocolgame.cpp` `ProtocolGame::login`).
/// C++ order: `0x17` → `0x0A` → `0x43` → `0x0F` → map → magic → inventory → stats → unjustified →
/// basic → skills → world light → creature light → VIP → basic → icons (then fight modes queued here).
fn enqueue_initial_login_packets_1098(
    world: &mut GameWorld,
    conn_id: ConnId,
    creature_id: CreatureId,
) {
    let (pid, pos, vocation_id, premium_ends_at, equipment_slots, vip_list, with_desc_inv) = {
        let Some(CreatureKind::Player(p)) = world.creatures.get(creature_id) else {
            return;
        };
        let with_desc_inv = p.item_with_description();
        (
            p.guid,
            p.base.position,
            p.vocation_id,
            p.premium_ends_at,
            p.equipment_slots,
            p.vip_list.clone(),
            with_desc_inv,
        )
    };
    let voc_client = world.vocations.client_id_u8(vocation_id);

    let free_premium = world.config.get_bool("freePremium").unwrap_or(false);
    let has_premium = world.player_is_premium(creature_id);
    let premium_packet_ends = if has_premium && !free_premium {
        premium_ends_at
    } else {
        0
    };

    let mut known = world.take_known_creatures_for_send(conn_id);
    world.reconcile_known_creatures_for_send(conn_id, &mut known);
    let map_bytes = build_initial_map_packet(world, creature_id, pos, &mut known);
    let map_0x64_len = map_bytes.len();
    world.commit_known_creatures_after_send(conn_id, &known);

    // `ProtocolGame::login` — OTCv8 (`src/protocolgame.cpp` ~168–178): `sendFeatures` + extended opcode init.
    world.enqueue_outgoing(
        conn_id,
        send_otcv8_features(&[
            (OTC_FEATURE_EXTENDED_OPCODE, true),
            (OTC_FEATURE_ITEM_TOOLTIP, true),
        ])
        .into_bytes(),
    );
    world.enqueue_outgoing(conn_id, send_extended_opcode(0, "").into_bytes());

    world.enqueue_encoded(conn_id, world.codec.encode_self_appear_login(pid, 0x32));
    world.enqueue_outgoing(conn_id, send_pending_state_entered().into_bytes());
    world.enqueue_outgoing(conn_id, send_otc_features_raw().into_bytes());
    world.enqueue_outgoing(conn_id, send_enter_world().into_bytes());
    world.enqueue_outgoing(conn_id, map_bytes);

    // `CONST_ME_TELEPORT` — `src/const.h`
    world.enqueue_outgoing(conn_id, send_magic_effect(pos, 11).into_bytes());

    for slot in 1u8..=11 {
        let idx = (slot - 1) as usize;
        if let Some(item_id) = equipment_slots[idx] {
            let Some(item) = world.items.get(item_id) else {
                world.enqueue_outgoing(conn_id, send_inventory_slot_empty(slot).into_bytes());
                continue;
            };
            let sid = item.item_type;
            let cid = world.items_db.client_id_for_server(sid);
            if cid == 0 {
                world.enqueue_outgoing(conn_id, send_inventory_slot_empty(slot).into_bytes());
                continue;
            }
            let cnt = world.item_wire_count(item);
            let stackable = world.item_wire_has_count(sid);
            let splash = world.items_db.is_splash_or_fluid_for_server(sid);
            let anim = world.items_db.is_animation_for_server(sid);
            world.enqueue_encoded(
                conn_id,
                world.codec.encode_inventory_item(
                    slot,
                    ItemTemplateArgs {
                        client_id: cid,
                        count: cnt,
                        stackable,
                        is_splash_or_fluid: splash && !stackable,
                        is_animation: anim,
                        with_description: with_desc_inv,
                    },
                ),
            );
        } else {
            world.enqueue_outgoing(conn_id, send_inventory_slot_empty(slot).into_bytes());
        }
    }

    // Use the centralized helper which correctly computes level_percent, capacity (1/100 oz),
    // offline training time (ms→minutes), etc. — matching C++ `Player::sendStats` (`player.cpp` ~882).
    world.send_player_stats(creature_id);
    world.enqueue_outgoing(conn_id, send_unjustified_stats_stub().into_bytes());
    world.enqueue_encoded(
        conn_id,
        world
            .codec
            .encode_basic_data(has_premium, premium_packet_ends, voc_client),
    );
    // OTClient 1098 + GameAdditionalSkills: skills 7–12 (critical / leech) still zero in
    // `send_player_skills` until those fields exist on `PlayerSkills`.
    world.send_player_skills(creature_id);

    let (wl_level, wl_color) = world.current_world_light();
    world.enqueue_outgoing(
        conn_id,
        send_world_light(wl_level, wl_color, false).into_bytes(),
    );
    let pl = world.player_creature_light(creature_id);
    world.enqueue_encoded(
        conn_id,
        world
            .codec
            .encode_creature_light(pid, pl.level, pl.color, false),
    );
    for e in &vip_list {
        let online = world.player_by_guid.contains_key(&e.player_id);
        let status = u8::from(online);
        let pkt = world.codec.encode_vip_entry(
            e.player_id,
            &e.name,
            &e.description,
            e.icon,
            e.notify,
            status,
        );
        world.enqueue_encoded(conn_id, pkt);
    }
    world.enqueue_encoded(
        conn_id,
        world
            .codec
            .encode_basic_data(has_premium, premium_packet_ends, voc_client),
    );
    world.enqueue_outgoing(conn_id, send_icons(0).into_bytes());
    world.send_player_icons(creature_id);
    world.reapply_persisted_condition_effects(creature_id);
    world.enqueue_outgoing(conn_id, send_fight_modes(1, 0, 0, 0).into_bytes());

    world.auto_open_containers_on_login(conn_id, creature_id);

    if map_0x64_len < 32 {
        warn!(
            conn_id = conn_id.0,
            player_id = pid,
            ?pos,
            map_0x64_len,
            "initial 0x64 map packet is very small — possible stub (creature/world not ready)"
        );
    }
}

#[cfg(test)]
mod map_creature_wire_tests {
    use super::*;
    use crate::creature::LightInfo;
    use crate::formulas::Mechanics;
    use crate::test_world::support::test_player;
    use tfs_rust_common::{Position, ProtocolVersion};

    #[test]
    fn self_map_wire_uses_creature_light_not_is_self_access() {
        let p = test_player("Test", Position::new(100, 100, 7));
        let mech = Mechanics::for_version(ProtocolVersion::V1098);
        let light = LightInfo {
            level: 7,
            color: 215,
        };
        let wire = player_to_add_creature_wire(&p, true, light, false, &mech, SkullType::None, 0);
        assert!(!wire.access_player);
        assert_eq!(wire.light_level, 7);
        assert_eq!(wire.light_color, 215);
    }

    #[test]
    fn gm_viewer_uses_access_player_wire_flag() {
        let p = test_player("GM", Position::new(100, 100, 7));
        let mech = Mechanics::for_version(ProtocolVersion::V1098);
        let wire = player_to_add_creature_wire(
            &p,
            true,
            LightInfo::default(),
            true,
            &mech,
            SkullType::None,
            0,
        );
        assert!(wire.access_player);
    }

    /// Login / map AddCreature must use player guid + GetSpeed (2×Go+80 on 772).
    /// Distinct from `0x8F` announce which previously sent SlotMap ffi ids.
    #[test]
    fn login_add_creature_wire_uses_guid_and_get_speed() {
        use crate::formulas::linear_go_effective_speed;
        use crate::walk::{WalkSpeedRole, wire_step_speed};

        let mut p = test_player("LoginSpeed", Position::new(100, 100, 7));
        p.guid = 42_001;
        p.base.speed = 220;
        p.base.base_speed = 220;
        p.base.var_speed = 0;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        let wire = player_to_add_creature_wire(
            &p,
            true,
            LightInfo::default(),
            false,
            &mech,
            SkullType::None,
            0,
        );

        assert_eq!(wire.id, p.guid);
        // Decompile `sending.cc` SendWord(GetSpeed()) = 2*220+80 = 520.
        assert_eq!(wire.step_speed, 520);
        assert_eq!(
            wire.step_speed,
            wire_step_speed(WalkSpeedRole::Player, &p.base, &mech)
        );
        assert_eq!(
            wire.step_speed,
            u16::try_from(linear_go_effective_speed(220)).expect("fits u16")
        );
    }

    /// Floor-change map refresh must keep empty outfit while Invisible is active.
    #[test]
    fn invisible_player_add_creature_uses_empty_outfit() {
        use crate::condition::{ActiveCondition, ConditionData};
        use tfs_rust_common::ConditionType;

        let mut p = test_player("Invis", Position::new(100, 100, 7));
        p.base.outfit.look_type = 128;
        p.base.active_conditions.push(ActiveCondition::new(
            0,
            0,
            ConditionType::Invisible,
            ConditionData::Generic { ticks: 0 },
            None,
        ));
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        let wire = player_to_add_creature_wire(
            &p,
            true,
            LightInfo::default(),
            false,
            &mech,
            SkullType::None,
            0,
        );
        assert_eq!(wire.outfit.look_type, 0);
        assert_eq!(wire.outfit.look_type_ex, 0);
    }

    #[test]
    fn ghost_player_add_creature_uses_empty_outfit() {
        let mut p = test_player("Ghost", Position::new(100, 100, 7));
        p.base.outfit.look_type = 128;
        p.ghost_mode = true;
        let mech = Mechanics::for_version(ProtocolVersion::V772);
        let wire = player_to_add_creature_wire(
            &p,
            true,
            LightInfo::default(),
            false,
            &mech,
            SkullType::None,
            0,
        );
        assert_eq!(wire.outfit.look_type, 0);
    }
}
