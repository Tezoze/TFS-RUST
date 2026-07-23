//! Apply a matched rule's actions into a plan + session mutations (NPC-4 non-mutating core).

use tfs_rust_content::npcs::{DialogueAction, DialogueProgram, SessionVar};

use super::events::{DialogueEvent, DialogueSituationKind, DialogueTrace, TodoOp};
use super::expr::{format_npc_response, EvalContext};
use super::match_rule::{MatchCaptures, RuleMatch};
use crate::formulas::NpcTuning;
use crate::ids::CreatureId;

/// Planned NPC reply with absolute delay from reaction start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedReply {
    pub text: String,
    pub delay_ms: u32,
    pub byte_len: usize,
}

/// Result of running dialogue actions for one reaction.
#[derive(Debug, Clone, Default)]
pub struct DialoguePlan {
    pub replies: Vec<PlannedReply>,
    pub queue_player: bool,
    pub go_idle: bool,
    pub topic: Option<i32>,
    pub price: Option<i32>,
    pub amount: Option<i32>,
    pub item_type: Option<i32>,
    pub data: Option<i32>,
    pub start_todo: bool,
    pub final_talk_delay_ms: u32,
    pub deferred: Vec<&'static str>,
}

/// Execute matched rule actions (and `*` repeats) into [`DialoguePlan`] + trace events.
///
/// C++ action loop — `crnonpl.cc:1085-1291`. Mutating world actions are deferred (NPC-5).
pub fn apply_dialogue_plan(
    program: &DialogueProgram,
    matched: RuleMatch,
    situation: DialogueSituationKind,
    player: CreatureId,
    text: &str,
    ctx: &mut EvalContext<'_>,
    tuning: NpcTuning,
    trace: &mut DialogueTrace,
) -> DialoguePlan {
    let _ = (player, text);
    ctx.captures = matched.captures.values;

    if situation != DialogueSituationKind::Busy {
        ctx.topic = 0;
        trace.push(DialogueEvent::Set {
            var: "topic",
            value: 0,
        });
    }

    let mut plan = DialoguePlan {
        topic: (situation != DialogueSituationKind::Busy).then_some(0),
        ..DialoguePlan::default()
    };
    let mut talk_delay = tuning.reply_initial_delay_ms;
    let mut rule_index = matched.rule_index;

    loop {
        let Some(rule) = program.rules.get(rule_index) else {
            break;
        };
        let mut repeat = false;
        for action in &rule.actions {
            match action {
                DialogueAction::RepeatPrevious { .. } => {
                    if rule_index > 0 {
                        rule_index -= 1;
                        repeat = true;
                    }
                    break;
                }
                DialogueAction::Say { text: template, .. } => {
                    let response = format_npc_response(template, ctx);
                    let byte_len = response.len();
                    plan.replies.push(PlannedReply {
                        text: response.clone(),
                        delay_ms: talk_delay,
                        byte_len,
                    });
                    trace.push(DialogueEvent::Say {
                        text: response,
                        delay_ms: talk_delay,
                        byte_len,
                    });
                    talk_delay = talk_delay.saturating_add(
                        tuning.reply_base_delay_ms
                            + (byte_len as u32 / 2).saturating_mul(tuning.reply_byte_factor_ms),
                    );
                    plan.start_todo = true;
                }
                DialogueAction::SetSession { var, expr, .. } => {
                    let value = super::expr::eval_expr(expr, ctx);
                    apply_session_set(*var, value, &mut plan, ctx, trace);
                }
                DialogueAction::Idle { .. } => {
                    if !plan.start_todo {
                        plan.go_idle = true;
                        // ADDRESSQUEUE idle without prior talk is an error in C++; still mark idle.
                    } else {
                        // After queued speech, Idle becomes Leaving then ToDoChangeState — NPC-6.
                        plan.go_idle = true;
                    }
                    plan.start_todo = true;
                }
                DialogueAction::Queue { .. } => {
                    if situation == DialogueSituationKind::Busy {
                        plan.queue_player = true;
                    }
                }
                DialogueAction::Nop { .. } => {}
                DialogueAction::Create { .. } => defer(&mut plan, trace, "create"),
                DialogueAction::Delete { .. } => defer(&mut plan, trace, "delete"),
                DialogueAction::CreateMoney { .. } => defer(&mut plan, trace, "createMoney"),
                DialogueAction::DeleteMoney { .. } => defer(&mut plan, trace, "deleteMoney"),
                DialogueAction::SetHp { .. } => defer(&mut plan, trace, "hp"),
                DialogueAction::Burning { .. } => defer(&mut plan, trace, "burning"),
                DialogueAction::Poison { .. } => defer(&mut plan, trace, "poison"),
                DialogueAction::EffectMe { .. } => defer(&mut plan, trace, "effectMe"),
                DialogueAction::EffectOpp { .. } => defer(&mut plan, trace, "effectOpp"),
                DialogueAction::SetQuestValue { .. } => defer(&mut plan, trace, "setQuestValue"),
                DialogueAction::Profession { .. } => defer(&mut plan, trace, "profession"),
                DialogueAction::TeachSpell { .. } => defer(&mut plan, trace, "teachSpell"),
                DialogueAction::Summon { .. } => defer(&mut plan, trace, "summon"),
                DialogueAction::Teleport { .. } => defer(&mut plan, trace, "teleport"),
                DialogueAction::StartPosition { .. } => defer(&mut plan, trace, "startPosition"),
                DialogueAction::Custom { .. } => defer(&mut plan, trace, "custom"),
            }
        }
        if !repeat {
            break;
        }
    }

    if plan.start_todo {
        plan.final_talk_delay_ms = talk_delay;
        trace.push(DialogueEvent::Todo {
            op: TodoOp::Wait,
            delay_ms: Some(talk_delay),
        });
        trace.push(DialogueEvent::Todo {
            op: TodoOp::Start,
            delay_ms: None,
        });
    }

    let _ = MatchCaptures::default();
    plan
}

fn apply_session_set(
    var: SessionVar,
    value: i32,
    plan: &mut DialoguePlan,
    ctx: &mut EvalContext<'_>,
    trace: &mut DialogueTrace,
) {
    let name = match var {
        SessionVar::Topic => {
            plan.topic = Some(value);
            ctx.topic = value;
            "topic"
        }
        SessionVar::Price => {
            plan.price = Some(value);
            ctx.price = value;
            "price"
        }
        SessionVar::Amount => {
            plan.amount = Some(value);
            ctx.amount = value;
            "amount"
        }
        SessionVar::Type => {
            plan.item_type = Some(value);
            ctx.item_type = value;
            "type"
        }
        SessionVar::Data => {
            plan.data = Some(value);
            ctx.data = value;
            "data"
        }
    };
    trace.push(DialogueEvent::Set { var: name, value });
}

fn defer(plan: &mut DialoguePlan, trace: &mut DialogueTrace, kind: &'static str) {
    plan.deferred.push(kind);
    trace.push(DialogueEvent::DeferredAction { kind });
}
