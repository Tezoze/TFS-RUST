//! NPC-4/5 unit and fixture-driven dialogue tests.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tfs_rust_common::enums::{ConditionType, Direction};
use tfs_rust_common::Position;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::npc_import::{lower_npc, parse_npc_file};
use tfs_rust_content::npcs::{
    validate_pending_definitions, DialogueAction, DialogueExpr, DialoguePolicy, DialoguePredicate,
    DialogueProgram, DialogueRule, DialogueSituation, ExprOp, NpcAppearance, NpcDatabase,
    NpcMovement, PendingNpcDefinition, SessionVar, SourceSpan,
};
use tfs_rust_content::otb::ItemType;

use super::events::{DialogueEvent, DialogueSituationKind, DialogueTrace, MutateOp, QueueOp};
use super::expr::{EvalContext, PlayerVocationKind};
use super::match_rule::match_dialogue_rule;
use super::words::{search_for_number, search_for_word};
use crate::condition::{ActiveCondition, ConditionData};
use crate::container::Container;
use crate::creature::{CreatureKind, Npc, NpcActivity, NpcRuntimeState, Outfit};
use crate::formulas::NpcTuning;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::item::Item;
use crate::player::inventory::money::{ITEM_GOLD_COIN, ITEM_PLATINUM_COIN};
use crate::sim_harness::{
    ensure_walkable_tile, insert_player, minimal_creature_base, minimal_world, sim_hero_player,
};
use slotmap::Key;

fn span() -> SourceSpan {
    SourceSpan {
        file: "t".into(),
        line: 1,
        column: 1,
        original_file: "t".into(),
        original_line: 1,
    }
}

fn load_quentin_db() -> Option<Arc<NpcDatabase>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/cipsoft-772/runtime/npc");
    if !root.exists() {
        return None;
    }
    let path = root.join("quentin.npc");
    let file = parse_npc_file(&root, &path).ok()?;
    let pending = lower_npc(file, None).ok()?;
    let db = validate_pending_definitions(vec![pending], None).ok()?;
    Some(Arc::new(db))
}

fn place_tiles(world: &mut GameWorld, center: Position) {
    for dx in -3..=3 {
        for dy in -3..=3 {
            ensure_walkable_tile(
                &mut world.map,
                Position::new(
                    (center.x as i32 + dx).max(0) as u16,
                    (center.y as i32 + dy).max(0) as u16,
                    center.z,
                ),
                100,
            );
        }
    }
}

/// Place an NPC instance from the database without spawn-slot / PZ placement gates.
fn insert_npc_from_db(world: &mut GameWorld, name: &str, pos: Position) -> CreatureId {
    let def = world
        .npcs_db
        .get_by_name(name)
        .cloned()
        .unwrap_or_else(|| panic!("missing npc def {name}"));
    ensure_walkable_tile(&mut world.map, pos, 100);
    let policy = def
        .dialogue
        .as_ref()
        .map(|d| d.policy)
        .unwrap_or(DialoguePolicy::QueuedSingleFocus);
    let mut base = minimal_creature_base();
    base.name = def.name.clone();
    base.position = pos;
    base.direction = Direction::South;
    base.health = def.health_max as i32;
    base.max_health = def.health_max as i32;
    base.outfit = Outfit {
        look_type: i32::from(def.appearance.look_type),
        look_head: i32::from(def.appearance.look_head),
        look_body: i32::from(def.appearance.look_body),
        look_legs: i32::from(def.appearance.look_legs),
        look_feet: i32::from(def.appearance.look_feet),
        look_addons: 0,
    };
    base.speed = i32::from(def.movement.speed);
    base.base_speed = i32::from(def.movement.speed);
    let cid = world.creatures.insert(CreatureKind::Npc(Npc {
        base,
        definition: def.id,
        speech_bubble: def.speech_bubble,
        wire_id: 0,
        runtime: NpcRuntimeState::at_home(pos, def.movement.radius, policy),
    }));
    world.map.register_creature_at(pos, cid);
    cid
}

fn eval_ctx<'a>(
    inv: &'a dyn Fn(i32) -> i32,
    quest: &'a dyn Fn(u32) -> i32,
    sk: &'a dyn Fn(i32) -> i32,
    sl: &'a dyn Fn(i32) -> i32,
    rng: &'a mut dyn FnMut(i32, i32) -> i32,
) -> EvalContext<'a> {
    EvalContext {
        topic: 0,
        price: 0,
        amount: 0,
        item_type: 0,
        data: 0,
        captures: [-1, -1],
        player_name: "Hero",
        player_hp: 100,
        player_level: 8,
        player_magic_level: 0,
        player_sex: 1,
        player_vocation: PlayerVocationKind::None,
        player_premium: false,
        player_promoted: false,
        player_pz_block: false,
        burning: 0,
        poison: 0,
        money: 0,
        inventory_count: inv,
        quest_value: quest,
        spell_known: sk,
        spell_level: sl,
        rng,
        game_hour: 10,
        game_minute: 0,
        world_pvp_enforced: false,
        world_non_pvp: false,
        tuning: NpcTuning::classic_772(),
    }
}

#[test]
fn word_match_respects_dollar_boundary() {
    assert!(search_for_word("hi$", "hi").is_some());
    assert!(search_for_word("hi$", "high").is_none());
    assert_eq!(search_for_number(1, "roll 99 please"), Some(5));
}

#[test]
fn select_bang_short_circuits() {
    let sp = span();
    let program = DialogueProgram {
        policy: DialoguePolicy::QueuedSingleFocus,
        rules: vec![
            DialogueRule {
                predicates: vec![
                    DialoguePredicate::Situation {
                        kind: DialogueSituation::Default,
                        span: sp.clone(),
                    },
                    DialoguePredicate::Words {
                        patterns: vec!["help".into()],
                        span: sp.clone(),
                    },
                    DialoguePredicate::Words {
                        patterns: vec!["me".into()],
                        span: sp.clone(),
                    },
                ],
                actions: vec![DialogueAction::Say {
                    text: "longer".into(),
                    span: sp.clone(),
                }],
                span: sp.clone(),
            },
            DialogueRule {
                predicates: vec![
                    DialoguePredicate::Situation {
                        kind: DialogueSituation::Default,
                        span: sp.clone(),
                    },
                    DialoguePredicate::Words {
                        patterns: vec!["help$".into()],
                        span: sp.clone(),
                    },
                    DialoguePredicate::Select { span: sp.clone() },
                ],
                actions: vec![DialogueAction::Say {
                    text: "bang".into(),
                    span: sp.clone(),
                }],
                span: sp,
            },
        ],
    };
    let inv = |_id: i32| 0i32;
    let quest = |_id: u32| 0i32;
    let sk = |_id: i32| 0i32;
    let sl = |_id: i32| 0i32;
    let mut rng = |a: i32, _b: i32| a;
    let mut ctx = eval_ctx(&inv, &quest, &sk, &sl, &mut rng);
    let m = match_dialogue_rule(&program, "help", DialogueSituationKind::Default, &mut ctx)
        .expect("match");
    assert_eq!(m.rule_index, 1);
}

#[test]
fn capture_applies_numeric_cap() {
    let sp = span();
    let program = DialogueProgram {
        policy: DialoguePolicy::QueuedSingleFocus,
        rules: vec![DialogueRule {
            predicates: vec![
                DialoguePredicate::Situation {
                    kind: DialogueSituation::Default,
                    span: sp.clone(),
                },
                DialoguePredicate::Words {
                    patterns: vec!["bet".into()],
                    span: sp.clone(),
                },
                DialoguePredicate::NumericCapture {
                    slot: 1,
                    span: sp.clone(),
                },
            ],
            actions: vec![DialogueAction::Nop { span: sp.clone() }],
            span: sp,
        }],
    };
    let inv = |_id: i32| 0i32;
    let quest = |_id: u32| 0i32;
    let sk = |_id: i32| 0i32;
    let sl = |_id: i32| 0i32;
    let mut rng = |a: i32, _b: i32| a;
    let mut ctx = eval_ctx(&inv, &quest, &sk, &sl, &mut rng);
    let m = match_dialogue_rule(&program, "bet 999", DialogueSituationKind::Default, &mut ctx)
        .unwrap();
    assert_eq!(m.captures.values[0], 500);
}

#[test]
fn greeting_farewell_trace() {
    let Some(db) = load_quentin_db() else {
        eprintln!("skip: reference quentin.npc missing");
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    world.round_nr = 100;
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);

    let npc = insert_npc_from_db(&mut world, "Quentin", home);
    let mut hero = sim_hero_player("Hero", Position::new(101, 100, 7));
    hero.base.name = "Hero".into();
    let p1 = insert_player(&mut world, hero);

    let mut trace = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "hi", &mut trace);

    assert!(trace
        .events
        .iter()
        .any(|e| matches!(e, DialogueEvent::Situation { name: "ADDRESS" })));
    assert!(trace.events.iter().any(|e| matches!(
        e,
        DialogueEvent::Say { text, .. } if text.contains("Welcome, adventurer Hero!")
    )));
    assert!(trace
        .events
        .iter()
        .any(|e| matches!(e, DialogueEvent::TurnTo { player } if *player == p1)));
    assert!(trace
        .events
        .iter()
        .any(|e| matches!(e, DialogueEvent::Set { var: "topic", value: 0 })));
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n))
            if n.runtime.activity == NpcActivity::Talking && n.runtime.focus == Some(p1)
    ));

    let mut trace2 = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "bye", &mut trace2);
    assert!(trace2
        .events
        .iter()
        .any(|e| matches!(e, DialogueEvent::Situation { name: "DEFAULT" })));
    assert!(trace2.events.iter().any(|e| matches!(
        e,
        DialogueEvent::Say { text, .. } if text == "Good bye, Hero!"
    )));
    // Say then Idle → Leaving until `ToDoChangeState` executes (`crnonpl.cc:1219-1222`).
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n)) if n.runtime.activity == NpcActivity::Leaving
    ));
    // Drain reply waits + ChangeState to Idle.
    world.server_ms = world.server_ms.saturating_add(20_000);
    crate::sim_harness::run_sim_tick(&mut world);
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n))
            if n.runtime.activity == NpcActivity::Idle && n.runtime.focus.is_none()
    ));
}

#[test]
fn two_player_busy_queue_and_timeout() {
    let Some(db) = load_quentin_db() else {
        eprintln!("skip: reference quentin.npc missing");
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    world.round_nr = 800;
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);

    let npc = insert_npc_from_db(&mut world, "Quentin", home);
    let mut h1 = sim_hero_player("Hero", Position::new(101, 100, 7));
    h1.base.name = "Hero".into();
    let p1 = insert_player(&mut world, h1);
    let mut h2 = sim_hero_player("Other", Position::new(102, 100, 7));
    h2.base.name = "Other".into();
    let p2 = insert_player(&mut world, h2);

    let mut t = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "hi", &mut t);
    let last = match world.creatures.get(npc) {
        Some(CreatureKind::Npc(n)) => n.runtime.last_talk_round,
        _ => panic!("npc"),
    };
    assert_eq!(last, 807, "TalkDelay/1000 + RoundNr after welcome reply");

    let mut t2 = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p2, "hi", &mut t2);
    assert!(t2
        .events
        .iter()
        .any(|e| matches!(e, DialogueEvent::Situation { name: "BUSY" })));
    assert!(t2.events.iter().any(|e| matches!(
        e,
        DialogueEvent::Queue {
            op: QueueOp::Push,
            player,
            ..
        } if *player == p2
    )));
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n)) if n.runtime.focus == Some(p1)
    ));

    world.round_nr = 837;
    let mut t3 = DialogueTrace::default();
    world.npc_idle_stimulus(npc, &mut t3);
    assert!(t3
        .events
        .iter()
        .any(|e| matches!(e, DialogueEvent::Situation { name: "VANISH" })));
    assert!(t3.events.iter().any(|e| matches!(
        e,
        DialogueEvent::Situation {
            name: "ADDRESSQUEUE"
        }
    )));
    assert!(t3.events.iter().any(|e| matches!(
        e,
        DialogueEvent::Say { text, .. } if text.contains("Welcome, adventurer Other!")
    )));
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n)) if n.runtime.focus == Some(p2)
    ));
}

#[test]
fn speech_candidates_same_floor_range() {
    let Some(db) = load_quentin_db() else {
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);
    ensure_walkable_tile(&mut world.map, Position::new(120, 100, 7), 100);
    ensure_walkable_tile(&mut world.map, Position::new(101, 100, 6), 100);

    let mut hero = sim_hero_player("Hero", home);
    hero.base.name = "Hero".into();
    let speaker = insert_player(&mut world, hero);
    let near = insert_npc_from_db(&mut world, "Quentin", Position::new(101, 100, 7));
    let _far = insert_npc_from_db(&mut world, "Quentin", Position::new(120, 100, 7));
    let _other_z = insert_npc_from_db(&mut world, "Quentin", Position::new(101, 100, 6));

    let cands = super::stimulus::collect_npc_speech_candidates(&world, speaker, home);
    assert!(cands.contains(&near));
    assert_eq!(cands.len(), 1, "far and other-floor excluded: {cands:?}");
}

#[test]
fn remove_player_releases_focus_via_vanish() {
    let Some(db) = load_quentin_db() else {
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    world.round_nr = 100;
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);

    let npc = insert_npc_from_db(&mut world, "Quentin", home);
    let mut hero = sim_hero_player("Hero", Position::new(101, 100, 7));
    hero.base.name = "Hero".into();
    let p1 = insert_player(&mut world, hero);

    let mut t = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "hi", &mut t);
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n)) if n.runtime.focus == Some(p1)
    ));

    world.remove_creature(p1);
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n))
            if n.runtime.focus.is_none() && n.runtime.activity == NpcActivity::Idle
    ));
}

// --- NPC-5: mutating actions ---

fn stackable_coin(server_id: u16) -> ItemType {
    let mut it = crate::sim_harness::pickup_item_type(server_id);
    it.flags |= 1 << 7; // FLAG_STACKABLE
    it
}

fn npc5_world() -> GameWorld {
    let mut world = minimal_world();
    let mut items = HashMap::new();
    items.insert(1987u16, crate::sim_harness::bag_item_type(1987));
    items.insert(ITEM_GOLD_COIN, stackable_coin(ITEM_GOLD_COIN));
    items.insert(ITEM_PLATINUM_COIN, stackable_coin(ITEM_PLATINUM_COIN));
    items.insert(2160u16, stackable_coin(2160));
    world.items_db = Arc::new(ItemDatabase {
        items,
        client_to_server: HashMap::new(),
    });
    world.mechanics.profile.npc = NpcTuning::classic_772();
    world
}

fn equip_backpack(world: &mut GameWorld, cid: CreatureId) {
    let bag = world.items.insert(Item::new_single(1987));
    world.container_registry.register(Container::new(bag, 20));
    if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
        p.equipment_slots[2] = Some(bag);
    }
}

fn give_gold(world: &mut GameWorld, cid: CreatureId, count: u16) {
    equip_backpack(world, cid);
    world
        .player_add_item_count(cid, ITEM_GOLD_COIN, u32::from(count))
        .expect("give gold");
}

fn register_named_npc(world: &mut GameWorld, pending: PendingNpcDefinition) {
    let db = validate_pending_definitions(vec![pending], None).expect("validate npc");
    world.npcs_db = Arc::new(db);
}

fn pending_with_rules(name: &str, rules: Vec<DialogueRule>) -> PendingNpcDefinition {
    PendingNpcDefinition {
        name: name.into(),
        source_file: "test.lua".into(),
        appearance: NpcAppearance::default(),
        health_max: 100,
        movement: NpcMovement::default(),
        speech_bubble: 0,
        sex: 1,
        race: 0,
        parameters: Default::default(),
        voices: Vec::new(),
        dialogue: Some(DialogueProgram {
            policy: DialoguePolicy::QueuedSingleFocus,
            rules,
        }),
        shop: None,
        custom_predicates: Vec::new(),
        custom_actions: Vec::new(),
        ..Default::default()
    }
}

#[test]
fn money_create_delete_and_insufficient() {
    let mut world = npc5_world();
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, 100);
    let cid = insert_player(&mut world, sim_hero_player("Hero", pos));
    equip_backpack(&mut world, cid);

    world.player_create_money(cid, 250).expect("create money");
    assert_eq!(world.player_count_money(cid), 250);
    assert_eq!(
        world.player_get_item_type_count(cid, ITEM_GOLD_COIN, -1),
        50
    );
    assert_eq!(
        world.player_get_item_type_count(cid, ITEM_PLATINUM_COIN, -1),
        2
    );

    world.player_delete_money(cid, 30).expect("delete 30");
    assert_eq!(world.player_count_money(cid), 220);

    let before_g = world.player_get_item_type_count(cid, ITEM_GOLD_COIN, -1);
    let before_p = world.player_get_item_type_count(cid, ITEM_PLATINUM_COIN, -1);
    assert!(world.player_delete_money(cid, 10_000).is_err());
    assert_eq!(
        world.player_get_item_type_count(cid, ITEM_GOLD_COIN, -1),
        before_g
    );
    assert_eq!(
        world.player_get_item_type_count(cid, ITEM_PLATINUM_COIN, -1),
        before_p
    );
}

#[test]
fn bank_change_delete_then_create_ordering() {
    let sp = span();
    let rules = vec![
        DialogueRule {
            predicates: vec![DialoguePredicate::Situation {
                kind: DialogueSituation::Address,
                span: sp.clone(),
            }],
            actions: vec![DialogueAction::Say {
                text: "Welcome %N!".into(),
                span: sp.clone(),
            }],
            span: sp.clone(),
        },
        DialogueRule {
            predicates: vec![
                DialoguePredicate::Situation {
                    kind: DialogueSituation::Default,
                    span: sp.clone(),
                },
                DialoguePredicate::Words {
                    patterns: vec!["change".into()],
                    span: sp.clone(),
                },
                DialoguePredicate::Expression {
                    expr: DialogueExpr::Count {
                        item: Box::new(DialogueExpr::Lit(2148)),
                    },
                    op: ExprOp::Ge,
                    rhs: DialogueExpr::Lit(200),
                    span: sp.clone(),
                },
            ],
            actions: vec![
                DialogueAction::Say {
                    text: "Here you are.".into(),
                    span: sp.clone(),
                },
                DialogueAction::Delete {
                    item: DialogueExpr::Lit(2148),
                    count: DialogueExpr::Lit(200),
                    span: sp.clone(),
                },
                DialogueAction::SetSession {
                    var: SessionVar::Amount,
                    expr: DialogueExpr::Lit(2),
                    span: sp.clone(),
                },
                DialogueAction::Create {
                    item: DialogueExpr::Lit(2152),
                    count: DialogueExpr::Session(SessionVar::Amount),
                    span: sp.clone(),
                },
            ],
            span: sp.clone(),
        },
    ];
    let mut world = npc5_world();
    register_named_npc(&mut world, pending_with_rules("Eva", rules));
    world.round_nr = 400;
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);
    let npc = insert_npc_from_db(&mut world, "Eva", home);
    let mut hero = sim_hero_player("Hero", Position::new(101, 100, 7));
    hero.base.name = "Hero".into();
    let p1 = insert_player(&mut world, hero);
    give_gold(&mut world, p1, 250);

    let mut t = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "hi", &mut t);
    let mut t2 = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "change", &mut t2);

    let mut saw_delete = false;
    let mut saw_create = false;
    for e in &t2.events {
        match e {
            DialogueEvent::Mutate {
                op: MutateOp::DeleteItem { item_id: 2148, count: 200 },
                ..
            } => {
                assert!(!saw_create, "delete before create");
                saw_delete = true;
            }
            DialogueEvent::Mutate {
                op: MutateOp::CreateItem { item_id: 2152, count: 2 },
                ..
            } => {
                assert!(saw_delete, "create after delete");
                saw_create = true;
            }
            _ => {}
        }
    }
    assert!(saw_delete && saw_create);
    assert_eq!(
        world.player_get_item_type_count(p1, ITEM_GOLD_COIN, -1),
        50
    );
    assert_eq!(
        world.player_get_item_type_count(p1, ITEM_PLATINUM_COIN, -1),
        2
    );
}

#[test]
fn quentin_heal_clears_poison_and_effect() {
    let Some(db) = load_quentin_db() else {
        eprintln!("skip: reference quentin.npc missing");
        return;
    };
    let mut world = npc5_world();
    world.npcs_db = db;
    world.round_nr = 200;
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);
    let npc = insert_npc_from_db(&mut world, "Quentin", home);
    let mut hero = sim_hero_player("Hero", Position::new(101, 100, 7));
    hero.base.name = "Hero".into();
    hero.base.active_conditions.push(ActiveCondition::new(
        0,
        0,
        ConditionType::Poison,
        ConditionData::Damage { total_rank: 5 },
        Some(5),
    ));
    let p1 = insert_player(&mut world, hero);

    let mut t = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "hello", &mut t);
    let mut t2 = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "heal", &mut t2);

    assert!(t2.events.iter().any(|e| matches!(
        e,
        DialogueEvent::Mutate {
            op: MutateOp::SetCondition {
                condition: "poison",
                value: 0
            },
            ..
        }
    )));
    assert!(t2.events.iter().any(|e| matches!(
        e,
        DialogueEvent::Mutate {
            op: MutateOp::Effect {
                effect_id: 14,
                on_npc: false
            },
            ..
        }
    )));
    let poison_left = match world.creatures.get(p1) {
        Some(CreatureKind::Player(p)) => p
            .base
            .active_conditions
            .iter()
            .any(|c| c.ctype == ConditionType::Poison),
        _ => true,
    };
    assert!(!poison_left);
}

#[test]
fn set_quest_value_persists_for_read_and_save() {
    let sp = span();
    let rules = vec![
        DialogueRule {
            predicates: vec![DialoguePredicate::Situation {
                kind: DialogueSituation::Address,
                span: sp.clone(),
            }],
            actions: vec![
                DialogueAction::SetQuestValue {
                    storage_id: 325,
                    value: DialogueExpr::Lit(1),
                    span: sp.clone(),
                },
                DialogueAction::Say {
                    text: "Marked.".into(),
                    span: sp.clone(),
                },
            ],
            span: sp.clone(),
        },
        DialogueRule {
            predicates: vec![
                DialoguePredicate::Situation {
                    kind: DialogueSituation::Default,
                    span: sp.clone(),
                },
                DialoguePredicate::Words {
                    patterns: vec!["check".into()],
                    span: sp.clone(),
                },
                DialoguePredicate::Expression {
                    expr: DialogueExpr::QuestValue { storage_id: 325 },
                    op: ExprOp::Eq,
                    rhs: DialogueExpr::Lit(1),
                    span: sp.clone(),
                },
            ],
            actions: vec![DialogueAction::Say {
                text: "Quest done.".into(),
                span: sp.clone(),
            }],
            span: sp.clone(),
        },
    ];
    let mut world = npc5_world();
    register_named_npc(&mut world, pending_with_rules("Guide", rules));
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);
    let npc = insert_npc_from_db(&mut world, "Guide", home);
    let mut hero = sim_hero_player("Hero", Position::new(101, 100, 7));
    hero.base.name = "Hero".into();
    let p1 = insert_player(&mut world, hero);

    let mut t = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "hi", &mut t);
    assert_eq!(world.player_get_storage(p1, 325), 1);
    assert!(t.events.iter().any(|e| matches!(
        e,
        DialogueEvent::Mutate {
            op: MutateOp::SetQuestValue { id: 325, value: 1 },
            ..
        }
    )));

    let mut t2 = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "check", &mut t2);
    assert!(t2.events.iter().any(|e| matches!(
        e,
        DialogueEvent::Say { text, .. } if text == "Quest done."
    )));

    let save = world.build_player_save_data(p1).expect("save");
    assert!(save.storage.iter().any(|(k, v)| *k == 325 && *v == 1));
}

#[test]
fn partial_failure_keeps_prior_mutations() {
    let sp = span();
    let rules = vec![DialogueRule {
        predicates: vec![DialoguePredicate::Situation {
            kind: DialogueSituation::Address,
            span: sp.clone(),
        }],
        actions: vec![
            DialogueAction::Delete {
                item: DialogueExpr::Lit(2148),
                count: DialogueExpr::Lit(10),
                span: sp.clone(),
            },
            DialogueAction::Delete {
                item: DialogueExpr::Lit(2152),
                count: DialogueExpr::Lit(1),
                span: sp.clone(),
            },
            DialogueAction::SetSession {
                var: SessionVar::Topic,
                expr: DialogueExpr::Lit(42),
                span: sp.clone(),
            },
        ],
        span: sp.clone(),
    }];
    let mut world = npc5_world();
    register_named_npc(&mut world, pending_with_rules("Clerk", rules));
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);
    let npc = insert_npc_from_db(&mut world, "Clerk", home);
    let mut hero = sim_hero_player("Hero", Position::new(101, 100, 7));
    hero.base.name = "Hero".into();
    let p1 = insert_player(&mut world, hero);
    give_gold(&mut world, p1, 50);

    let mut t = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "hi", &mut t);

    assert_eq!(
        world.player_get_item_type_count(p1, ITEM_GOLD_COIN, -1),
        40,
        "first delete must stick after failed second delete"
    );
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n)) if n.runtime.topic == 42
    ));
    let delete_events: Vec<_> = t
        .events
        .iter()
        .filter(|e| {
            matches!(
                e,
                DialogueEvent::Mutate {
                    op: MutateOp::DeleteItem { .. },
                    ..
                }
            )
        })
        .collect();
    assert_eq!(delete_events.len(), 1, "only successful delete is traced");
}

fn load_tom_db() -> Option<Arc<NpcDatabase>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/cipsoft-772/runtime/npc");
    if !root.exists() {
        return None;
    }
    let path = root.join("tom.npc");
    let file = parse_npc_file(&root, &path).ok()?;
    let pending = lower_npc(file, None).ok()?;
    let db = validate_pending_definitions(vec![pending], None).ok()?;
    Some(Arc::new(db))
}

#[test]
fn reply_todo_schedules_wait_talk_chain() {
    let Some(db) = load_quentin_db() else {
        eprintln!("skip: reference quentin.npc missing");
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    world.round_nr = 100;
    world.server_ms = 100_000;
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);

    let npc = insert_npc_from_db(&mut world, "Quentin", home);
    let mut hero = sim_hero_player("Hero", Position::new(101, 100, 7));
    hero.base.name = "Hero".into();
    let p1 = insert_player(&mut world, hero);

    let mut trace = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "hi", &mut trace);

    let base = world.creatures.get(npc).expect("npc").base();
    assert!(base.todo.locked, "ToDoStart must lock the batch");
    assert!(
        base.todo.has_wait() && base.todo.has_talk(),
        "greeting must enqueue Wait+Talk"
    );
    let talks: Vec<_> = base
        .todo
        .queue
        .iter()
        .filter_map(|a| match a {
            crate::creature_todo::CreatureAction::Talk { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        talks.iter().any(|t| t.contains("Welcome, adventurer Hero!")),
        "owned reply text queued: {talks:?}"
    );

    // Before first delay expires, Talk must not have executed yet (queue still has Talk).
    assert!(base.todo.has_talk());

    // Jump past initial 1000 ms wait and drain.
    world.server_ms = 101_000;
    crate::sim_harness::run_sim_tick(&mut world);
    // After first Wait+Talk, remaining trailing wait may still be present; speech text was queued.
}

#[test]
fn talking_keepalive_schedules_wait_2000() {
    let Some(db) = load_quentin_db() else {
        eprintln!("skip: reference quentin.npc missing");
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    world.round_nr = 100;
    world.server_ms = 50_000;
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);

    let npc = insert_npc_from_db(&mut world, "Quentin", home);
    let mut hero = sim_hero_player("Hero", Position::new(101, 100, 7));
    hero.base.name = "Hero".into();
    let p1 = insert_player(&mut world, hero);

    let mut t = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "hi", &mut t);
    // Clear greeting batch so IdleStimulus can run (`LockToDo` blocks otherwise).
    let _ = world.player_todo_clear(npc);
    if let Some(CreatureKind::Npc(n)) = world.creatures.get_mut(npc) {
        n.runtime.activity = NpcActivity::Talking;
        n.runtime.focus = Some(p1);
        n.runtime.last_talk_round = 100;
    }
    world.round_nr = 110; // still within 30-round window

    let mut t2 = DialogueTrace::default();
    world.npc_idle_stimulus(npc, &mut t2);
    let base = world.creatures.get(npc).expect("npc").base();
    assert!(
        matches!(
            base.todo.queue.front(),
            Some(crate::creature_todo::CreatureAction::Wait { deadline_ms })
                if *deadline_ms == world.server_ms.saturating_add(2000)
                    || *deadline_ms == 52_000
        ),
        "keepalive Wait(2000) expected, queue={:?}",
        base.todo.queue
    );
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n)) if n.runtime.activity == NpcActivity::Talking
    ));
}

#[test]
fn idle_sleep_when_no_players_nearby() {
    let Some(db) = load_quentin_db() else {
        eprintln!("skip: reference quentin.npc missing");
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);
    let npc = insert_npc_from_db(&mut world, "Quentin", home);
    let _ = world.player_todo_clear(npc);

    let mut t = DialogueTrace::default();
    world.npc_idle_stimulus(npc, &mut t);
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n)) if n.runtime.activity == NpcActivity::Sleeping
    ));
    assert!(t
        .events
        .iter()
        .any(|e| matches!(e, DialogueEvent::State { value: "sleeping" })));
}

#[test]
fn sleep_wakes_on_player_move() {
    let Some(db) = load_quentin_db() else {
        eprintln!("skip: reference quentin.npc missing");
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);
    let npc = insert_npc_from_db(&mut world, "Quentin", home);
    if let Some(CreatureKind::Npc(n)) = world.creatures.get_mut(npc) {
        n.runtime.activity = NpcActivity::Sleeping;
    }
    let mut hero = sim_hero_player("Hero", Position::new(101, 100, 7));
    hero.base.name = "Hero".into();
    let p1 = insert_player(&mut world, hero);

    let mut t = DialogueTrace::default();
    world.npc_creature_move_stimulus(npc, p1, false, &mut t);
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n)) if n.runtime.activity == NpcActivity::Idle
    ));
    assert!(t
        .events
        .iter()
        .any(|e| matches!(e, DialogueEvent::State { value: "idle" })));
}

#[test]
fn idle_roam_enqueues_go_with_fixed_rng() {
    let Some(db) = load_quentin_db() else {
        eprintln!("skip: reference quentin.npc missing");
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    world.seed_parity_rng(42);
    world.server_ms = 10_000;
    let home = Position::new(100, 100, 7);
    // Large enough pad for radius + sleep search.
    for dx in -12..=12 {
        for dy in -12..=12 {
            ensure_walkable_tile(
                &mut world.map,
                Position::new(
                    (home.x as i32 + dx).max(0) as u16,
                    (home.y as i32 + dy).max(0) as u16,
                    home.z,
                ),
                100,
            );
        }
    }
    let npc = insert_npc_from_db(&mut world, "Quentin", home);
    let _ = world.player_todo_clear(npc);
    // Player in sleep range so we roam instead of sleep.
    let mut hero = sim_hero_player("Hero", Position::new(105, 100, 7));
    hero.base.name = "Hero".into();
    let _p1 = insert_player(&mut world, hero);

    let mut t = DialogueTrace::default();
    world.npc_idle_stimulus(npc, &mut t);
    let base = world.creatures.get(npc).expect("npc").base();
    assert!(
        base.todo.has_go() || base.todo.has_wait(),
        "roam must enqueue Go+Wait or Wait-only on miss; queue={:?}",
        base.todo.queue
    );
    assert!(matches!(
        world.creatures.get(npc),
        Some(CreatureKind::Npc(n)) if n.runtime.activity == NpcActivity::Idle
    ));
}

#[test]
fn idle_roam_wait_holds_pause_before_re_idle() {
    // C++ `ToDoWait(2000)` after roam — future Wait must arm wakeup and must not
    // immediately re-enter IdleStimulus (`crnonpl.cc:1797-1799`, `cract.cc:795-801`).
    // Exercise Wait alone: synthetic tiles often reject `Go`, and `on_walk_step_rejected`
    // clears the trailing Wait (`walk/mod.rs` ToDoClear).
    let Some(db) = load_quentin_db() else {
        eprintln!("skip: reference quentin.npc missing");
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    world.server_ms = 10_000;
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);
    let npc = insert_npc_from_db(&mut world, "Quentin", home);
    let _ = world.player_todo_clear(npc);

    let delay = u64::from(world.mechanics.profile.npc.idle_roam_delay_ms);
    assert!(world.enqueue_creature_wait(npc, delay));
    world.todo_start_from_action(npc, 1);

    world.run_monster_todo_execute(npc);

    let base = world.creatures.get(npc).unwrap().base();
    assert!(
        base.todo.is_empty(),
        "Wait must drain; arms wakeup instead of staying queued"
    );
    let wakeup = base.next_wakeup.expect("roam Wait must arm wakeup");
    assert!(
        wakeup >= 10_000 + delay,
        "roam pause Wait({delay}) expected; wakeup={wakeup} server_ms={}",
        world.server_ms
    );
    assert!(
        base.todo.locked,
        "LockToDo must stay set through the roam pause"
    );

    // Locked + future wakeup must block immediate re-roam via IdleStimulus.
    world.idle_stimulus(npc);
    assert!(
        world.creatures.get(npc).unwrap().base().todo.is_empty(),
        "future Wait must block immediate re-roam"
    );
}

#[test]
fn multi_reply_delay_accounting_tom_job() {
    let Some(db) = load_tom_db() else {
        eprintln!("skip: reference tom.npc missing");
        return;
    };
    let mut world = minimal_world();
    world.npcs_db = db;
    world.mechanics.profile.npc = NpcTuning::classic_772();
    world.round_nr = 700;
    world.server_ms = 700_000;
    let home = Position::new(100, 100, 7);
    place_tiles(&mut world, home);
    let npc = insert_npc_from_db(&mut world, "Tom", home);
    let mut hero = sim_hero_player("Hero", Position::new(101, 100, 7));
    hero.base.name = "Hero".into();
    let p1 = insert_player(&mut world, hero);

    let mut t = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "hi", &mut t);
    let _ = world.player_todo_clear(npc);

    let mut t2 = DialogueTrace::default();
    world.npc_talk_stimulus(npc, p1, "job", &mut t2);

    let say_delays: Vec<u32> = t2
        .events
        .iter()
        .filter_map(|e| match e {
            DialogueEvent::Say { delay_ms, .. } => Some(*delay_ms),
            _ => None,
        })
        .collect();
    // multi_reply_timing.json: first job reply 1000, second 9400 (lens-dependent).
    assert!(
        say_delays.len() >= 2,
        "job rule must emit two REPLY delays, got {say_delays:?}"
    );
    assert_eq!(say_delays[0], 1000);
    assert_eq!(
        say_delays[1], 9400,
        "second reply TalkDelay after first length factor"
    );

    let base = world.creatures.get(npc).expect("npc").base();
    let waits: Vec<u64> = base
        .todo
        .queue
        .iter()
        .filter_map(|a| match a {
            crate::creature_todo::CreatureAction::Wait { deadline_ms } => {
                Some(deadline_ms.saturating_sub(700_000))
            }
            _ => None,
        })
        .collect();
    assert!(
        waits.contains(&1000) && waits.contains(&9400),
        "ToDo waits must match reply delays; waits={waits:?}"
    );
    // Final trailing wait 17600 from fixture.
    assert!(
        waits.contains(&17_600),
        "trailing Wait(17600) expected; waits={waits:?}"
    );
}

#[test]
fn custom_predicate_host_selects_rule() {
    use super::match_rule::{match_dialogue_rule_with_custom, CustomPredicateHost};
    use tfs_rust_content::npcs::NpcCallbackId;

    struct AlwaysTrue;
    impl CustomPredicateHost for AlwaysTrue {
        fn eval_custom(&mut self, _id: NpcCallbackId) -> bool {
            true
        }
    }
    struct AlwaysFalse;
    impl CustomPredicateHost for AlwaysFalse {
        fn eval_custom(&mut self, _id: NpcCallbackId) -> bool {
            false
        }
    }

    let sp = span();
    let program = DialogueProgram {
        policy: DialoguePolicy::QueuedSingleFocus,
        rules: vec![DialogueRule {
            predicates: vec![
                DialoguePredicate::Situation {
                    kind: DialogueSituation::Default,
                    span: sp.clone(),
                },
                DialoguePredicate::Custom {
                    callback_id: NpcCallbackId(1),
                    name: "ok".into(),
                    span: sp.clone(),
                },
            ],
            actions: vec![DialogueAction::Nop { span: sp.clone() }],
            span: sp,
        }],
    };
    let mut money = 0i32;
    let mut rng = |lo: i32, hi: i32| lo.max(hi);
    let inv = |_id: i32| 0i32;
    let quest = |_id: u32| -1i32;
    let spell_k = |_id: i32| 0i32;
    let spell_l = |_id: i32| 0i32;
    let mut ctx = EvalContext {
        topic: 0,
        price: 0,
        amount: 0,
        item_type: 0,
        data: 0,
        captures: [-1, -1],
        player_name: "Hero",
        player_hp: 100,
        player_level: 8,
        player_magic_level: 0,
        player_sex: 1,
        player_vocation: PlayerVocationKind::None,
        player_premium: false,
        player_promoted: false,
        player_pz_block: false,
        burning: 0,
        poison: 0,
        money: 0,
        inventory_count: &inv,
        quest_value: &quest,
        spell_known: &spell_k,
        spell_level: &spell_l,
        rng: &mut rng,
        game_hour: 12,
        game_minute: 0,
        world_pvp_enforced: false,
        world_non_pvp: false,
        tuning: NpcTuning::classic_772(),
    };
    let _ = money;
    assert!(match_dialogue_rule_with_custom(
        &program,
        "hi",
        DialogueSituationKind::Default,
        &mut ctx,
        &mut AlwaysTrue,
    )
    .is_some());
    assert!(match_dialogue_rule_with_custom(
        &program,
        "hi",
        DialogueSituationKind::Default,
        &mut ctx,
        &mut AlwaysFalse,
    )
    .is_none());
}

#[test]
fn custom_action_host_records_mutate() {
    use super::actions::NpcActionHost;
    use super::react::{apply_dialogue_plan, ReactMeta};
    use tfs_rust_content::npcs::NpcCallbackId;

    struct StubHost {
        called: bool,
    }
    impl NpcActionHost for StubHost {
        fn create_item(&mut self, _: CreatureId, _: i32, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn delete_item(&mut self, _: CreatureId, _: i32, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn create_money(&mut self, _: CreatureId, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn delete_money(&mut self, _: CreatureId, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn set_hp(&mut self, _: CreatureId, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn set_poison(&mut self, _: CreatureId, _: i32, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn set_burning(&mut self, _: CreatureId, _: i32, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn effect_me(&mut self, _: CreatureId, _: u16) -> Result<(), String> {
            Ok(())
        }
        fn effect_opp(&mut self, _: CreatureId, _: u16) -> Result<(), String> {
            Ok(())
        }
        fn set_quest_value(&mut self, _: CreatureId, _: u32, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn set_profession(&mut self, _: CreatureId, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn teach_spell(&mut self, _: CreatureId, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn summon(&mut self, _: CreatureId, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn teleport(&mut self, _: CreatureId, _: i32, _: i32, _: i32) -> Result<(), String> {
            Ok(())
        }
        fn set_start_position(
            &mut self,
            _: CreatureId,
            _: CreatureId,
            _: Option<(i32, i32, i32)>,
        ) -> Result<(i32, i32, i32), String> {
            Ok((0, 0, 0))
        }
        fn invoke_custom_action(
            &mut self,
            _: CreatureId,
            _: CreatureId,
            _: NpcCallbackId,
        ) -> Result<(), String> {
            self.called = true;
            Ok(())
        }
    }

    let sp = span();
    let program = DialogueProgram {
        policy: DialoguePolicy::QueuedSingleFocus,
        rules: vec![DialogueRule {
            predicates: vec![DialoguePredicate::Situation {
                kind: DialogueSituation::Default,
                span: sp.clone(),
            }],
            actions: vec![DialogueAction::Custom {
                callback_id: NpcCallbackId(42),
                name: "x".into(),
                span: sp.clone(),
            }],
            span: sp,
        }],
    };
    let inv = |_id: i32| 0i32;
    let quest = |_id: u32| -1i32;
    let spell_k = |_id: i32| 0i32;
    let spell_l = |_id: i32| 0i32;
    let mut rng = |lo: i32, hi: i32| lo;
    let mut ctx = EvalContext {
        topic: 0,
        price: 0,
        amount: 0,
        item_type: 0,
        data: 0,
        captures: [-1, -1],
        player_name: "Hero",
        player_hp: 100,
        player_level: 8,
        player_magic_level: 0,
        player_sex: 1,
        player_vocation: PlayerVocationKind::None,
        player_premium: false,
        player_promoted: false,
        player_pz_block: false,
        burning: 0,
        poison: 0,
        money: 0,
        inventory_count: &inv,
        quest_value: &quest,
        spell_known: &spell_k,
        spell_level: &spell_l,
        rng: &mut rng,
        game_hour: 12,
        game_minute: 0,
        world_pvp_enforced: false,
        world_non_pvp: false,
        tuning: NpcTuning::classic_772(),
    };
    let mut host = StubHost { called: false };
    let mut trace = DialogueTrace::default();
    // Opaque ids unused by StubHost — any bits fine.
    let dummy = CreatureId::from(slotmap::KeyData::from_ffi(1));
    let meta = ReactMeta {
        npc_id: dummy,
        npc_name: "X",
    };
    let matched = super::match_rule::RuleMatch {
        rule_index: 0,
        captures: Default::default(),
    };
    apply_dialogue_plan(
        &program,
        matched,
        DialogueSituationKind::Default,
        dummy,
        "",
        &mut ctx,
        NpcTuning::classic_772(),
        &mut host,
        &meta,
        &mut trace,
    );
    assert!(host.called);
    assert!(trace.events.iter().any(|e| matches!(
        e,
        DialogueEvent::Mutate {
            op: MutateOp::CustomAction,
            ..
        }
    )));
}

#[test]
fn npc_immune_to_combat_damage_and_conditions() {
    use crate::combat::{apply_condition, CombatDamage, CombatParams};
    use crate::sim_harness::insert_npc;
    use tfs_rust_common::enums::CombatType;

    let mut world = minimal_world();
    let pos = Position::new(100, 100, 7);
    let npc = insert_npc(&mut world, "Cipfried", pos, 100);
    let hp_before = world.creatures.get(npc).unwrap().base().health;

    let applied = world.combat_execute_with_stimulus(
        None,
        npc,
        &CombatDamage {
            primary: (CombatType::Fire, -50),
            secondary: (CombatType::Undefined, 0),
        },
        &CombatParams::default(),
    );
    assert!(!applied, "NPC must not take spell/AoE HP damage");
    assert_eq!(
        world.creatures.get(npc).unwrap().base().health,
        hp_before,
        "NPC HP must be unchanged after fire damage"
    );

    apply_condition(
        &mut world.creatures,
        npc,
        ActiveCondition {
            id: 1,
            sub_id: 0,
            ctype: ConditionType::Poison,
            data: ConditionData::Damage { total_rank: 40 },
            timer_rounds_left: None,
        },
    );
    assert!(
        world
            .creatures
            .get(npc)
            .unwrap()
            .base()
            .active_conditions
            .is_empty(),
        "NPC must reject combat conditions (TFS isImmune when !attackable)"
    );
}
