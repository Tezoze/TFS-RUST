//! Dialogue expression evaluation against a read-only player / session snapshot.

use tfs_rust_content::npcs::{DialogueExpr, ExprOp, SessionVar};

use crate::formulas::NpcTuning;

/// Read-only values needed to evaluate predicates / reply substitutions.
pub struct EvalContext<'a> {
    pub topic: i32,
    pub price: i32,
    pub amount: i32,
    pub item_type: i32,
    pub data: i32,
    pub captures: [i32; 2],
    pub player_name: &'a str,
    pub player_hp: i32,
    pub player_level: i32,
    pub player_magic_level: i32,
    pub player_sex: u8,
    pub player_vocation: PlayerVocationKind,
    pub player_premium: bool,
    pub player_promoted: bool,
    pub player_pz_block: bool,
    pub burning: i32,
    pub poison: i32,
    pub money: i32,
    pub inventory_count: &'a dyn Fn(i32) -> i32,
    pub quest_value: &'a dyn Fn(u32) -> i32,
    pub spell_known: &'a dyn Fn(i32) -> i32,
    pub spell_level: &'a dyn Fn(i32) -> i32,
    pub rng: &'a mut dyn FnMut(i32, i32) -> i32,
    pub game_hour: u8,
    pub game_minute: u8,
    pub world_pvp_enforced: bool,
    pub world_non_pvp: bool,
    pub tuning: NpcTuning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerVocationKind {
    None,
    Knight,
    Paladin,
    Sorcerer,
    Druid,
}

pub fn eval_expr(expr: &DialogueExpr, ctx: &mut EvalContext<'_>) -> i32 {
    match expr {
        DialogueExpr::Lit(n) => *n,
        DialogueExpr::Session(v) => session_value(*v, ctx),
        DialogueExpr::Capture { slot } => {
            let idx = (*slot as usize).saturating_sub(1);
            ctx.captures.get(idx).copied().unwrap_or(-1)
        }
        DialogueExpr::Hp => ctx.player_hp,
        DialogueExpr::Burning => ctx.burning,
        DialogueExpr::Poison => ctx.poison,
        DialogueExpr::Count { item } => {
            let id = eval_expr(item, ctx);
            (ctx.inventory_count)(id)
        }
        DialogueExpr::CountMoney => ctx.money,
        DialogueExpr::Level => ctx.player_level,
        DialogueExpr::MagicLevel => ctx.player_magic_level,
        DialogueExpr::QuestValue { storage_id } => (ctx.quest_value)(*storage_id),
        DialogueExpr::Random { lo, hi } => (ctx.rng)(*lo, *hi),
        DialogueExpr::SpellKnown { spell } => {
            let id = eval_expr(spell, ctx);
            (ctx.spell_known)(id)
        }
        DialogueExpr::SpellLevel { spell } => {
            let id = eval_expr(spell, ctx);
            (ctx.spell_level)(id)
        }
        DialogueExpr::Binary { op, lhs, rhs } => {
            let l = eval_expr(lhs, ctx);
            let r = eval_expr(rhs, ctx);
            apply_op(*op, l, r)
        }
    }
}

pub fn eval_compare(
    lhs: &DialogueExpr,
    op: ExprOp,
    rhs: &DialogueExpr,
    ctx: &mut EvalContext<'_>,
) -> bool {
    let l = eval_expr(lhs, ctx);
    let r = eval_expr(rhs, ctx);
    apply_op(op, l, r) != 0
}

fn session_value(v: SessionVar, ctx: &EvalContext<'_>) -> i32 {
    match v {
        SessionVar::Topic => ctx.topic,
        SessionVar::Price => ctx.price,
        SessionVar::Amount => ctx.amount,
        SessionVar::Type => ctx.item_type,
        SessionVar::Data => ctx.data,
    }
}

fn apply_op(op: ExprOp, l: i32, r: i32) -> i32 {
    match op {
        ExprOp::Add => l.saturating_add(r),
        ExprOp::Sub => l.saturating_sub(r),
        ExprOp::Mul => l.saturating_mul(r),
        ExprOp::Eq => i32::from(l == r),
        ExprOp::Ne => i32::from(l != r),
        ExprOp::Lt => i32::from(l < r),
        ExprOp::Le => i32::from(l <= r),
        ExprOp::Gt => i32::from(l > r),
        ExprOp::Ge => i32::from(l >= r),
    }
}

/// Substitute `%N` / `%A` / `%P` / `%T` in reply templates (`FormatNpcResponse`).
pub fn format_npc_response(template: &str, ctx: &EvalContext<'_>) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            let repl = match bytes[i + 1] {
                b'N' | b'n' => Some(ctx.player_name.to_string()),
                b'A' | b'a' => Some(ctx.amount.to_string()),
                b'P' | b'p' => Some(ctx.price.to_string()),
                b'T' | b't' => Some(format_game_time(ctx.game_hour, ctx.game_minute)),
                _ => None,
            };
            if let Some(s) = repl {
                out.push_str(&s);
                i += 2;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn format_game_time(hour: u8, minute: u8) -> String {
    // 772 `FormatNpcResponse` `%T` arm verbatim (`crnonpl.cc:930-937`, `time.cc:43-49`).
    if hour < 12 {
        format!("{hour}:{minute:02} am")
    } else {
        format!("{}:{minute:02} pm", hour - 12)
    }
}
