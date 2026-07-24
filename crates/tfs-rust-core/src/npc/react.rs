//! Apply a matched rule's actions into a plan + immediate mutations (NPC-5).

use tfs_rust_content::npcs::{DialogueAction, DialogueProgram, SessionVar};

use super::actions::{log_action_failure, ActionFailCtx, NpcActionHost};
use super::events::{DialogueEvent, DialogueSituationKind, DialogueTrace, MutateOp, TodoOp};
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
    /// Idle after speech already queued → Leaving now + `ToDoChangeState(IDLE)` (`crnonpl.cc:1219-1222`).
    pub deferred_idle: bool,
    pub topic: Option<i32>,
    pub price: Option<i32>,
    pub amount: Option<i32>,
    pub item_type: Option<i32>,
    pub data: Option<i32>,
    pub start_todo: bool,
    pub final_talk_delay_ms: u32,
}

/// Metadata for mutation logging / EffectMe / Summon placement.
pub struct ReactMeta<'a> {
    pub npc_id: CreatureId,
    pub npc_name: &'a str,
}

/// Execute matched rule actions (and `*` repeats) into [`DialoguePlan`] + trace events.
///
/// C++ action loop — `crnonpl.cc:1085-1291`. Mutating world actions apply immediately
/// via [`NpcActionHost`] (NPC-5). Custom actions invoke Lua via
/// [`NpcActionHost::invoke_custom_action`] (NPC-7).
pub fn apply_dialogue_plan(
    program: &DialogueProgram,
    matched: RuleMatch,
    situation: DialogueSituationKind,
    player: CreatureId,
    text: &str,
    ctx: &mut EvalContext<'_>,
    tuning: NpcTuning,
    host: &mut dyn NpcActionHost,
    meta: &ReactMeta<'_>,
    trace: &mut DialogueTrace,
) -> DialoguePlan {
    let _ = text;
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
        let rule_span = rule.span.display();
        let mut repeat = false;
        for (action_index, action) in rule.actions.iter().enumerate() {
            let fail_ctx = ActionFailCtx {
                npc_name: meta.npc_name,
                player,
                rule_span: &rule_span,
                action_index,
            };
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
                    } else {
                        // After queued speech, Idle becomes Leaving then ToDoChangeState
                        // (`crnonpl.cc:1219-1222`).
                        plan.deferred_idle = true;
                    }
                    plan.start_todo = true;
                }
                DialogueAction::Queue { .. } => {
                    if situation == DialogueSituationKind::Busy {
                        plan.queue_player = true;
                    }
                }
                DialogueAction::Nop { .. } => {}
                DialogueAction::Create { item, count, .. } => {
                    let item_id = super::expr::eval_expr(item, ctx);
                    let count_v = super::expr::eval_expr(count, ctx);
                    match host.create_item(player, item_id, count_v) {
                        Ok(()) => {
                            refresh_money(ctx);
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::CreateItem {
                                    item_id,
                                    count: count_v,
                                },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::Delete { item, count, .. } => {
                    let item_id = super::expr::eval_expr(item, ctx);
                    let count_v = super::expr::eval_expr(count, ctx);
                    match host.delete_item(player, item_id, count_v) {
                        Ok(()) => {
                            refresh_money(ctx);
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::DeleteItem {
                                    item_id,
                                    count: count_v,
                                },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::CreateMoney { amount, .. } => {
                    let amount_v = super::expr::eval_expr(amount, ctx);
                    match host.create_money(player, amount_v) {
                        Ok(()) => {
                            refresh_money(ctx);
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::CreateMoney { amount: amount_v },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::DeleteMoney { amount, .. } => {
                    let amount_v = super::expr::eval_expr(amount, ctx);
                    match host.delete_money(player, amount_v) {
                        Ok(()) => {
                            refresh_money(ctx);
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::DeleteMoney { amount: amount_v },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::SetHp { expr, .. } => {
                    let value = super::expr::eval_expr(expr, ctx);
                    match host.set_hp(player, value) {
                        Ok(()) => {
                            ctx.player_hp = value;
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::SetHp { value },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::Burning { cycles, param, .. } => {
                    let cycles_v = super::expr::eval_expr(cycles, ctx);
                    let param_v = super::expr::eval_expr(param, ctx);
                    match host.set_burning(player, cycles_v, param_v) {
                        Ok(()) => {
                            ctx.burning = cycles_v;
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::SetCondition {
                                    condition: "burning",
                                    value: cycles_v,
                                },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::Poison { cycles, param, .. } => {
                    let cycles_v = super::expr::eval_expr(cycles, ctx);
                    let param_v = super::expr::eval_expr(param, ctx);
                    match host.set_poison(player, cycles_v, param_v) {
                        Ok(()) => {
                            ctx.poison = cycles_v;
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::SetCondition {
                                    condition: "poison",
                                    value: cycles_v,
                                },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::EffectMe { effect_id, .. } => {
                    match host.effect_me(meta.npc_id, *effect_id) {
                        Ok(()) => {
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::Effect {
                                    effect_id: *effect_id,
                                    on_npc: true,
                                },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::EffectOpp { effect_id, .. } => {
                    match host.effect_opp(player, *effect_id) {
                        Ok(()) => {
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::Effect {
                                    effect_id: *effect_id,
                                    on_npc: false,
                                },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::SetQuestValue {
                    storage_id, value, ..
                } => {
                    let value_v = super::expr::eval_expr(value, ctx);
                    match host.set_quest_value(player, *storage_id, value_v) {
                        Ok(()) => {
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::SetQuestValue {
                                    id: *storage_id,
                                    value: value_v,
                                },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::Profession { vocation, .. } => {
                    let voc = super::expr::eval_expr(vocation, ctx);
                    match host.set_profession(player, voc) {
                        Ok(()) => {
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::Profession { vocation: voc },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::TeachSpell { spell, .. } => {
                    let spell_v = super::expr::eval_expr(spell, ctx);
                    match host.teach_spell(player, spell_v) {
                        Ok(()) => {
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::TeachSpell { spell: spell_v },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::Summon { monster, .. } => match host.summon(meta.npc_id, monster) {
                    Ok(()) => {
                        trace.push(DialogueEvent::Mutate {
                            player,
                            op: MutateOp::Summon {
                                monster: monster.clone(),
                            },
                        });
                    }
                    Err(e) => log_action_failure(&fail_ctx, &e),
                },
                DialogueAction::Teleport { x, y, z, .. } => {
                    match host.teleport(player, *x, *y, *z) {
                        Ok(()) => {
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::Teleport {
                                    x: *x,
                                    y: *y,
                                    z: *z,
                                },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::StartPosition { pos, .. } => {
                    match host.set_start_position(player, meta.npc_id, *pos) {
                        Ok((x, y, z)) => {
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::StartPosition { x, y, z },
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
                DialogueAction::Custom { callback_id, .. } => {
                    match host.invoke_custom_action(meta.npc_id, player, *callback_id) {
                        Ok(()) => {
                            trace.push(DialogueEvent::Mutate {
                                player,
                                op: MutateOp::CustomAction,
                            });
                        }
                        Err(e) => log_action_failure(&fail_ctx, &e),
                    }
                }
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

fn refresh_money(ctx: &mut EvalContext<'_>) {
    let g = (ctx.inventory_count)(2148) as i64;
    let p = (ctx.inventory_count)(2152) as i64;
    let c = (ctx.inventory_count)(2160) as i64;
    ctx.money = (g + p * 100 + c * 10_000).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
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
