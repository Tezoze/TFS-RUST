//! Typed NPC dialogue program: policy, ordered rules, predicates, actions, expressions.
//!
//! Declarative data only — matching and mutation land in NPC-4/5.
//!
//! Domain: TFS-style `NpcDialogue` registrations (not KeywordHandler).
//! 772 outcomes: `tibia-game-master/src/crnonpl.cc` `TBehaviourDatabase::react`,
//! condition/action tables, `%1` capture, `!` select, `*` repeat.

use crate::npcs::span::SourceSpan;

/// Conversation ownership policy for an NPC definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialoguePolicy {
    /// One active interlocutor + FIFO wait queue (imported 772 default).
    #[default]
    QueuedSingleFocus,
    /// Opt-in per-player sessions for new/TFS content.
    PerPlayer,
}

/// Situation tokens used as dialogue predicates (772 ADDRESS/BUSY/VANISH + runtime DEFAULT/ADDRESSQUEUE).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueSituation {
    Address,
    Default,
    Busy,
    Vanish,
    AddressQueue,
}

/// Player/world property tokens used as boolean predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueProperty {
    Address,
    Busy,
    Vanish,
    Male,
    Female,
    Knight,
    Paladin,
    Sorcerer,
    Druid,
    Premium,
    Promoted,
    PvpEnforced,
    NonPvp,
    PzBlock,
}

/// Session / assignment variables stored on the NPC instance (772 per-NPC, not per-player).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionVar {
    Topic,
    Price,
    Amount,
    /// Item type / generic type slot (`type` in behaviour files).
    Type,
    Data,
}

/// Comparison / arithmetic operators in expression trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprOp {
    Add,
    Sub,
    Mul,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Typed integer expression tree (validated at load; evaluated at runtime later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueExpr {
    Lit(i32),
    Session(SessionVar),
    /// `%1` / `%2` captured numeric word (runtime value).
    Capture {
        slot: u8,
    },
    Hp,
    Burning,
    Poison,
    /// `Count(item)` — item may be a literal id or session `Type`.
    Count {
        item: Box<DialogueExpr>,
    },
    CountMoney,
    Level,
    MagicLevel,
    QuestValue {
        storage_id: u32,
    },
    Random {
        lo: i32,
        hi: i32,
    },
    /// `SpellKnown(spell)` — spell may be a literal id or session `Type`.
    SpellKnown {
        spell: Box<DialogueExpr>,
    },
    /// `SpellLevel(spell)` — spell may be a literal id or session `Type`.
    SpellLevel {
        spell: Box<DialogueExpr>,
    },
    Binary {
        op: ExprOp,
        lhs: Box<DialogueExpr>,
        rhs: Box<DialogueExpr>,
    },
}

/// Opaque custom predicate/action callback id (RegistryKey lives on LuaRuntime).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NpcCallbackId(pub u32);

/// One ordered predicate in a rule's `when` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialoguePredicate {
    Situation {
        kind: DialogueSituation,
        span: SourceSpan,
    },
    Words {
        patterns: Vec<String>,
        span: SourceSpan,
    },
    /// `%1` / `%2` numeric word capture (cap applied at runtime).
    NumericCapture {
        slot: u8,
        span: SourceSpan,
    },
    Expression {
        expr: DialogueExpr,
        op: ExprOp,
        rhs: DialogueExpr,
        span: SourceSpan,
    },
    Property {
        name: DialogueProperty,
        span: SourceSpan,
    },
    /// `!` — select this rule immediately once preceding conditions match.
    Select {
        span: SourceSpan,
    },
    Custom {
        callback_id: NpcCallbackId,
        name: String,
        span: SourceSpan,
    },
}

/// One ordered action in a rule's `actions` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueAction {
    Say {
        text: String,
        span: SourceSpan,
    },
    SetSession {
        var: SessionVar,
        expr: DialogueExpr,
        span: SourceSpan,
    },
    SetHp {
        expr: DialogueExpr,
        span: SourceSpan,
    },
    Idle {
        span: SourceSpan,
    },
    Queue {
        span: SourceSpan,
    },
    Nop {
        span: SourceSpan,
    },
    /// `Create(item[, count])` — item may be literal or session `Type`.
    Create {
        item: DialogueExpr,
        count: DialogueExpr,
        span: SourceSpan,
    },
    /// `Delete(item[, count])` — item may be literal or session `Type`.
    Delete {
        item: DialogueExpr,
        count: DialogueExpr,
        span: SourceSpan,
    },
    CreateMoney {
        amount: DialogueExpr,
        span: SourceSpan,
    },
    DeleteMoney {
        amount: DialogueExpr,
        span: SourceSpan,
    },
    /// 772 `Burning(cycles, param)` action (condition reads use [`DialogueExpr::Burning`]).
    Burning {
        cycles: DialogueExpr,
        param: DialogueExpr,
        span: SourceSpan,
    },
    /// 772 `Poison(cycles, param)` action (condition reads use [`DialogueExpr::Poison`]).
    Poison {
        cycles: DialogueExpr,
        param: DialogueExpr,
        span: SourceSpan,
    },
    EffectMe {
        effect_id: u16,
        span: SourceSpan,
    },
    EffectOpp {
        effect_id: u16,
        span: SourceSpan,
    },
    SetQuestValue {
        storage_id: u32,
        value: DialogueExpr,
        span: SourceSpan,
    },
    Profession {
        /// Vocation id expression (literal or session `Type`).
        vocation: DialogueExpr,
        span: SourceSpan,
    },
    /// `TeachSpell(spell)` — spell may be literal or session `Type`.
    TeachSpell {
        spell: DialogueExpr,
        span: SourceSpan,
    },
    Summon {
        /// Monster type id or name (772 often uses numeric race/monster ids).
        monster: String,
        span: SourceSpan,
    },
    Teleport {
        x: i32,
        y: i32,
        z: i32,
        span: SourceSpan,
    },
    /// Return player to temple / set start; optional explicit coordinates.
    StartPosition {
        pos: Option<(i32, i32, i32)>,
        span: SourceSpan,
    },
    /// `*` — re-execute the previously declared rule's actions.
    RepeatPrevious {
        span: SourceSpan,
    },
    Custom {
        callback_id: NpcCallbackId,
        name: String,
        span: SourceSpan,
    },
}

/// One dialogue rule: ordered predicates + ordered actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueRule {
    pub predicates: Vec<DialoguePredicate>,
    pub actions: Vec<DialogueAction>,
    pub span: SourceSpan,
}

/// Complete dialogue program attached to an NPC definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueProgram {
    pub policy: DialoguePolicy,
    pub rules: Vec<DialogueRule>,
}
