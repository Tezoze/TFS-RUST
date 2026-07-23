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
    let pending = lower_npc(file).ok()?;
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
