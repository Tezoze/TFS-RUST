//! Parse Lua `NpcDialogue({ ... })` tables into typed [`DialogueProgram`].
//!
//! Domain: first-party declarative NPC authoring (not KeywordHandler).
//! 772 rule shape: ordered conditions → actions; `!` select; `*` repeat; `%1` capture.
//!
//! Predicate/action arrays must be Lua sequences — hash-only tables are rejected so
//! declaration order is preserved.

use mlua::{Lua, Table, UserData, Value};

use tfs_rust_content::npcs::{
    DialogueAction, DialogueExpr, DialoguePolicy, DialoguePredicate, DialogueProgram,
    DialogueProperty, DialogueRule, DialogueSituation, ExprOp, SessionVar, SourceSpan,
};

/// Parsed dialogue program held as Lua userdata until attached via `npc:dialogue(...)`.
#[derive(Debug, Clone)]
pub struct NpcDialogueProgram(pub DialogueProgram);

impl UserData for NpcDialogueProgram {}

/// Register global `NpcDialogue(table)` constructor.
pub fn register_npc_dialogue(lua: &Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<NpcDialogueProgram>(|_registry| {})?;

    let ctor = lua.create_function(|lua, table: Table| {
        let file = current_script_name(lua);
        let program = parse_dialogue_table(&table, &file)?;
        Ok(NpcDialogueProgram(program))
    })?;
    lua.globals().set("NpcDialogue", ctor)?;
    Ok(())
}

fn current_script_name(lua: &Lua) -> String {
    // Best-effort: prefer chunk name from debug.getinfo(2).
    let Ok(debug): Result<Table, _> = lua.globals().get("debug") else {
        return "<lua>".into();
    };
    let Ok(getinfo): Result<mlua::Function, _> = debug.get("getinfo") else {
        return "<lua>".into();
    };
    // Level 2 = caller of NpcDialogue (the definition script).
    let Ok(info): Result<Value, _> = getinfo.call((2i32, "S")) else {
        return "<lua>".into();
    };
    if let Value::Table(t) = info {
        if let Ok(source) = t.get::<String>("source") {
            let trimmed = source.trim_start_matches('@');
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        if let Ok(short) = t.get::<String>("short_src")
            && !short.is_empty()
            && short != "[C]"
        {
            return short;
        }
    }
    "<lua>".into()
}

fn parse_dialogue_table(table: &Table, file: &str) -> Result<DialogueProgram, mlua::Error> {
    let policy = match table.get::<Value>("policy")? {
        Value::Nil => DialoguePolicy::QueuedSingleFocus,
        Value::String(s) => parse_policy(&s.to_str().map_err(mlua::Error::external)?)?,
        other => {
            return Err(runtime(format!(
                "NpcDialogue: policy must be a string (got {})",
                type_name(&other)
            )));
        }
    };

    let rules_val: Value = table.get("rules")?;
    let rules_table = match rules_val {
        Value::Table(t) => t,
        Value::Nil => {
            return Err(runtime("NpcDialogue: rules array is required".into()));
        }
        other => {
            return Err(runtime(format!(
                "NpcDialogue: rules must be an array table (got {})",
                type_name(&other)
            )));
        }
    };
    require_sequence(&rules_table, "NpcDialogue.rules")?;

    let mut rules = Vec::new();
    for pair in rules_table.sequence_values::<Table>() {
        let rule_table = pair?;
        rules.push(parse_rule(&rule_table, file)?);
    }

    Ok(DialogueProgram { policy, rules })
}

fn parse_rule(table: &Table, file: &str) -> Result<DialogueRule, mlua::Error> {
    let line = table.get::<u32>("line").unwrap_or(0);
    let span = SourceSpan::lua(file, line);

    let when_val: Value = table.get("when")?;
    let when_table = match when_val {
        Value::Table(t) => t,
        Value::Nil => {
            return Err(runtime("dialogue rule: when array is required".into()));
        }
        other => {
            return Err(runtime(format!(
                "dialogue rule: when must be an array (got {})",
                type_name(&other)
            )));
        }
    };
    require_sequence(&when_table, "rule.when")?;

    let actions_val: Value = table.get("actions")?;
    let actions_table = match actions_val {
        Value::Table(t) => t,
        Value::Nil => {
            return Err(runtime("dialogue rule: actions array is required".into()));
        }
        other => {
            return Err(runtime(format!(
                "dialogue rule: actions must be an array (got {})",
                type_name(&other)
            )));
        }
    };
    require_sequence(&actions_table, "rule.actions")?;

    let mut predicates = Vec::new();
    for pair in when_table.sequence_values::<Table>() {
        let pred = pair?;
        predicates.push(parse_predicate(&pred, &span)?);
    }

    let mut actions = Vec::new();
    for pair in actions_table.sequence_values::<Table>() {
        let act = pair?;
        actions.push(parse_action(&act, &span)?);
    }

    Ok(DialogueRule {
        predicates,
        actions,
        span,
    })
}

fn parse_predicate(table: &Table, span: &SourceSpan) -> Result<DialoguePredicate, mlua::Error> {
    // Reject unsupported legacy tokens if someone passes them as keys.
    reject_unsupported_keys(table, &["bless", "town", "string", "promote"])?;

    if let Ok(Value::String(s)) = table.get::<Value>("situation") {
        let kind = parse_situation(&s.to_str().map_err(mlua::Error::external)?)?;
        return Ok(DialoguePredicate::Situation {
            kind,
            span: span.clone(),
        });
    }
    if let Ok(Value::Table(words)) = table.get::<Value>("words") {
        require_sequence(&words, "when.words")?;
        let mut patterns = Vec::new();
        for w in words.sequence_values::<String>() {
            patterns.push(w?);
        }
        return Ok(DialoguePredicate::Words {
            patterns,
            span: span.clone(),
        });
    }
    if let Ok(Value::Boolean(true)) = table.get::<Value>("select") {
        return Ok(DialoguePredicate::Select {
            span: span.clone(),
        });
    }
    if let Ok(Value::Integer(slot)) = table.get::<Value>("capture") {
        return Ok(DialoguePredicate::NumericCapture {
            slot: slot as u8,
            span: span.clone(),
        });
    }
    if let Ok(Value::String(s)) = table.get::<Value>("property") {
        let name = parse_property(&s.to_str().map_err(mlua::Error::external)?)?;
        return Ok(DialoguePredicate::Property {
            name,
            span: span.clone(),
        });
    }
    if let Some(expr_v) = get_non_nil(table, "expr")? {
        let expr = parse_expr(&expr_v)?;
        let op = match table.get::<Value>("op")? {
            Value::String(s) => parse_op(&s.to_str().map_err(mlua::Error::external)?)?,
            _ => {
                return Err(runtime(
                    "when.expr requires string op (=, ~=, <, <=, >, >=)".into(),
                ));
            }
        };
        let rhs = parse_expr(&table.get("rhs")?)?;
        return Ok(DialoguePredicate::Expression {
            expr,
            op,
            rhs,
            span: span.clone(),
        });
    }
    if let Ok(Value::String(s)) = table.get::<Value>("custom") {
        // Callback id filled during register/drain; placeholder 0 until then.
        return Ok(DialoguePredicate::Custom {
            callback_id: tfs_rust_content::npcs::NpcCallbackId(0),
            name: s.to_str().map_err(mlua::Error::external)?.to_string(),
            span: span.clone(),
        });
    }

    Err(runtime(
        "when entry: expected situation, words, select, capture, property, expr, or custom".into(),
    ))
}

fn parse_action(table: &Table, span: &SourceSpan) -> Result<DialogueAction, mlua::Error> {
    reject_unsupported_keys(table, &["bless", "town", "string", "promote"])?;

    if let Ok(Value::String(s)) = table.get::<Value>("say") {
        return Ok(DialogueAction::Say {
            text: s.to_str().map_err(mlua::Error::external)?.to_string(),
            span: span.clone(),
        });
    }
    if let Ok(Value::Table(set)) = table.get::<Value>("set") {
        let var_name: String = set.get("var").map_err(|_| {
            runtime("actions.set: requires var".into())
        })?;
        let var = parse_session_var(&var_name)?;
        let expr = parse_expr(&set.get("value")?)?;
        return Ok(DialogueAction::SetSession {
            var,
            expr,
            span: span.clone(),
        });
    }
    if let Ok(Value::Boolean(true)) = table.get::<Value>("idle") {
        return Ok(DialogueAction::Idle {
            span: span.clone(),
        });
    }
    if let Ok(Value::Boolean(true)) = table.get::<Value>("queue") {
        return Ok(DialogueAction::Queue {
            span: span.clone(),
        });
    }
    if let Ok(Value::Boolean(true)) = table.get::<Value>("nop") {
        return Ok(DialogueAction::Nop {
            span: span.clone(),
        });
    }
    if let Ok(Value::Boolean(true)) = table.get::<Value>("startPosition") {
        return Ok(DialogueAction::StartPosition {
            pos: None,
            span: span.clone(),
        });
    }
    if let Ok(Value::Table(t)) = table.get::<Value>("startPosition") {
        let x: i32 = t.get("x")?;
        let y: i32 = t.get("y")?;
        let z: i32 = t.get("z")?;
        return Ok(DialogueAction::StartPosition {
            pos: Some((x, y, z)),
            span: span.clone(),
        });
    }
    if let Ok(Value::Boolean(true)) = table.get::<Value>("repeatPrevious") {
        return Ok(DialogueAction::RepeatPrevious {
            span: span.clone(),
        });
    }
    // Accept legacy key only if authored with ["repeat"] = true (rare).
    if let Ok(Value::Boolean(true)) = table.get::<Value>("repeat") {
        return Ok(DialogueAction::RepeatPrevious {
            span: span.clone(),
        });
    }
    if let Ok(Value::Table(t)) = table.get::<Value>("create") {
        let item = parse_expr(&t.get("item")?)?;
        let count = match t.get::<Value>("count")? {
            Value::Nil => DialogueExpr::Lit(1),
            v => parse_expr(&v)?,
        };
        return Ok(DialogueAction::Create {
            item,
            count,
            span: span.clone(),
        });
    }
    if let Ok(Value::Table(t)) = table.get::<Value>("delete") {
        let item = parse_expr(&t.get("item")?)?;
        let count = match t.get::<Value>("count")? {
            Value::Nil => DialogueExpr::Lit(1),
            v => parse_expr(&v)?,
        };
        return Ok(DialogueAction::Delete {
            item,
            count,
            span: span.clone(),
        });
    }
    if let Some(v) = get_non_nil(table, "createMoney")? {
        // Boolean true = bare CreateMoney → Amount session (772).
        let amount = match v {
            Value::Boolean(true) => DialogueExpr::Session(SessionVar::Amount),
            other => parse_expr(&other)?,
        };
        return Ok(DialogueAction::CreateMoney {
            amount,
            span: span.clone(),
        });
    }
    if let Some(v) = get_non_nil(table, "deleteMoney")? {
        // Boolean true = bare DeleteMoney → Price session (772).
        let amount = match v {
            Value::Boolean(true) => DialogueExpr::Session(SessionVar::Price),
            other => parse_expr(&other)?,
        };
        return Ok(DialogueAction::DeleteMoney {
            amount,
            span: span.clone(),
        });
    }
    if let Some(v) = get_non_nil(table, "hp")? {
        return Ok(DialogueAction::SetHp {
            expr: parse_expr(&v)?,
            span: span.clone(),
        });
    }
    if let Ok(Value::Table(t)) = table.get::<Value>("burning") {
        let cycles = parse_expr(&t.get("cycles")?)?;
        let param = parse_expr(&t.get("param")?)?;
        return Ok(DialogueAction::Burning {
            cycles,
            param,
            span: span.clone(),
        });
    }
    if let Ok(Value::Table(t)) = table.get::<Value>("poison") {
        let cycles = parse_expr(&t.get("cycles")?)?;
        let param = parse_expr(&t.get("param")?)?;
        return Ok(DialogueAction::Poison {
            cycles,
            param,
            span: span.clone(),
        });
    }
    if let Ok(id) = table.get::<u16>("effectMe") {
        return Ok(DialogueAction::EffectMe {
            effect_id: id,
            span: span.clone(),
        });
    }
    if let Ok(id) = table.get::<u16>("effectOpp") {
        return Ok(DialogueAction::EffectOpp {
            effect_id: id,
            span: span.clone(),
        });
    }
    if let Ok(Value::Table(t)) = table.get::<Value>("setQuestValue") {
        let storage_id: u32 = t.get("id")?;
        let value = parse_expr(&t.get("value")?)?;
        return Ok(DialogueAction::SetQuestValue {
            storage_id,
            value,
            span: span.clone(),
        });
    }
    if let Some(v) = get_non_nil(table, "profession")? {
        return Ok(DialogueAction::Profession {
            vocation: parse_expr(&v)?,
            span: span.clone(),
        });
    }
    if let Some(v) = get_non_nil(table, "teachSpell")? {
        return Ok(DialogueAction::TeachSpell {
            spell: parse_expr(&v)?,
            span: span.clone(),
        });
    }
    if let Ok(name) = table.get::<String>("summon") {
        return Ok(DialogueAction::Summon {
            monster: name,
            span: span.clone(),
        });
    }
    if let Ok(Value::Table(t)) = table.get::<Value>("teleport") {
        let x: i32 = t.get("x")?;
        let y: i32 = t.get("y")?;
        let z: i32 = t.get("z")?;
        return Ok(DialogueAction::Teleport {
            x,
            y,
            z,
            span: span.clone(),
        });
    }
    if let Ok(Value::String(s)) = table.get::<Value>("custom") {
        return Ok(DialogueAction::Custom {
            callback_id: tfs_rust_content::npcs::NpcCallbackId(0),
            name: s.to_str().map_err(mlua::Error::external)?.to_string(),
            span: span.clone(),
        });
    }

    Err(runtime(
        "action entry: unrecognized keys (expected say, set, idle, create, …)".into(),
    ))
}

fn parse_expr(value: &Value) -> Result<DialogueExpr, mlua::Error> {
    match value {
        Value::Integer(n) => Ok(DialogueExpr::Lit(*n as i32)),
        Value::Number(n) => Ok(DialogueExpr::Lit(*n as i32)),
        Value::String(s) => {
            let name = s.to_str().map_err(mlua::Error::external)?;
            parse_expr_ident(&name)
        }
        Value::Table(t) => {
            // Unsupported constructs
            for bad in ["bless", "town", "string", "promote"] {
                if t.contains_key(bad)? {
                    return Err(runtime(format!(
                        "unsupported dialogue construct {bad:?} (not accepted in NpcDialogue)"
                    )));
                }
            }
            if let Ok(Value::Integer(n)) = t.get::<Value>("lit") {
                return Ok(DialogueExpr::Lit(n as i32));
            }
            if let Ok(Value::String(s)) = t.get::<Value>("session") {
                return Ok(DialogueExpr::Session(parse_session_var(
                    &s.to_str().map_err(mlua::Error::external)?,
                )?));
            }
            if let Ok(Value::Integer(slot)) = t.get::<Value>("capture") {
                if !(1..=2).contains(&slot) {
                    return Err(runtime(format!(
                        "capture slot must be 1 or 2 (got {slot})"
                    )));
                }
                return Ok(DialogueExpr::Capture { slot: slot as u8 });
            }
            if let Ok(Value::Boolean(true)) = t.get::<Value>("hp") {
                return Ok(DialogueExpr::Hp);
            }
            if let Ok(Value::Boolean(true)) = t.get::<Value>("burning") {
                return Ok(DialogueExpr::Burning);
            }
            if let Ok(Value::Boolean(true)) = t.get::<Value>("poison") {
                return Ok(DialogueExpr::Poison);
            }
            if let Ok(Value::Boolean(true)) = t.get::<Value>("countMoney") {
                return Ok(DialogueExpr::CountMoney);
            }
            if let Ok(Value::Boolean(true)) = t.get::<Value>("level") {
                return Ok(DialogueExpr::Level);
            }
            if let Ok(Value::Boolean(true)) = t.get::<Value>("magicLevel") {
                return Ok(DialogueExpr::MagicLevel);
            }
            if let Ok(count_v) = t.get::<Value>("count") {
                // Prefer nested expr table/string; bare integer remains supported.
                match count_v {
                    Value::Nil => {}
                    v => {
                        return Ok(DialogueExpr::Count {
                            item: Box::new(parse_expr(&v)?),
                        });
                    }
                }
            }
            if let Ok(storage_id) = t.get::<u32>("questValue") {
                return Ok(DialogueExpr::QuestValue { storage_id });
            }
            if let Some(spell_v) = get_non_nil(t, "spellKnown")? {
                return Ok(DialogueExpr::SpellKnown {
                    spell: Box::new(parse_expr(&spell_v)?),
                });
            }
            if let Some(spell_v) = get_non_nil(t, "spellLevel")? {
                return Ok(DialogueExpr::SpellLevel {
                    spell: Box::new(parse_expr(&spell_v)?),
                });
            }
            if let Ok(Value::Table(r)) = t.get::<Value>("random") {
                let lo: i32 = r.get(1)?;
                let hi: i32 = r.get(2)?;
                return Ok(DialogueExpr::Random { lo, hi });
            }
            if let Ok(Value::Table(bin)) = t.get::<Value>("binary") {
                let op = parse_op(&bin.get::<String>("op")?)?;
                let lhs = parse_expr(&bin.get("lhs")?)?;
                let rhs = parse_expr(&bin.get("rhs")?)?;
                return Ok(DialogueExpr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                });
            }
            Err(runtime("unrecognized expression table".into()))
        }
        other => Err(runtime(format!(
            "expression must be number, string, or table (got {})",
            type_name(other)
        ))),
    }
}

fn parse_expr_ident(name: &str) -> Result<DialogueExpr, mlua::Error> {
    match name.to_ascii_lowercase().as_str() {
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
        "bless" | "town" | "string" | "promote" => Err(runtime(format!(
            "unsupported dialogue construct {name:?}"
        ))),
        _ => Err(runtime(format!("unknown expression identifier {name:?}"))),
    }
}

fn parse_policy(s: &str) -> Result<DialoguePolicy, mlua::Error> {
    match s.to_ascii_lowercase().as_str() {
        "queued_single_focus" | "queued-single-focus" => Ok(DialoguePolicy::QueuedSingleFocus),
        "per_player" | "per-player" => Ok(DialoguePolicy::PerPlayer),
        other => Err(runtime(format!("unknown dialogue policy {other:?}"))),
    }
}

fn parse_situation(s: &str) -> Result<DialogueSituation, mlua::Error> {
    match s.to_ascii_lowercase().as_str() {
        "address" => Ok(DialogueSituation::Address),
        "default" => Ok(DialogueSituation::Default),
        "busy" => Ok(DialogueSituation::Busy),
        "vanish" => Ok(DialogueSituation::Vanish),
        "addressqueue" | "address_queue" | "queued-address" => Ok(DialogueSituation::AddressQueue),
        other => Err(runtime(format!("unknown situation {other:?}"))),
    }
}

fn parse_property(s: &str) -> Result<DialogueProperty, mlua::Error> {
    match s.to_ascii_lowercase().as_str() {
        "address" => Ok(DialogueProperty::Address),
        "busy" => Ok(DialogueProperty::Busy),
        "vanish" => Ok(DialogueProperty::Vanish),
        "male" => Ok(DialogueProperty::Male),
        "female" => Ok(DialogueProperty::Female),
        "knight" => Ok(DialogueProperty::Knight),
        "paladin" => Ok(DialogueProperty::Paladin),
        "sorcerer" => Ok(DialogueProperty::Sorcerer),
        "druid" => Ok(DialogueProperty::Druid),
        "premium" => Ok(DialogueProperty::Premium),
        "promoted" => Ok(DialogueProperty::Promoted),
        "pvpenforced" => Ok(DialogueProperty::PvpEnforced),
        "nonpvp" => Ok(DialogueProperty::NonPvp),
        "pzblock" => Ok(DialogueProperty::PzBlock),
        other => Err(runtime(format!("unknown property {other:?}"))),
    }
}

fn parse_session_var(s: &str) -> Result<SessionVar, mlua::Error> {
    match s.to_ascii_lowercase().as_str() {
        "topic" => Ok(SessionVar::Topic),
        "price" => Ok(SessionVar::Price),
        "amount" => Ok(SessionVar::Amount),
        "type" => Ok(SessionVar::Type),
        "data" => Ok(SessionVar::Data),
        "string" => Err(runtime(
            "unsupported session var \"string\" (not accepted)".into(),
        )),
        other => Err(runtime(format!("unknown session var {other:?}"))),
    }
}

fn parse_op(s: &str) -> Result<ExprOp, mlua::Error> {
    match s {
        "+" => Ok(ExprOp::Add),
        "-" => Ok(ExprOp::Sub),
        "*" => Ok(ExprOp::Mul),
        "=" | "==" => Ok(ExprOp::Eq),
        "~=" | "!=" | "<>" => Ok(ExprOp::Ne),
        "<" => Ok(ExprOp::Lt),
        "<=" => Ok(ExprOp::Le),
        ">" => Ok(ExprOp::Gt),
        ">=" => Ok(ExprOp::Ge),
        other => Err(runtime(format!("unknown operator {other:?}"))),
    }
}

/// Ensure a table is a proper Lua sequence (integer keys 1..n with no holes that
/// would make `sequence_values` drop entries). Reject pure hash tables used as lists.
fn require_sequence(table: &Table, label: &str) -> Result<(), mlua::Error> {
    let len = table.len()?; // border length
    let mut count = 0i64;
    for pair in table.pairs::<Value, Value>() {
        let (k, _) = pair?;
        match k {
            Value::Integer(i) if i >= 1 => count += 1,
            Value::Integer(_) => {
                return Err(runtime(format!(
                    "{label}: array indices must start at 1"
                )));
            }
            Value::String(_) | Value::Number(_) => {
                // Allow optional metadata keys alongside a sequence? Plan says
                // never depend on hash iteration for when/actions — reject mixed.
                return Err(runtime(format!(
                    "{label}: must be an ordered array (got non-integer key); \
                     do not use hash-only tables for when/actions/rules"
                )));
            }
            _ => {
                return Err(runtime(format!(
                    "{label}: must be an ordered array"
                )));
            }
        }
    }
    if count != len {
        return Err(runtime(format!(
            "{label}: array has holes or non-sequence keys (len={len}, pairs={count})"
        )));
    }
    Ok(())
}

fn get_non_nil(table: &Table, key: &str) -> Result<Option<Value>, mlua::Error> {
    match table.get::<Value>(key)? {
        Value::Nil => Ok(None),
        v => Ok(Some(v)),
    }
}

fn reject_unsupported_keys(table: &Table, keys: &[&str]) -> Result<(), mlua::Error> {
    for key in keys {
        if table.contains_key(*key)? {
            return Err(runtime(format!(
                "unsupported dialogue construct {key:?} (not accepted in NpcDialogue)"
            )));
        }
    }
    Ok(())
}

fn runtime(msg: String) -> mlua::Error {
    mlua::Error::runtime(msg)
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Nil => "nil",
        Value::Boolean(_) => "boolean",
        Value::Integer(_) => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Table(_) => "table",
        Value::Function(_) => "function",
        Value::UserData(_) | Value::LightUserData(_) => "userdata",
        Value::Thread(_) => "thread",
        Value::Error(_) => "error",
        _ => "value",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_content::npcs::DialogueSituation;

    fn setup() -> Lua {
        let lua = Lua::new();
        register_npc_dialogue(&lua).expect("reg");
        lua
    }

    #[test]
    fn parses_ordered_rules() {
        let lua = setup();
        let prog: mlua::AnyUserData = lua
            .load(
                r#"
                return NpcDialogue({
                    policy = "queued_single_focus",
                    rules = {
                        {
                            when = {
                                { situation = "address" },
                                { words = { "hi$" } },
                                { select = true }
                            },
                            actions = {
                                { say = "Hello %N!" }
                            }
                        }
                    }
                })
                "#,
            )
            .eval()
            .expect("parse");
        let p = prog.borrow::<NpcDialogueProgram>().unwrap();
        assert_eq!(p.0.policy, DialoguePolicy::QueuedSingleFocus);
        assert_eq!(p.0.rules.len(), 1);
        assert!(matches!(
            p.0.rules[0].predicates[0],
            DialoguePredicate::Situation {
                kind: DialogueSituation::Address,
                ..
            }
        ));
        assert!(matches!(
            &p.0.rules[0].actions[0],
            DialogueAction::Say { text, .. } if text == "Hello %N!"
        ));
    }

    #[test]
    fn rejects_hash_only_when() {
        let lua = setup();
        let err = lua
            .load(
                r#"
                return NpcDialogue({
                    rules = {
                        {
                            when = { situation = "address" },
                            actions = { { say = "x" } }
                        }
                    }
                })
                "#,
            )
            .exec();
        assert!(err.is_err(), "hash-only when must fail");
    }

    #[test]
    fn rejects_unsupported_bless() {
        let lua = setup();
        let err = lua
            .load(
                r#"
                return NpcDialogue({
                    rules = {
                        {
                            when = { { situation = "address" } },
                            actions = { { bless = true } }
                        }
                    }
                })
                "#,
            )
            .exec();
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("bless") || msg.contains("unrecognized"), "{msg}");
    }
}
