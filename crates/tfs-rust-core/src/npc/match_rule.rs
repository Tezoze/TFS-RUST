//! Deterministic rule selection (condition count, `!`, captures).

use tfs_rust_content::npcs::{
    DialoguePredicate, DialogueProgram, DialogueProperty, DialogueSituation, DialogueSituation as Sit,
};

use super::events::DialogueSituationKind;
use super::expr::{eval_compare, EvalContext, PlayerVocationKind};
use super::words::{search_for_number, search_for_word};

/// Captured `%1` / `%2` values for a matched rule (`-1` = unset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchCaptures {
    pub values: [i32; 2],
}

impl Default for MatchCaptures {
    fn default() -> Self {
        Self { values: [-1, -1] }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleMatch {
    pub rule_index: usize,
    pub captures: MatchCaptures,
}

/// Select the best matching rule for `text` under `situation`.
///
/// C++ `TBehaviourDatabase::react` match loop — `crnonpl.cc:995-1075`.
pub fn match_dialogue_rule(
    program: &DialogueProgram,
    text: &str,
    situation: DialogueSituationKind,
    ctx: &mut EvalContext<'_>,
) -> Option<RuleMatch> {
    let mut best: Option<RuleMatch> = None;
    let mut max_conditions: i32 = -1;

    for (rule_index, rule) in program.rules.iter().enumerate() {
        let mut match_ok = true;
        let mut short_circuit = false;
        let mut text_ptr = 0usize;
        let mut captures = MatchCaptures::default();

        for pred in &rule.predicates {
            if !match_ok {
                break;
            }
            if matches!(pred, DialoguePredicate::Select { .. }) {
                // C++ loop stops on Match==false before reaching `!` (`crnonpl.cc:1005-1011`).
                short_circuit = true;
                break;
            }
            match pred {
                DialoguePredicate::Situation { kind, .. } => {
                    if !situation_matches(*kind, situation) {
                        match_ok = false;
                    }
                }
                DialoguePredicate::Words { patterns, .. } => {
                    let mut ok = false;
                    for pattern in patterns {
                        if let Some(start) = search_for_word(pattern, &text[text_ptr..]) {
                            // Advance by full pattern length (including `$`), matching C++
                            // `TextPtr = Word + strlen(Pattern)` (`crnonpl.cc:1019`).
                            let abs = text_ptr + start;
                            text_ptr = abs + pattern.len();
                            if text_ptr > text.len() {
                                text_ptr = text.len();
                            }
                            ok = true;
                            break;
                        }
                    }
                    if !ok {
                        match_ok = false;
                    }
                }
                DialoguePredicate::NumericCapture { slot, .. } => {
                    let rest = &text[text_ptr..];
                    if let Some(start) = search_for_number(*slot, rest) {
                        let abs = text_ptr + start;
                        let digits = read_digits(&text[abs..]);
                        let mut value = digits.parse::<i32>().unwrap_or(0);
                        if value > ctx.tuning.numeric_capture_cap {
                            value = ctx.tuning.numeric_capture_cap;
                        }
                        let idx = (*slot as usize).saturating_sub(1);
                        if idx < 2 {
                            captures.values[idx] = value;
                        }
                        // C++ advances by one character only (`Parameter + 1`) — keep that quirk.
                        text_ptr = abs + 1;
                        if text_ptr > text.len() {
                            text_ptr = text.len();
                        }
                    } else {
                        match_ok = false;
                    }
                }
                DialoguePredicate::Expression {
                    expr, op, rhs, ..
                } => {
                    ctx.captures = captures.values;
                    if !eval_compare(expr, *op, rhs, ctx) {
                        match_ok = false;
                    }
                }
                DialoguePredicate::Property { name, .. } => {
                    if !property_matches(*name, situation, ctx) {
                        match_ok = false;
                    }
                }
                DialoguePredicate::Select { .. } => unreachable!("handled above"),
                DialoguePredicate::Custom { .. } => {
                    // Custom predicates require Lua (NPC-7); treat as non-match until then.
                    match_ok = false;
                }
            }
        }

        let cond_count = rule.predicates.len() as i32;
        if short_circuit || (match_ok && cond_count > max_conditions) {
            best = Some(RuleMatch {
                rule_index,
                captures,
            });
            max_conditions = cond_count;
            if short_circuit {
                break;
            }
        }
    }

    best
}

fn situation_matches(pred: DialogueSituation, actual: DialogueSituationKind) -> bool {
    match pred {
        Sit::Address => matches!(
            actual,
            DialogueSituationKind::Address | DialogueSituationKind::AddressQueue
        ),
        Sit::AddressQueue => actual == DialogueSituationKind::AddressQueue,
        Sit::Default => actual == DialogueSituationKind::Default,
        Sit::Busy => actual == DialogueSituationKind::Busy,
        Sit::Vanish => actual == DialogueSituationKind::Vanish,
    }
}

fn property_matches(
    prop: DialogueProperty,
    situation: DialogueSituationKind,
    ctx: &EvalContext<'_>,
) -> bool {
    match prop {
        DialogueProperty::Address => matches!(
            situation,
            DialogueSituationKind::Address | DialogueSituationKind::AddressQueue
        ),
        DialogueProperty::Busy => situation == DialogueSituationKind::Busy,
        DialogueProperty::Vanish => situation == DialogueSituationKind::Vanish,
        DialogueProperty::Male => ctx.player_sex == 1,
        DialogueProperty::Female => ctx.player_sex == 2,
        DialogueProperty::Knight => ctx.player_vocation == PlayerVocationKind::Knight,
        DialogueProperty::Paladin => ctx.player_vocation == PlayerVocationKind::Paladin,
        DialogueProperty::Sorcerer => ctx.player_vocation == PlayerVocationKind::Sorcerer,
        DialogueProperty::Druid => ctx.player_vocation == PlayerVocationKind::Druid,
        DialogueProperty::Premium => ctx.player_premium,
        DialogueProperty::Promoted => ctx.player_promoted,
        DialogueProperty::PvpEnforced => ctx.world_pvp_enforced,
        DialogueProperty::NonPvp => ctx.world_non_pvp,
        DialogueProperty::PzBlock => ctx.player_pz_block,
    }
}

fn read_digits(s: &str) -> &str {
    let end = s
        .as_bytes()
        .iter()
        .position(|c| !c.is_ascii_digit())
        .unwrap_or(s.len());
    &s[..end]
}
