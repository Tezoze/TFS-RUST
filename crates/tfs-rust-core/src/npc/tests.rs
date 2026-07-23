//! NPC-4 unit and fixture-driven dialogue tests.

use std::path::PathBuf;
use std::sync::Arc;

use tfs_rust_common::enums::Direction;
use tfs_rust_common::Position;
use tfs_rust_content::npc_import::{lower_npc, parse_npc_file};
use tfs_rust_content::npcs::{
    validate_pending_definitions, DialogueAction, DialoguePolicy, DialoguePredicate,
    DialogueProgram, DialogueRule, DialogueSituation, NpcDatabase, SourceSpan,
};

use super::events::{DialogueEvent, DialogueSituationKind, DialogueTrace, QueueOp};
use super::expr::{EvalContext, PlayerVocationKind};
use super::match_rule::match_dialogue_rule;
use super::words::{search_for_number, search_for_word};
use crate::creature::{CreatureKind, Npc, NpcActivity, NpcRuntimeState, Outfit};
use crate::formulas::NpcTuning;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::sim_harness::{ensure_walkable_tile, insert_player, minimal_creature_base, minimal_world, sim_hero_player};

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
