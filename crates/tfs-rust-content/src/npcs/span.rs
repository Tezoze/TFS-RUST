//! Source location for NPC dialogue diagnostics.
//!
//! Attached to rules (and optionally predicates/actions) so loader and runtime
//! errors can cite the authored Lua file and original import span.
//!
//! Domain: TFS-style Lua `NpcType` definitions under `data/npc/scripts/definitions/`.
//! 772 outcome source for eventual importer spans: `tibia-game-master/src/crnonpl.cc`
//! behaviour parser diagnostics (file/line through includes).

/// Source location for a dialogue rule, predicate, or action.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceSpan {
    /// Path of the defining Lua file (or generated definition path).
    pub file: String,
    /// 1-based line in `file`, or `0` when unknown.
    pub line: u32,
    /// 1-based column in `file`, or `0` when unknown.
    pub column: u32,
    /// Original legacy source path when imported (`.npc` / `.ndb`); empty for hand-authored Lua.
    pub original_file: String,
    /// 1-based line in `original_file`, or `0` when unused.
    pub original_line: u32,
}

impl SourceSpan {
    /// Span for a Lua-authored definition (no legacy original).
    pub fn lua(file: impl Into<String>, line: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column: 0,
            original_file: String::new(),
            original_line: 0,
        }
    }

    /// Format for error messages.
    pub fn display(&self) -> String {
        if self.line > 0 {
            format!("{}:{}", self.file, self.line)
        } else {
            self.file.clone()
        }
    }
}
