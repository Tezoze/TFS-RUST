//! Validate pending NPC definitions into an immutable [`NpcDatabase`].
//!
//! Checks: duplicate names, empty names, impossible expressions, missing custom
//! callback references, unknown item ids (when [`ItemDatabase`] is provided),
//! and unsupported legacy constructs surfaced as explicit action/predicate errors
//! at parse time (bless/town/string/promote rejected in the Lua bridge).

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::items::ItemDatabase;
use crate::npcs::dialogue::{
    DialogueAction, DialogueExpr, DialoguePredicate, DialogueProgram, NpcCallbackId,
};
use crate::npcs::{NpcDatabase, NpcDefinition, NpcTypeId, PendingNpcDefinition};

/// Validation / freeze errors for NPC definitions.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NpcValidateError {
    #[error("NPC definition error in {file}: {message}")]
    Content { file: String, message: String },
}

impl NpcValidateError {
    fn content(file: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Content {
            file: file.into(),
            message: message.into(),
        }
    }
}

/// Validate and freeze pending definitions into [`NpcDatabase`].
///
/// When `items` is `Some`, `create`/`delete`/`count` item ids must exist.
pub fn validate_pending_definitions(
    pending: Vec<PendingNpcDefinition>,
    items: Option<&ItemDatabase>,
) -> Result<NpcDatabase, NpcValidateError> {
    let mut seen_names: HashMap<String, String> = HashMap::new();
    let mut defs = Vec::with_capacity(pending.len());

    for (index, p) in pending.into_iter().enumerate() {
        let file = if p.source_file.is_empty() {
            "<pending>".to_string()
        } else {
            p.source_file.clone()
        };

        if p.name.trim().is_empty() {
            return Err(NpcValidateError::content(
                &file,
                "NpcType name must not be empty",
            ));
        }

        let key = p.name.to_ascii_lowercase();
        if let Some(prev) = seen_names.insert(key, p.name.clone()) {
            return Err(NpcValidateError::content(
                &file,
                format!(
                    "duplicate NPC name {:?} (already registered as {:?})",
                    p.name, prev
                ),
            ));
        }

        if let Some(ref dialogue) = p.dialogue {
            validate_dialogue(&file, dialogue, &p, items)?;
        }

        if let Some(ref shop) = p.shop {
            if let Some(items_db) = items {
                for (i, line) in shop.items.iter().enumerate() {
                    if !items_db.items.contains_key(&line.item_id) {
                        return Err(NpcValidateError::content(
                            &file,
                            format!("shop item[{i}]: unknown item id {}", line.item_id),
                        ));
                    }
                }
            }
        }

        let id = NpcTypeId(index as u32);
        defs.push(NpcDefinition {
            id,
            name: p.name,
            appearance: p.appearance,
            health_max: if p.health_max == 0 { 100 } else { p.health_max },
            movement: p.movement,
            speech_bubble: p.speech_bubble,
            sex: p.sex,
            race: p.race,
            parameters: p.parameters,
            voices: p.voices,
            dialogue: p.dialogue,
            shop: p.shop,
            custom_predicates: p.custom_predicates,
            custom_actions: p.custom_actions,
            on_appear: p.on_appear,
            on_disappear: p.on_disappear,
            on_move: p.on_move,
            on_say: p.on_say,
            on_think: p.on_think,
        });
    }

    Ok(NpcDatabase::from_validated(defs))
}

fn validate_dialogue(
    file: &str,
    dialogue: &DialogueProgram,
    pending: &PendingNpcDefinition,
    items: Option<&ItemDatabase>,
) -> Result<(), NpcValidateError> {
    let predicate_names: HashSet<&str> = pending
        .custom_predicates
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    let action_names: HashSet<&str> = pending
        .custom_actions
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    let predicate_ids: HashSet<NpcCallbackId> =
        pending.custom_predicates.iter().map(|s| s.id).collect();
    let action_ids: HashSet<NpcCallbackId> = pending.custom_actions.iter().map(|s| s.id).collect();

    for (ri, rule) in dialogue.rules.iter().enumerate() {
        if rule.predicates.is_empty() && rule.actions.is_empty() {
            return Err(NpcValidateError::content(
                file,
                format!("rule[{ri}]: empty when/actions"),
            ));
        }
        for (pi, pred) in rule.predicates.iter().enumerate() {
            validate_predicate(file, ri, pi, pred, &predicate_names, &predicate_ids, items)?;
        }
        for (ai, action) in rule.actions.iter().enumerate() {
            validate_action(file, ri, ai, action, &action_names, &action_ids, items)?;
        }
    }
    Ok(())
}

fn validate_predicate(
    file: &str,
    ri: usize,
    pi: usize,
    pred: &DialoguePredicate,
    names: &HashSet<&str>,
    ids: &HashSet<NpcCallbackId>,
    items: Option<&ItemDatabase>,
) -> Result<(), NpcValidateError> {
    match pred {
        DialoguePredicate::Words { patterns, .. } => {
            if patterns.is_empty() {
                return Err(NpcValidateError::content(
                    file,
                    format!("rule[{ri}].when[{pi}]: words list must not be empty"),
                ));
            }
        }
        DialoguePredicate::NumericCapture { slot, .. } => {
            if *slot == 0 || *slot > 2 {
                return Err(NpcValidateError::content(
                    file,
                    format!(
                        "rule[{ri}].when[{pi}]: numeric capture slot must be 1 or 2 (got {slot})"
                    ),
                ));
            }
        }
        DialoguePredicate::Expression { expr, rhs, .. } => {
            validate_expr(file, &format!("rule[{ri}].when[{pi}].expr"), expr, items)?;
            validate_expr(file, &format!("rule[{ri}].when[{pi}].rhs"), rhs, items)?;
        }
        DialoguePredicate::Custom {
            callback_id, name, ..
        } => {
            if !names.contains(name.as_str()) || !ids.contains(callback_id) {
                return Err(NpcValidateError::content(
                    file,
                    format!("rule[{ri}].when[{pi}]: missing custom predicate callback {name:?}"),
                ));
            }
        }
        DialoguePredicate::Situation { .. }
        | DialoguePredicate::Property { .. }
        | DialoguePredicate::Select { .. } => {}
    }
    Ok(())
}

fn validate_action(
    file: &str,
    ri: usize,
    ai: usize,
    action: &DialogueAction,
    names: &HashSet<&str>,
    ids: &HashSet<NpcCallbackId>,
    items: Option<&ItemDatabase>,
) -> Result<(), NpcValidateError> {
    let loc = format!("rule[{ri}].actions[{ai}]");
    match action {
        DialogueAction::Say { text, .. } => {
            if text.is_empty() {
                return Err(NpcValidateError::content(
                    file,
                    format!("{loc}: say text must not be empty"),
                ));
            }
        }
        DialogueAction::SetSession { expr, .. } | DialogueAction::SetHp { expr, .. } => {
            validate_expr(file, &format!("{loc}.expr"), expr, items)?;
        }
        DialogueAction::Create { item, count, .. } | DialogueAction::Delete { item, count, .. } => {
            validate_item_expr(file, &format!("{loc}.item"), item, items)?;
            validate_expr(file, &format!("{loc}.count"), count, items)?;
        }
        DialogueAction::CreateMoney { amount, .. } | DialogueAction::DeleteMoney { amount, .. } => {
            validate_expr(file, &format!("{loc}.amount"), amount, items)?;
        }
        DialogueAction::Burning { cycles, param, .. }
        | DialogueAction::Poison { cycles, param, .. } => {
            validate_expr(file, &format!("{loc}.cycles"), cycles, items)?;
            validate_expr(file, &format!("{loc}.param"), param, items)?;
        }
        DialogueAction::SetQuestValue { value, .. } => {
            validate_expr(file, &format!("{loc}.value"), value, items)?;
        }
        DialogueAction::TeachSpell { spell, .. } => {
            validate_expr(file, &format!("{loc}.spell"), spell, items)?;
        }
        DialogueAction::Profession { vocation, .. } => {
            validate_expr(file, &format!("{loc}.vocation"), vocation, items)?;
        }
        DialogueAction::Summon { monster, .. } => {
            if monster.trim().is_empty() {
                return Err(NpcValidateError::content(
                    file,
                    format!("{loc}: summon monster must not be empty"),
                ));
            }
        }
        DialogueAction::Custom {
            callback_id, name, ..
        } => {
            if !names.contains(name.as_str()) || !ids.contains(callback_id) {
                return Err(NpcValidateError::content(
                    file,
                    format!("{loc}: missing custom action callback {name:?}"),
                ));
            }
        }
        DialogueAction::Idle { .. }
        | DialogueAction::Queue { .. }
        | DialogueAction::Nop { .. }
        | DialogueAction::StartPosition { .. }
        | DialogueAction::EffectMe { .. }
        | DialogueAction::EffectOpp { .. }
        | DialogueAction::Teleport { .. }
        | DialogueAction::RepeatPrevious { .. } => {}
    }
    Ok(())
}

fn validate_item_expr(
    file: &str,
    loc: &str,
    expr: &DialogueExpr,
    items: Option<&ItemDatabase>,
) -> Result<(), NpcValidateError> {
    validate_expr(file, loc, expr, items)?;
    if let (Some(db), DialogueExpr::Lit(id)) = (items, expr) {
        let item_id = *id as u16;
        if *id < 0 || *id > u16::MAX as i32 || !db.items.contains_key(&item_id) {
            return Err(NpcValidateError::content(
                file,
                format!("{loc}: unknown item id {id}"),
            ));
        }
    }
    Ok(())
}

fn validate_expr(
    file: &str,
    loc: &str,
    expr: &DialogueExpr,
    items: Option<&ItemDatabase>,
) -> Result<(), NpcValidateError> {
    match expr {
        DialogueExpr::Random { lo, hi } => {
            if lo > hi {
                return Err(NpcValidateError::content(
                    file,
                    format!("{loc}: random({lo},{hi}) has lo > hi"),
                ));
            }
        }
        DialogueExpr::Count { item } => {
            validate_item_expr(file, &format!("{loc}.count"), item, items)?;
        }
        DialogueExpr::Binary { lhs, rhs, .. } => {
            validate_expr(file, loc, lhs, items)?;
            validate_expr(file, loc, rhs, items)?;
        }
        DialogueExpr::SpellKnown { spell } | DialogueExpr::SpellLevel { spell } => {
            validate_expr(file, loc, spell, items)?;
        }
        DialogueExpr::Lit(_)
        | DialogueExpr::Session(_)
        | DialogueExpr::Capture { .. }
        | DialogueExpr::Hp
        | DialogueExpr::Burning
        | DialogueExpr::Poison
        | DialogueExpr::CountMoney
        | DialogueExpr::Level
        | DialogueExpr::MagicLevel
        | DialogueExpr::QuestValue { .. } => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npcs::dialogue::{
        DialogueAction, DialoguePolicy, DialoguePredicate, DialogueRule, DialogueSituation,
        SessionVar,
    };
    use crate::npcs::span::SourceSpan;
    use crate::npcs::{NpcAppearance, NpcMovement};

    fn span() -> SourceSpan {
        SourceSpan::lua("test.lua", 1)
    }

    fn minimal_pending(name: &str) -> PendingNpcDefinition {
        PendingNpcDefinition {
            name: name.to_string(),
            source_file: "test.lua".into(),
            appearance: NpcAppearance::default(),
            health_max: 100,
            movement: NpcMovement::default(),
            dialogue: Some(DialogueProgram {
                policy: DialoguePolicy::QueuedSingleFocus,
                rules: vec![DialogueRule {
                    predicates: vec![DialoguePredicate::Situation {
                        kind: DialogueSituation::Address,
                        span: span(),
                    }],
                    actions: vec![DialogueAction::Say {
                        text: "hello".into(),
                        span: span(),
                    }],
                    span: span(),
                }],
            }),
            ..Default::default()
        }
    }

    #[test]
    fn freezes_unique_names() {
        let db = validate_pending_definitions(vec![minimal_pending("Quentin")], None).expect("ok");
        assert_eq!(db.len(), 1);
        let def = db.get_by_name("quentin").expect("lookup");
        assert_eq!(def.name, "Quentin");
        assert_eq!(def.id, NpcTypeId(0));
    }

    #[test]
    fn rejects_duplicate_names() {
        let err = validate_pending_definitions(
            vec![minimal_pending("Quentin"), minimal_pending("quentin")],
            None,
        )
        .expect_err("dup");
        match err {
            NpcValidateError::Content { message, .. } => {
                assert!(message.contains("duplicate"), "{message}");
            }
        }
    }

    #[test]
    fn rejects_impossible_random() {
        let mut p = minimal_pending("Bad");
        if let Some(d) = p.dialogue.as_mut() {
            d.rules[0].actions = vec![DialogueAction::SetSession {
                var: SessionVar::Amount,
                expr: DialogueExpr::Random { lo: 5, hi: 1 },
                span: span(),
            }];
        }
        let err = validate_pending_definitions(vec![p], None).expect_err("random");
        match err {
            NpcValidateError::Content { message, .. } => {
                assert!(message.contains("random"), "{message}");
            }
        }
    }

    #[test]
    fn rejects_missing_custom_action_callback() {
        use crate::npcs::dialogue::NpcCallbackId;
        let mut p = minimal_pending("Custom");
        if let Some(d) = p.dialogue.as_mut() {
            d.rules[0].actions = vec![DialogueAction::Custom {
                callback_id: NpcCallbackId(1),
                name: "quest_reward".into(),
                span: span(),
            }];
        }
        let err = validate_pending_definitions(vec![p], None).expect_err("missing cb");
        match err {
            NpcValidateError::Content { message, .. } => {
                assert!(message.contains("missing custom action"), "{message}");
            }
        }
    }
}
