//! Private import AST (not public TFS domain types).

use crate::npcs::SourceSpan;

#[derive(Debug, Clone)]
pub struct RawNpcFile {
    pub name: Option<String>,
    pub sex: Option<u8>,
    pub race: Option<u16>,
    pub outfit: Option<RawOutfit>,
    pub home: Option<(i32, i32, i32)>,
    pub radius: Option<u16>,
    pub go_strength: Option<u16>,
    pub rules: Vec<RawRule>,
    pub source_file: String,
}

#[derive(Debug, Clone)]
pub struct RawOutfit {
    pub look_type: u16,
    pub look_head: u8,
    pub look_body: u8,
    pub look_legs: u8,
    pub look_feet: u8,
    /// 772 `(lookType, lookTypeEx)` short form.
    pub look_type_ex: u16,
}

#[derive(Debug, Clone)]
pub struct RawRule {
    pub conditions: Vec<RawCond>,
    pub actions: Vec<RawAction>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum RawCond {
    Situation(String, SourceSpan),
    Words(String, SourceSpan),
    Select(SourceSpan),
    Capture(u8, SourceSpan),
    Property(String, SourceSpan),
    Compare {
        lhs: RawExpr,
        op: RawOp,
        rhs: RawExpr,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone)]
pub enum RawAction {
    Say(String, SourceSpan),
    Repeat(SourceSpan),
    Ident(String, SourceSpan), // Idle, Queue, NOP, StartPosition, CreateMoney, DeleteMoney, …
    Assign {
        name: String,
        value: RawExpr,
        span: SourceSpan,
    },
    Call {
        name: String,
        args: Vec<RawExpr>,
        span: SourceSpan,
    },
    /// `Summon("name")` or `Summon(name)` — string preferred.
    Summon(String, SourceSpan),
    Teleport {
        x: i32,
        y: i32,
        z: i32,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone)]
pub enum RawExpr {
    Lit(i32, SourceSpan),
    Ident(String, SourceSpan),
    Capture(u8, SourceSpan),
    Call {
        name: String,
        args: Vec<RawExpr>,
        span: SourceSpan,
    },
    Binary {
        op: RawOp,
        lhs: Box<RawExpr>,
        rhs: Box<RawExpr>,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawOp {
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
