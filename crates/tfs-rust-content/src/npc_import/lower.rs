//! Lower import AST → [`PendingNpcDefinition`] / [`DialogueProgram`].

use crate::npc_import::ast::{
    RawAction, RawCond, RawExpr, RawNpcFile, RawOp, RawRule,
};
use crate::npc_import::error::{ImportError, ImportResult};
use crate::npcs::{
    DialogueAction, DialogueExpr, DialoguePolicy, DialoguePredicate, DialogueProgram,
    DialogueProperty, DialogueRule, DialogueSituation, ExprOp, NpcAppearance, NpcMovement,
    PendingNpcDefinition, SessionVar, SourceSpan,
};

const UNSUPPORTED: &[&str] = &["string", "bless", "town", "promote"];

/// Lower a parsed legacy NPC file into a pending definition.
pub fn lower_npc(file: RawNpcFile) -> ImportResult<PendingNpcDefinition> {
    let name = file.name.clone().ok_or_else(|| {
        ImportError::msg(format!("{}: missing Name", file.source_file))
    })?;

    let appearance = if let Some(o) = file.outfit {
        NpcAppearance {
            look_type: o.look_type,
            look_head: o.look_head,
            look_body: o.look_body,
            look_legs: o.look_legs,
            look_feet: o.look_feet,
            look_addons: 0,
            look_type_ex: o.look_type_ex,
            look_mount: 0,
        }
    } else {
        NpcAppearance::default()
    };

    let mut movement = NpcMovement::default();
    if let Some(r) = file.radius {
        movement.radius = r;
    }
    if let Some(g) = file.go_strength {
        movement.go_strength = g;
        // 772 GoStrength doubles as walk attempt budget; map into speed when XML absent.
        if movement.speed == 100 {
            movement.speed = g.max(1);
        }
    }

    let mut rules = Vec::with_capacity(file.rules.len());
    for rule in file.rules {
        rules.push(lower_rule(rule)?);
    }

    Ok(PendingNpcDefinition {
        name,
        source_file: file.source_file,
        appearance,
        health_max: 100,
        movement,
        speech_bubble: 0,
        sex: file.sex.unwrap_or(1),
        race: file.race.unwrap_or(0),
        parameters: Default::default(),
        voices: Vec::new(),
        dialogue: Some(DialogueProgram {
            policy: DialoguePolicy::QueuedSingleFocus,
            rules,
        }),
        shop: None,
        custom_predicates: Vec::new(),
        custom_actions: Vec::new(),
    })
}

fn lower_rule(rule: RawRule) -> ImportResult<DialogueRule> {
    let mut predicates = Vec::new();
    for c in rule.conditions {
        predicates.push(lower_cond(c)?);
    }
    let mut actions = Vec::new();
    for a in rule.actions {
        actions.push(lower_action(a)?);
    }
    Ok(DialogueRule {
        predicates,
        actions,
        span: rule.span,
    })
}

fn lower_cond(c: RawCond) -> ImportResult<DialoguePredicate> {
    match c {
        RawCond::Situation(name, span) => {
            let kind = match name.as_str() {
                "address" => DialogueSituation::Address,
                "default" => DialogueSituation::Default,
                "busy" => DialogueSituation::Busy,
                "vanish" => DialogueSituation::Vanish,
                "addressqueue" => DialogueSituation::AddressQueue,
                other => {
                    return Err(ImportError::spanned(
                        span,
                        format!("unknown situation {other}"),
                    ));
                }
            };
            Ok(DialoguePredicate::Situation { kind, span })
        }
        RawCond::Words(text, span) => Ok(DialoguePredicate::Words {
            patterns: vec![text],
            span,
        }),
        RawCond::Select(span) => Ok(DialoguePredicate::Select { span }),
        RawCond::Capture(slot, span) => Ok(DialoguePredicate::NumericCapture { slot, span }),
        RawCond::Property(name, span) => {
            let name = match name.as_str() {
                "address" => DialogueProperty::Address,
                "busy" => DialogueProperty::Busy,
                "vanish" => DialogueProperty::Vanish,
                "male" => DialogueProperty::Male,
                "female" => DialogueProperty::Female,
                "knight" => DialogueProperty::Knight,
                "paladin" => DialogueProperty::Paladin,
                "sorcerer" => DialogueProperty::Sorcerer,
                "druid" => DialogueProperty::Druid,
                "premium" => DialogueProperty::Premium,
                "promoted" => DialogueProperty::Promoted,
                "pvpenforced" => DialogueProperty::PvpEnforced,
                "nonpvp" => DialogueProperty::NonPvp,
                "pzblock" => DialogueProperty::PzBlock,
                other => {
                    return Err(ImportError::spanned(
                        span,
                        format!("unknown property {other}"),
                    ));
                }
            };
            Ok(DialoguePredicate::Property { name, span })
        }
        RawCond::Compare {
            lhs,
            op,
            rhs,
            span,
        } => Ok(DialoguePredicate::Expression {
            expr: lower_expr(lhs)?,
            op: lower_op(op),
            rhs: lower_expr(rhs)?,
            span,
        }),
    }
}

fn lower_action(a: RawAction) -> ImportResult<DialogueAction> {
    match a {
        RawAction::Say(text, span) => Ok(DialogueAction::Say { text, span }),
        RawAction::Repeat(span) => Ok(DialogueAction::RepeatPrevious { span }),
        RawAction::Ident(name, span) => match name.as_str() {
            "idle" => Ok(DialogueAction::Idle { span }),
            "queue" => Ok(DialogueAction::Queue { span }),
            "nop" => Ok(DialogueAction::Nop { span }),
            "startposition" => Ok(DialogueAction::StartPosition {
                pos: None,
                span,
            }),
            // Bare money ops: 772 DeleteMoney uses Price; CreateMoney uses Amount.
            "deletemoney" => Ok(DialogueAction::DeleteMoney {
                amount: DialogueExpr::Session(SessionVar::Price),
                span,
            }),
            "createmoney" => Ok(DialogueAction::CreateMoney {
                amount: DialogueExpr::Session(SessionVar::Amount),
                span,
            }),
            other if UNSUPPORTED.contains(&other) => Err(ImportError::spanned(
                span,
                format!("unsupported action {other:?}"),
            )),
            other => Err(ImportError::spanned(
                span,
                format!("unknown action identifier {other:?}"),
            )),
        },
        RawAction::Assign { name, value, span } => {
            if UNSUPPORTED.contains(&name.as_str()) {
                return Err(ImportError::spanned(
                    span,
                    format!("unsupported assignment {name:?}"),
                ));
            }
            let expr = lower_expr(value)?;
            match name.as_str() {
                "topic" => Ok(DialogueAction::SetSession {
                    var: SessionVar::Topic,
                    expr,
                    span,
                }),
                "price" => Ok(DialogueAction::SetSession {
                    var: SessionVar::Price,
                    expr,
                    span,
                }),
                "amount" => Ok(DialogueAction::SetSession {
                    var: SessionVar::Amount,
                    expr,
                    span,
                }),
                "type" => Ok(DialogueAction::SetSession {
                    var: SessionVar::Type,
                    expr,
                    span,
                }),
                "data" => Ok(DialogueAction::SetSession {
                    var: SessionVar::Data,
                    expr,
                    span,
                }),
                "hp" => Ok(DialogueAction::SetHp { expr, span }),
                other => Err(ImportError::spanned(
                    span,
                    format!("unknown assignment target {other:?}"),
                )),
            }
        }
        RawAction::Call { name, args, span } => lower_call(&name, args, span),
        RawAction::Summon(monster, span) => Ok(DialogueAction::Summon { monster, span }),
        RawAction::Teleport { x, y, z, span } => Ok(DialogueAction::Teleport { x, y, z, span }),
    }
}

fn lower_call(name: &str, args: Vec<RawExpr>, span: SourceSpan) -> ImportResult<DialogueAction> {
    if UNSUPPORTED.contains(&name) {
        return Err(ImportError::spanned(
            span,
            format!("unsupported action {name:?}"),
        ));
    }
    match name {
        "burning" => {
            let (cycles, param) = two_args(name, args, &span)?;
            Ok(DialogueAction::Burning {
                cycles: lower_expr(cycles)?,
                param: lower_expr(param)?,
                span,
            })
        }
        "poison" => {
            let (cycles, param) = two_args(name, args, &span)?;
            Ok(DialogueAction::Poison {
                cycles: lower_expr(cycles)?,
                param: lower_expr(param)?,
                span,
            })
        }
        "effectme" => {
            let id = one_lit_u16(name, args, &span)?;
            Ok(DialogueAction::EffectMe {
                effect_id: id,
                span,
            })
        }
        "effectopp" => {
            let id = one_lit_u16(name, args, &span)?;
            Ok(DialogueAction::EffectOpp {
                effect_id: id,
                span,
            })
        }
        "create" => {
            let (item, count) = one_or_two_expr(name, args, &span)?;
            Ok(DialogueAction::Create {
                item: lower_expr(item)?,
                count: count
                    .map(lower_expr)
                    .transpose()?
                    .unwrap_or(DialogueExpr::Lit(1)),
                span,
            })
        }
        "delete" => {
            let (item, count) = one_or_two_expr(name, args, &span)?;
            Ok(DialogueAction::Delete {
                item: lower_expr(item)?,
                count: count
                    .map(lower_expr)
                    .transpose()?
                    .unwrap_or(DialogueExpr::Lit(1)),
                span,
            })
        }
        "createmoney" => {
            let amount = if args.is_empty() {
                DialogueExpr::Session(SessionVar::Amount)
            } else if args.len() == 1 {
                lower_expr(args.into_iter().next().unwrap())?
            } else {
                return Err(ImportError::spanned(
                    span,
                    "CreateMoney expects 0 or 1 args",
                ));
            };
            Ok(DialogueAction::CreateMoney { amount, span })
        }
        "deletemoney" => {
            let amount = if args.is_empty() {
                DialogueExpr::Session(SessionVar::Price)
            } else if args.len() == 1 {
                lower_expr(args.into_iter().next().unwrap())?
            } else {
                return Err(ImportError::spanned(
                    span,
                    "DeleteMoney expects 0 or 1 args",
                ));
            };
            Ok(DialogueAction::DeleteMoney { amount, span })
        }
        "setquestvalue" => {
            if args.len() != 2 {
                return Err(ImportError::spanned(
                    span,
                    "SetQuestValue expects (id, value)",
                ));
            }
            let mut it = args.into_iter();
            let id_expr = it.next().unwrap();
            let val = it.next().unwrap();
            let storage_id = match id_expr {
                RawExpr::Lit(n, _) if n >= 0 => n as u32,
                other => {
                    return Err(ImportError::spanned(
                        span_of_expr(&other),
                        "SetQuestValue id must be a literal",
                    ));
                }
            };
            Ok(DialogueAction::SetQuestValue {
                storage_id,
                value: lower_expr(val)?,
                span,
            })
        }
        "profession" => {
            if args.len() != 1 {
                return Err(ImportError::spanned(span, "Profession expects 1 arg"));
            }
            Ok(DialogueAction::Profession {
                vocation: lower_expr(args.into_iter().next().unwrap())?,
                span,
            })
        }
        "startposition" => {
            if args.is_empty() {
                Ok(DialogueAction::StartPosition { pos: None, span })
            } else if args.len() == 3 {
                let mut it = args.into_iter();
                let x = match it.next().unwrap() {
                    RawExpr::Lit(n, _) => n,
                    other => {
                        return Err(ImportError::spanned(
                            span_of_expr(&other),
                            "StartPosition coords must be literals",
                        ));
                    }
                };
                let y = match it.next().unwrap() {
                    RawExpr::Lit(n, _) => n,
                    other => {
                        return Err(ImportError::spanned(
                            span_of_expr(&other),
                            "StartPosition coords must be literals",
                        ));
                    }
                };
                let z = match it.next().unwrap() {
                    RawExpr::Lit(n, _) => n,
                    other => {
                        return Err(ImportError::spanned(
                            span_of_expr(&other),
                            "StartPosition coords must be literals",
                        ));
                    }
                };
                Ok(DialogueAction::StartPosition {
                    pos: Some((x, y, z)),
                    span,
                })
            } else {
                Err(ImportError::spanned(
                    span,
                    "StartPosition expects 0 or 3 args",
                ))
            }
        }
        "teachspell" => {
            if args.len() != 1 {
                return Err(ImportError::spanned(span, "TeachSpell expects 1 arg"));
            }
            Ok(DialogueAction::TeachSpell {
                spell: lower_expr(args.into_iter().next().unwrap())?,
                span,
            })
        }
        other => Err(ImportError::spanned(
            span,
            format!("unknown action call {other:?}"),
        )),
    }
}

fn lower_expr(e: RawExpr) -> ImportResult<DialogueExpr> {
    match e {
        RawExpr::Lit(n, _) => Ok(DialogueExpr::Lit(n)),
        RawExpr::Capture(slot, _) => Ok(DialogueExpr::Capture { slot }),
        RawExpr::Ident(name, span) => match name.as_str() {
            "topic" => Ok(DialogueExpr::Session(SessionVar::Topic)),
            "price" => Ok(DialogueExpr::Session(SessionVar::Price)),
            "amount" => Ok(DialogueExpr::Session(SessionVar::Amount)),
            "type" => Ok(DialogueExpr::Session(SessionVar::Type)),
            "data" => Ok(DialogueExpr::Session(SessionVar::Data)),
            "hp" => Ok(DialogueExpr::Hp),
            "burning" => Ok(DialogueExpr::Burning),
            "poison" => Ok(DialogueExpr::Poison),
            "countmoney" => Ok(DialogueExpr::CountMoney),
            "level" => Ok(DialogueExpr::Level),
            "magiclevel" => Ok(DialogueExpr::MagicLevel),
            other if UNSUPPORTED.contains(&other) => Err(ImportError::spanned(
                span,
                format!("unsupported expression {other:?}"),
            )),
            other => Err(ImportError::spanned(
                span,
                format!("unknown expression identifier {other:?}"),
            )),
        },
        RawExpr::Call { name, args, span } => {
            if UNSUPPORTED.contains(&name.as_str()) {
                return Err(ImportError::spanned(
                    span,
                    format!("unsupported expression {name:?}"),
                ));
            }
            match name.as_str() {
                "count" => {
                    if args.len() != 1 {
                        return Err(ImportError::spanned(span, "Count expects 1 arg"));
                    }
                    Ok(DialogueExpr::Count {
                        item: Box::new(lower_expr(args.into_iter().next().unwrap())?),
                    })
                }
                "questvalue" => {
                    if args.len() != 1 {
                        return Err(ImportError::spanned(span, "QuestValue expects 1 arg"));
                    }
                    let a = args.into_iter().next().unwrap();
                    let storage_id = match a {
                        RawExpr::Lit(n, _) if n >= 0 => n as u32,
                        other => {
                            return Err(ImportError::spanned(
                                span_of_expr(&other),
                                "QuestValue id must be a literal",
                            ));
                        }
                    };
                    Ok(DialogueExpr::QuestValue { storage_id })
                }
                "random" => {
                    if args.len() != 2 {
                        return Err(ImportError::spanned(span, "Random expects 2 args"));
                    }
                    let mut it = args.into_iter();
                    let lo = match it.next().unwrap() {
                        RawExpr::Lit(n, _) => n,
                        other => {
                            return Err(ImportError::spanned(
                                span_of_expr(&other),
                                "Random lo must be literal",
                            ));
                        }
                    };
                    let hi = match it.next().unwrap() {
                        RawExpr::Lit(n, _) => n,
                        other => {
                            return Err(ImportError::spanned(
                                span_of_expr(&other),
                                "Random hi must be literal",
                            ));
                        }
                    };
                    Ok(DialogueExpr::Random { lo, hi })
                }
                "spellknown" => {
                    if args.len() != 1 {
                        return Err(ImportError::spanned(span, "SpellKnown expects 1 arg"));
                    }
                    Ok(DialogueExpr::SpellKnown {
                        spell: Box::new(lower_expr(args.into_iter().next().unwrap())?),
                    })
                }
                "spelllevel" => {
                    if args.len() != 1 {
                        return Err(ImportError::spanned(span, "SpellLevel expects 1 arg"));
                    }
                    Ok(DialogueExpr::SpellLevel {
                        spell: Box::new(lower_expr(args.into_iter().next().unwrap())?),
                    })
                }
                "countmoney" => {
                    if !args.is_empty() {
                        return Err(ImportError::spanned(span, "CountMoney takes no args"));
                    }
                    Ok(DialogueExpr::CountMoney)
                }
                other => Err(ImportError::spanned(
                    span,
                    format!("unknown expression call {other:?}"),
                )),
            }
        }
        RawExpr::Binary {
            op,
            lhs,
            rhs,
            span: _,
        } => Ok(DialogueExpr::Binary {
            op: lower_op(op),
            lhs: Box::new(lower_expr(*lhs)?),
            rhs: Box::new(lower_expr(*rhs)?),
        }),
    }
}

fn lower_op(op: RawOp) -> ExprOp {
    match op {
        RawOp::Add => ExprOp::Add,
        RawOp::Sub => ExprOp::Sub,
        RawOp::Mul => ExprOp::Mul,
        RawOp::Eq => ExprOp::Eq,
        RawOp::Ne => ExprOp::Ne,
        RawOp::Lt => ExprOp::Lt,
        RawOp::Le => ExprOp::Le,
        RawOp::Gt => ExprOp::Gt,
        RawOp::Ge => ExprOp::Ge,
    }
}

fn two_args(
    name: &str,
    args: Vec<RawExpr>,
    span: &SourceSpan,
) -> ImportResult<(RawExpr, RawExpr)> {
    if args.len() != 2 {
        return Err(ImportError::spanned(
            span.clone(),
            format!("{name} expects 2 args"),
        ));
    }
    let mut it = args.into_iter();
    Ok((it.next().unwrap(), it.next().unwrap()))
}

fn one_or_two_expr(
    name: &str,
    args: Vec<RawExpr>,
    span: &SourceSpan,
) -> ImportResult<(RawExpr, Option<RawExpr>)> {
    match args.len() {
        1 => Ok((args.into_iter().next().unwrap(), None)),
        2 => {
            let mut it = args.into_iter();
            Ok((it.next().unwrap(), Some(it.next().unwrap())))
        }
        _ => Err(ImportError::spanned(
            span.clone(),
            format!("{name} expects 1 or 2 args"),
        )),
    }
}

fn one_lit_u16(name: &str, args: Vec<RawExpr>, span: &SourceSpan) -> ImportResult<u16> {
    if args.len() != 1 {
        return Err(ImportError::spanned(
            span.clone(),
            format!("{name} expects 1 arg"),
        ));
    }
    match args.into_iter().next().unwrap() {
        RawExpr::Lit(n, _) if n >= 0 && n <= u16::MAX as i32 => Ok(n as u16),
        other => Err(ImportError::spanned(
            span_of_expr(&other),
            format!("{name} arg must be a non-negative literal"),
        )),
    }
}

fn span_of_expr(e: &RawExpr) -> SourceSpan {
    match e {
        RawExpr::Lit(_, s)
        | RawExpr::Ident(_, s)
        | RawExpr::Capture(_, s)
        | RawExpr::Call { span: s, .. }
        | RawExpr::Binary { span: s, .. } => s.clone(),
    }
}
