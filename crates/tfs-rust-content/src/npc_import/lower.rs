//! Lower import AST → [`PendingNpcDefinition`] / [`DialogueProgram`].
//!
//! When an [`ItemDatabase`] is provided, CipSoft TypeIDs (OTB `client_id`) on
//! item literals are remapped to OTB `server_id` via [`ItemDatabase::server_id_for_client`].

use crate::items::ItemDatabase;
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
///
/// Pass `items = Some(...)` when the source uses CipSoft TypeIDs (e.g.
/// `reference/cipsoft-772/runtime/npc`). Pass `None` when literals are already
/// OTB server ids (e.g. TVP archive behavior files).
pub fn lower_npc(
    file: RawNpcFile,
    items: Option<&ItemDatabase>,
) -> ImportResult<PendingNpcDefinition> {
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
        rules.push(lower_rule(rule, items)?);
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
        ..Default::default()
    })
}

fn lower_rule(
    rule: RawRule,
    items: Option<&ItemDatabase>,
) -> ImportResult<DialogueRule> {
    let mut predicates = Vec::new();
    for c in rule.conditions {
        predicates.push(lower_cond(c, items)?);
    }
    let mut actions = Vec::new();
    for a in rule.actions {
        actions.push(lower_action(a, items)?);
    }
    Ok(DialogueRule {
        predicates,
        actions,
        span: rule.span,
    })
}

fn lower_cond(
    c: RawCond,
    items: Option<&ItemDatabase>,
) -> ImportResult<DialoguePredicate> {
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
            expr: lower_expr(lhs, items)?,
            op: lower_op(op),
            rhs: lower_expr(rhs, items)?,
            span,
        }),
    }
}

fn lower_action(
    a: RawAction,
    items: Option<&ItemDatabase>,
) -> ImportResult<DialogueAction> {
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
            let expr = lower_expr(value, items)?;
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
                    expr: remap_item_lit_expr(expr, &span, items)?,
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
        RawAction::Call { name, args, span } => lower_call(&name, args, span, items),
        RawAction::Summon(monster, span) => Ok(DialogueAction::Summon { monster, span }),
        RawAction::Teleport { x, y, z, span } => Ok(DialogueAction::Teleport { x, y, z, span }),
    }
}

fn lower_call(
    name: &str,
    args: Vec<RawExpr>,
    span: SourceSpan,
    items: Option<&ItemDatabase>,
) -> ImportResult<DialogueAction> {
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
                cycles: lower_expr(cycles, items)?,
                param: lower_expr(param, items)?,
                span,
            })
        }
        "poison" => {
            let (cycles, param) = two_args(name, args, &span)?;
            Ok(DialogueAction::Poison {
                cycles: lower_expr(cycles, items)?,
                param: lower_expr(param, items)?,
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
                item: remap_item_lit_expr(lower_expr(item, items)?, &span, items)?,
                count: count
                    .map(|c| lower_expr(c, items))
                    .transpose()?
                    .unwrap_or(DialogueExpr::Lit(1)),
                span,
            })
        }
        "delete" => {
            let (item, count) = one_or_two_expr(name, args, &span)?;
            Ok(DialogueAction::Delete {
                item: remap_item_lit_expr(lower_expr(item, items)?, &span, items)?,
                count: count
                    .map(|c| lower_expr(c, items))
                    .transpose()?
                    .unwrap_or(DialogueExpr::Lit(1)),
                span,
            })
        }
        "createmoney" => {
            let amount = if args.is_empty() {
                DialogueExpr::Session(SessionVar::Amount)
            } else if args.len() == 1 {
                lower_expr(args.into_iter().next().unwrap(), items)?
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
                lower_expr(args.into_iter().next().unwrap(), items)?
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
                value: lower_expr(val, items)?,
                span,
            })
        }
        "profession" => {
            if args.len() != 1 {
                return Err(ImportError::spanned(span, "Profession expects 1 arg"));
            }
            Ok(DialogueAction::Profession {
                vocation: lower_expr(args.into_iter().next().unwrap(), items)?,
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
                spell: lower_expr(args.into_iter().next().unwrap(), items)?,
                span,
            })
        }
        other => Err(ImportError::spanned(
            span,
            format!("unknown action call {other:?}"),
        )),
    }
}

fn lower_expr(e: RawExpr, items: Option<&ItemDatabase>) -> ImportResult<DialogueExpr> {
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
                    let item = remap_item_lit_expr(
                        lower_expr(args.into_iter().next().unwrap(), items)?,
                        &span,
                        items,
                    )?;
                    Ok(DialogueExpr::Count {
                        item: Box::new(item),
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
                        spell: Box::new(lower_expr(args.into_iter().next().unwrap(), items)?),
                    })
                }
                "spelllevel" => {
                    if args.len() != 1 {
                        return Err(ImportError::spanned(span, "SpellLevel expects 1 arg"));
                    }
                    Ok(DialogueExpr::SpellLevel {
                        spell: Box::new(lower_expr(args.into_iter().next().unwrap(), items)?),
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
            lhs: Box::new(lower_expr(*lhs, items)?),
            rhs: Box::new(lower_expr(*rhs, items)?),
        }),
    }
}

/// Remap a CipSoft TypeID literal to OTB `server_id` when `items` is provided.
fn remap_item_lit_expr(
    expr: DialogueExpr,
    span: &SourceSpan,
    items: Option<&ItemDatabase>,
) -> ImportResult<DialogueExpr> {
    match expr {
        DialogueExpr::Lit(n) => Ok(DialogueExpr::Lit(remap_client_item_id(n, span, items)?)),
        other => Ok(other),
    }
}

fn remap_client_item_id(
    n: i32,
    span: &SourceSpan,
    items: Option<&ItemDatabase>,
) -> ImportResult<i32> {
    let Some(db) = items else {
        return Ok(n);
    };
    if n < 0 || n > i32::from(u16::MAX) {
        return Err(ImportError::spanned(
            span.clone(),
            format!("item id {n} out of u16 range"),
        ));
    }
    let client_id = n as u16;
    // Remap when this is a known OTB client_id. Leave unknowns alone — `Type=` is
    // also used for spell ids on teacher NPCs (e.g. Type=20 for "find person").
    Ok(match db.server_id_for_client(client_id) {
        Some(server_id) => i32::from(server_id),
        None => n,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::ItemDatabase;
    use crate::npc_import::parse::parse_npc_source;
    use std::path::PathBuf;

    fn repo_items() -> Option<ItemDatabase> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let otb = root.join("data/items/items.otb");
        let xml = root.join("data/items/items.xml");
        if !otb.exists() || !xml.exists() {
            return None;
        }
        Some(ItemDatabase::load(&otb, &xml).expect("load items"))
    }

    #[test]
    fn remaps_banana_typeid_to_server_id() {
        let Some(items) = repo_items() else {
            eprintln!("skip: data/items missing");
            return;
        };
        // objects.srv TypeID 3587 = banana → OTB server_id 2676
        assert_eq!(items.server_id_for_client(3587), Some(2676));

        let src = r#"
Name = "Shop"
Behaviour = {
"banana" -> Type=3587, Amount=1, Price=5, "buy?", Topic=1
Topic=1,"yes" -> "ok", Create(3587)
"check",Count(3587)>=1 -> "have"
}
"#;
        let root = std::env::temp_dir();
        let file = parse_npc_source(&root, &root.join("shop.npc"), src).expect("parse");
        let pending = lower_npc(file, Some(&items)).expect("lower");
        let dialogue = pending.dialogue.as_ref().expect("dialogue");

        let type_set = dialogue.rules[0]
            .actions
            .iter()
            .find_map(|a| match a {
                DialogueAction::SetSession {
                    var: SessionVar::Type,
                    expr: DialogueExpr::Lit(n),
                    ..
                } => Some(*n),
                _ => None,
            });
        assert_eq!(type_set, Some(2676));

        let create_item = dialogue.rules[1].actions.iter().find_map(|a| match a {
            DialogueAction::Create {
                item: DialogueExpr::Lit(n),
                ..
            } => Some(*n),
            _ => None,
        });
        assert_eq!(create_item, Some(2676));

        let count_item = dialogue.rules[2].predicates.iter().find_map(|p| match p {
            DialoguePredicate::Expression {
                expr: DialogueExpr::Count { item },
                ..
            } => match item.as_ref() {
                DialogueExpr::Lit(n) => Some(*n),
                _ => None,
            },
            _ => None,
        });
        assert_eq!(count_item, Some(2676));
    }

    #[test]
    fn spell_type_ids_not_in_otb_passthrough() {
        let Some(items) = repo_items() else {
            eprintln!("skip: data/items missing");
            return;
        };
        assert!(items.server_id_for_client(20).is_none());

        let src = r#"
Name = "Teacher"
Behaviour = {
Sorcerer,"find","person" -> Type=20, Price=80, "buy spell?", Topic=1
}
"#;
        let root = std::env::temp_dir();
        let file = parse_npc_source(&root, &root.join("teacher.npc"), src).expect("parse");
        let pending = lower_npc(file, Some(&items)).expect("lower");
        let dialogue = pending.dialogue.as_ref().expect("dialogue");
        let type_set = dialogue.rules[0]
            .actions
            .iter()
            .find_map(|a| match a {
                DialogueAction::SetSession {
                    var: SessionVar::Type,
                    expr: DialogueExpr::Lit(n),
                    ..
                } => Some(*n),
                _ => None,
            });
        assert_eq!(type_set, Some(20));
    }

    #[test]
    fn without_items_keeps_typeid_literal() {
        let src = r#"
Name = "Raw"
Behaviour = {
"banana" -> Type=3587, "buy"
}
"#;
        let root = std::env::temp_dir();
        let file = parse_npc_source(&root, &root.join("raw.npc"), src).expect("parse");
        let pending = lower_npc(file, None).expect("lower");
        let dialogue = pending.dialogue.as_ref().expect("dialogue");
        let type_set = dialogue.rules[0]
            .actions
            .iter()
            .find_map(|a| match a {
                DialogueAction::SetSession {
                    var: SessionVar::Type,
                    expr: DialogueExpr::Lit(n),
                    ..
                } => Some(*n),
                _ => None,
            });
        assert_eq!(type_set, Some(3587));
    }
}
