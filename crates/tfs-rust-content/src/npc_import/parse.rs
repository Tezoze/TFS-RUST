//! Parse legacy `.npc` / `.ndb` into [`RawNpcFile`] / rule lists.
//!
//! Outcome grammar: cipsoft-772 behaviour files; includes flatten at the
//! include site preserving declaration order for `*`.

use std::path::{Path, PathBuf};

use crate::npc_import::ast::{RawAction, RawCond, RawExpr, RawNpcFile, RawOp, RawOutfit, RawRule};
use crate::npc_import::error::{ImportError, ImportResult};
use crate::npc_import::include::{IncludeStack, read_npc_file, resolve_include};
use crate::npc_import::lex::{Lexer, Token, TokenKind};
use crate::npcs::SourceSpan;

/// Parse a full legacy `.npc` file (metadata + Behaviour).
pub fn parse_npc_file(root: &Path, path: &Path) -> ImportResult<RawNpcFile> {
    let canon = path
        .canonicalize()
        .map_err(|e| ImportError::io(path, e.to_string()))?;
    let src = read_npc_file(&canon)?;
    parse_npc_source(root, &canon, &src)
}

/// Parse NPC source text as if it lived at `path` (for includes / synthetic wraps).
pub fn parse_npc_source(root: &Path, path: &Path, src: &str) -> ImportResult<RawNpcFile> {
    let canon = if path.exists() {
        path.canonicalize()
            .map_err(|e| ImportError::io(path, e.to_string()))?
    } else {
        path.to_path_buf()
    };
    let display = path.display().to_string();
    let tokens = Lexer::tokenize_all(src, &display)?;
    let mut stack = IncludeStack::default();
    stack.push(canon, &SourceSpan::lua(&display, 1))?;
    let mut parser = Parser {
        tokens: &tokens,
        i: 0,
        root: root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
        stack,
    };
    parser.parse_full_npc(display)
}

/// Parse a bare `.ndb` rule fragment (tests / includes).
pub fn parse_ndb_rules(
    root: &Path,
    path: &Path,
    stack: &mut IncludeStack,
) -> ImportResult<Vec<RawRule>> {
    let canon = path
        .canonicalize()
        .map_err(|e| ImportError::io(path, e.to_string()))?;
    let src = read_npc_file(&canon)?;
    let display = path.display().to_string();
    let tokens = Lexer::tokenize_all(&src, &display)?;
    let mut parser = Parser {
        tokens: &tokens,
        i: 0,
        root: root.to_path_buf(),
        stack: IncludeStack::default(),
    };
    // Reuse caller's stack for cycle detection across recursion.
    std::mem::swap(&mut parser.stack, stack);
    let rules = parser.parse_rule_list_until_eof()?;
    std::mem::swap(&mut parser.stack, stack);
    let _ = canon;
    Ok(rules)
}

struct Parser<'a> {
    tokens: &'a [Token],
    i: usize,
    root: PathBuf,
    stack: IncludeStack,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &Token {
        self.tokens
            .get(self.i)
            .or_else(|| self.tokens.last())
            .expect("tokenize_all always ends with Eof")
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if !matches!(t.kind, TokenKind::Eof) {
            self.i += 1;
        }
        t
    }

    fn span(&self) -> SourceSpan {
        self.peek().span.clone()
    }

    fn expect_ident(&mut self) -> ImportResult<(String, SourceSpan)> {
        let t = self.peek().clone();
        match t.kind {
            TokenKind::Ident(s) => {
                self.bump();
                Ok((s, t.span))
            }
            _ => Err(ImportError::spanned(t.span, "expected identifier")),
        }
    }

    fn parse_full_npc(&mut self, source_file: String) -> ImportResult<RawNpcFile> {
        let mut file = RawNpcFile {
            name: None,
            sex: None,
            race: None,
            outfit: None,
            home: None,
            radius: None,
            go_strength: None,
            rules: Vec::new(),
            source_file,
        };

        loop {
            match &self.peek().kind {
                TokenKind::Eof => break,
                TokenKind::Ident(name) => {
                    let key = name.to_ascii_lowercase();
                    match key.as_str() {
                        "name" | "sex" | "race" | "outfit" | "home" | "radius" | "gostrength"
                        | "behaviour" | "behavior" => {
                            self.parse_metadata_field(&mut file)?;
                        }
                        _ => {
                            return Err(ImportError::spanned(
                                self.span(),
                                format!("unexpected top-level identifier {name:?}"),
                            ));
                        }
                    }
                }
                other => {
                    return Err(ImportError::spanned(
                        self.span(),
                        format!("unexpected token at top level: {other:?}"),
                    ));
                }
            }
        }
        Ok(file)
    }

    fn parse_metadata_field(&mut self, file: &mut RawNpcFile) -> ImportResult<()> {
        let (key, span) = self.expect_ident()?;
        let key_l = key.to_ascii_lowercase();
        self.expect_kind(TokenKind::Eq, "expected '='")?;

        match key_l.as_str() {
            "name" => {
                let s = self.expect_string()?;
                file.name = Some(s);
            }
            "sex" => {
                let (id, _) = self.expect_ident()?;
                file.sex = Some(match id.to_ascii_lowercase().as_str() {
                    "male" => 1,
                    "female" => 0,
                    other => {
                        return Err(ImportError::spanned(span, format!("unknown sex {other:?}")));
                    }
                });
            }
            "race" => {
                file.race = Some(self.expect_number()? as u16);
            }
            "radius" => {
                file.radius = Some(self.expect_number()? as u16);
            }
            "gostrength" => {
                file.go_strength = Some(self.expect_number()? as u16);
            }
            "outfit" => {
                file.outfit = Some(self.parse_outfit()?);
            }
            "home" => {
                file.home = Some(self.parse_home()?);
            }
            "behaviour" | "behavior" => {
                self.expect_kind(TokenKind::LBrace, "expected '{' after Behaviour")?;
                file.rules = self.parse_rule_list_until_rbrace()?;
            }
            _ => {
                return Err(ImportError::spanned(
                    span,
                    format!("unknown metadata field {key}"),
                ));
            }
        }
        Ok(())
    }

    fn parse_outfit(&mut self) -> ImportResult<RawOutfit> {
        self.expect_kind(TokenKind::LParen, "expected '('")?;
        let look_type = self.expect_number()? as u16;
        // Forms: (look), (look, lookTypeEx), (look,h-b-l-f)
        if matches!(self.peek().kind, TokenKind::RParen) {
            self.bump();
            return Ok(RawOutfit {
                look_type,
                look_head: 0,
                look_body: 0,
                look_legs: 0,
                look_feet: 0,
                look_type_ex: 0,
            });
        }
        self.expect_kind(TokenKind::Comma, "expected ','")?;
        let second = self.expect_number()?;
        if matches!(self.peek().kind, TokenKind::RParen) {
            self.bump();
            return Ok(RawOutfit {
                look_type,
                look_head: 0,
                look_body: 0,
                look_legs: 0,
                look_feet: 0,
                look_type_ex: second as u16,
            });
        }
        self.expect_kind(TokenKind::Minus, "expected '-' in outfit")?;
        let look_body = self.expect_number()? as u8;
        self.expect_kind(TokenKind::Minus, "expected '-' in outfit")?;
        let look_legs = self.expect_number()? as u8;
        self.expect_kind(TokenKind::Minus, "expected '-' in outfit")?;
        let look_feet = self.expect_number()? as u8;
        self.expect_kind(TokenKind::RParen, "expected ')'")?;
        Ok(RawOutfit {
            look_type,
            look_head: second as u8,
            look_body,
            look_legs,
            look_feet,
            look_type_ex: 0,
        })
    }

    fn parse_home(&mut self) -> ImportResult<(i32, i32, i32)> {
        self.expect_kind(TokenKind::LBracket, "expected '['")?;
        let x = self.expect_number()?;
        self.expect_kind(TokenKind::Comma, "expected ','")?;
        let y = self.expect_number()?;
        self.expect_kind(TokenKind::Comma, "expected ','")?;
        let z = self.expect_number()?;
        self.expect_kind(TokenKind::RBracket, "expected ']'")?;
        Ok((x, y, z))
    }

    fn parse_rule_list_until_rbrace(&mut self) -> ImportResult<Vec<RawRule>> {
        let mut rules = Vec::new();
        loop {
            match &self.peek().kind {
                TokenKind::RBrace => {
                    self.bump();
                    break;
                }
                TokenKind::Eof => {
                    return Err(ImportError::spanned(
                        self.span(),
                        "unterminated Behaviour block",
                    ));
                }
                TokenKind::Include(path) => {
                    let span = self.span();
                    let rel = path.clone();
                    self.bump();
                    rules.extend(self.load_include(&rel, &span)?);
                }
                _ => {
                    rules.push(self.parse_rule()?);
                }
            }
        }
        Ok(rules)
    }

    fn parse_rule_list_until_eof(&mut self) -> ImportResult<Vec<RawRule>> {
        let mut rules = Vec::new();
        loop {
            match &self.peek().kind {
                TokenKind::Eof => break,
                TokenKind::Include(path) => {
                    let span = self.span();
                    let rel = path.clone();
                    self.bump();
                    rules.extend(self.load_include(&rel, &span)?);
                }
                _ => rules.push(self.parse_rule()?),
            }
        }
        Ok(rules)
    }

    fn load_include(&mut self, relative: &str, span: &SourceSpan) -> ImportResult<Vec<RawRule>> {
        let from = self
            .stack
            .current()
            .ok_or_else(|| ImportError::spanned(span.clone(), "include without file context"))?
            .to_path_buf();
        let path = resolve_include(&self.root, &from, relative, span)?;
        self.stack.push(path.clone(), span)?;
        let src = read_npc_file(&path)?;
        let display = path.display().to_string();
        let tokens = Lexer::tokenize_all(&src, &display)?;
        let mut nested = Parser {
            tokens: &tokens,
            i: 0,
            root: self.root.clone(),
            stack: IncludeStack::default(),
        };
        std::mem::swap(&mut nested.stack, &mut self.stack);
        let rules = nested.parse_rule_list_until_eof()?;
        std::mem::swap(&mut nested.stack, &mut self.stack);
        self.stack.pop();
        Ok(rules)
    }

    fn parse_rule(&mut self) -> ImportResult<RawRule> {
        let span = self.span();
        // Allow leading `->` (empty conditions → DEFAULT).
        let conditions = if matches!(self.peek().kind, TokenKind::Arrow) {
            vec![RawCond::Situation("default".into(), span.clone())]
        } else {
            self.parse_condition_list()?
        };
        self.expect_arrow()?;
        let actions = self.parse_action_list()?;
        Ok(RawRule {
            conditions,
            actions,
            span,
        })
    }

    fn expect_arrow(&mut self) -> ImportResult<()> {
        match &self.peek().kind {
            TokenKind::Arrow => {
                self.bump();
                Ok(())
            }
            other => Err(ImportError::spanned(
                self.span(),
                format!("expected '->', found {other:?}"),
            )),
        }
    }

    fn parse_condition_list(&mut self) -> ImportResult<Vec<RawCond>> {
        let mut out = Vec::new();
        loop {
            // Stop before arrow
            if matches!(self.peek().kind, TokenKind::Arrow) {
                break;
            }
            if !out.is_empty() {
                if matches!(self.peek().kind, TokenKind::Comma) {
                    self.bump();
                } else if matches!(self.peek().kind, TokenKind::Arrow) {
                    break;
                } else {
                    // Adjacent conditions without comma are allowed? Corpus always uses commas.
                    // Fall through — try parse next condition; if fail, break.
                }
            }
            if matches!(self.peek().kind, TokenKind::Arrow | TokenKind::Eof) {
                break;
            }
            out.push(self.parse_condition()?);
            if matches!(self.peek().kind, TokenKind::Comma) {
                self.bump();
                continue;
            }
            break;
        }
        if out.is_empty() {
            // 772 allows condition-less rules (DEFAULT situation).
            out.push(RawCond::Situation("default".into(), self.span()));
        }
        Ok(out)
    }

    fn parse_condition(&mut self) -> ImportResult<RawCond> {
        let span = self.span();
        match &self.peek().kind {
            TokenKind::Bang => {
                self.bump();
                Ok(RawCond::Select(span))
            }
            TokenKind::Capture(slot) => {
                let slot = *slot;
                self.bump();
                // Bare capture, or start of compare if op follows? %1 alone is capture.
                // `0<%1` starts with number. `Topic=%1` is assign-like compare.
                Ok(RawCond::Capture(slot, span))
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.bump();
                Ok(RawCond::Words(s, span))
            }
            TokenKind::Ident(_) | TokenKind::Number(_) | TokenKind::LParen => {
                // Could be situation/property or expression compare.
                // Peek: Ident followed by comma/arrow/bang → situation or property.
                // Ident followed by op or '(' → expression.
                if let TokenKind::Ident(name) = &self.peek().kind {
                    let name_l = name.to_ascii_lowercase();
                    let next = self.tokens.get(self.i + 1).map(|t| &t.kind);
                    let alone = matches!(
                        next,
                        Some(
                            TokenKind::Comma | TokenKind::Arrow | TokenKind::Bang | TokenKind::Eof
                        ) | None
                    );
                    if alone {
                        let (id, sp) = self.expect_ident()?;
                        let id_l = id.to_ascii_lowercase();
                        return Ok(match id_l.as_str() {
                            "address" | "busy" | "vanish" | "default" | "addressqueue" => {
                                RawCond::Situation(id_l, sp)
                            }
                            "male" | "female" | "knight" | "paladin" | "sorcerer" | "druid"
                            | "premium" | "promoted" | "pvpenforced" | "nonpvp" | "pzblock" => {
                                RawCond::Property(id_l, sp)
                            }
                            other => {
                                return Err(ImportError::spanned(
                                    sp,
                                    format!("unknown condition identifier {other:?}"),
                                ));
                            }
                        });
                    }
                    // Special: ADDRESS alone already handled. Ident with '(' is call expr.
                    let _ = name_l;
                }
                let lhs = self.parse_expr()?;
                let op = self.parse_cmp_op()?;
                let rhs = self.parse_expr()?;
                Ok(RawCond::Compare { lhs, op, rhs, span })
            }
            other => Err(ImportError::spanned(
                span,
                format!("unexpected condition token {other:?}"),
            )),
        }
    }

    fn parse_action_list(&mut self) -> ImportResult<Vec<RawAction>> {
        let mut out = Vec::new();
        loop {
            match &self.peek().kind {
                TokenKind::Eof | TokenKind::RBrace | TokenKind::Include(_) => break,
                TokenKind::Comma => {
                    // Stray comma — skip.
                    self.bump();
                    continue;
                }
                _ => {
                    // Actions are comma-separated. No comma after an action ends the list;
                    // the next tokens belong to the following rule (declaration order).
                    out.push(self.parse_action()?);
                    if matches!(self.peek().kind, TokenKind::Comma) {
                        self.bump();
                        continue;
                    }
                    break;
                }
            }
        }
        if out.is_empty() {
            return Err(ImportError::spanned(
                self.span(),
                "rule requires at least one action",
            ));
        }
        Ok(out)
    }

    fn parse_action(&mut self) -> ImportResult<RawAction> {
        let span = self.span();
        match &self.peek().kind {
            TokenKind::Star => {
                self.bump();
                Ok(RawAction::Repeat(span))
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.bump();
                Ok(RawAction::Say(s, span))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                let name_l = name.to_ascii_lowercase();
                self.bump();
                // Assign: Topic=…
                if matches!(self.peek().kind, TokenKind::Eq) {
                    self.bump();
                    let value = self.parse_expr()?;
                    return Ok(RawAction::Assign {
                        name: name_l,
                        value,
                        span,
                    });
                }
                // Call: Burning(…
                if matches!(self.peek().kind, TokenKind::LParen) {
                    if name_l == "summon" {
                        return self.parse_summon_call(span);
                    }
                    if name_l == "teleport" {
                        return self.parse_teleport_call(span);
                    }
                    if name_l == "startposition" {
                        return self.parse_start_position_call(span);
                    }
                    let args = self.parse_arg_list()?;
                    return Ok(RawAction::Call {
                        name: name_l,
                        args,
                        span,
                    });
                }
                // Bare ident action
                Ok(RawAction::Ident(name_l, span))
            }
            other => Err(ImportError::spanned(
                span,
                format!("unexpected action token {other:?}"),
            )),
        }
    }

    fn parse_summon_call(&mut self, span: SourceSpan) -> ImportResult<RawAction> {
        self.expect_kind(TokenKind::LParen, "expected '('")?;
        let name = match &self.peek().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.bump();
                s
            }
            TokenKind::Ident(s) => {
                let s = s.clone();
                self.bump();
                s
            }
            TokenKind::Number(n) => {
                let s = n.to_string();
                self.bump();
                s
            }
            _ => {
                return Err(ImportError::spanned(
                    self.span(),
                    "Summon expects string, identifier, or number",
                ));
            }
        };
        self.expect_kind(TokenKind::RParen, "expected ')'")?;
        Ok(RawAction::Summon(name, span))
    }

    fn parse_start_position_call(&mut self, span: SourceSpan) -> ImportResult<RawAction> {
        // StartPosition or StartPosition(x,y,z)
        self.expect_kind(TokenKind::LParen, "expected '('")?;
        if matches!(self.peek().kind, TokenKind::RParen) {
            self.bump();
            return Ok(RawAction::Ident("startposition".into(), span));
        }
        let x = self.expect_number()?;
        self.expect_kind(TokenKind::Comma, "expected ','")?;
        let y = self.expect_number()?;
        self.expect_kind(TokenKind::Comma, "expected ','")?;
        let z = self.expect_number()?;
        self.expect_kind(TokenKind::RParen, "expected ')'")?;
        Ok(RawAction::Call {
            name: "startposition".into(),
            args: vec![
                RawExpr::Lit(x, span.clone()),
                RawExpr::Lit(y, span.clone()),
                RawExpr::Lit(z, span.clone()),
            ],
            span,
        })
    }

    fn parse_teleport_call(&mut self, span: SourceSpan) -> ImportResult<RawAction> {
        self.expect_kind(TokenKind::LParen, "expected '('")?;
        let x = self.expect_number()?;
        self.expect_kind(TokenKind::Comma, "expected ','")?;
        let y = self.expect_number()?;
        self.expect_kind(TokenKind::Comma, "expected ','")?;
        let z = self.expect_number()?;
        self.expect_kind(TokenKind::RParen, "expected ')'")?;
        Ok(RawAction::Teleport { x, y, z, span })
    }

    fn parse_arg_list(&mut self) -> ImportResult<Vec<RawExpr>> {
        self.expect_kind(TokenKind::LParen, "expected '('")?;
        let mut args = Vec::new();
        if matches!(self.peek().kind, TokenKind::RParen) {
            self.bump();
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr()?);
            match &self.peek().kind {
                TokenKind::Comma => {
                    self.bump();
                    continue;
                }
                TokenKind::RParen => {
                    self.bump();
                    break;
                }
                _ => {
                    return Err(ImportError::spanned(
                        self.span(),
                        "expected ',' or ')' in argument list",
                    ));
                }
            }
        }
        Ok(args)
    }

    /// Expression with + - * (left-assoc), no comparison ops inside (those are conditions).
    fn parse_expr(&mut self) -> ImportResult<RawExpr> {
        self.parse_expr_add()
    }

    fn parse_expr_add(&mut self) -> ImportResult<RawExpr> {
        let mut lhs = self.parse_expr_mul()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => RawOp::Add,
                TokenKind::Minus => RawOp::Sub,
                _ => break,
            };
            let span = self.span();
            self.bump();
            let rhs = self.parse_expr_mul()?;
            lhs = RawExpr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_expr_mul(&mut self) -> ImportResult<RawExpr> {
        let mut lhs = self.parse_expr_primary()?;
        loop {
            if !matches!(self.peek().kind, TokenKind::Star) {
                break;
            }
            let span = self.span();
            self.bump();
            let rhs = self.parse_expr_primary()?;
            lhs = RawExpr::Binary {
                op: RawOp::Mul,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_expr_primary(&mut self) -> ImportResult<RawExpr> {
        let span = self.span();
        match &self.peek().kind {
            TokenKind::Minus => {
                self.bump();
                let inner = self.parse_expr_primary()?;
                match inner {
                    RawExpr::Lit(n, s) => Ok(RawExpr::Lit(-n, s)),
                    other => Ok(RawExpr::Binary {
                        op: RawOp::Sub,
                        lhs: Box::new(RawExpr::Lit(0, span.clone())),
                        rhs: Box::new(other),
                        span,
                    }),
                }
            }
            TokenKind::Number(n) => {
                let n = *n;
                self.bump();
                Ok(RawExpr::Lit(n, span))
            }
            TokenKind::Capture(slot) => {
                let slot = *slot;
                self.bump();
                Ok(RawExpr::Capture(slot, span))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.bump();
                if matches!(self.peek().kind, TokenKind::LParen) {
                    let args = self.parse_arg_list()?;
                    Ok(RawExpr::Call {
                        name: name.to_ascii_lowercase(),
                        args,
                        span,
                    })
                } else {
                    Ok(RawExpr::Ident(name.to_ascii_lowercase(), span))
                }
            }
            TokenKind::LParen => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect_kind(TokenKind::RParen, "expected ')'")?;
                Ok(e)
            }
            other => Err(ImportError::spanned(
                span,
                format!("unexpected expression token {other:?}"),
            )),
        }
    }

    fn parse_cmp_op(&mut self) -> ImportResult<RawOp> {
        let span = self.span();
        let op = match self.peek().kind {
            TokenKind::Eq => RawOp::Eq,
            TokenKind::Ne => RawOp::Ne,
            TokenKind::Lt => RawOp::Lt,
            TokenKind::Le => RawOp::Le,
            TokenKind::Gt => RawOp::Gt,
            TokenKind::Ge => RawOp::Ge,
            _ => {
                return Err(ImportError::spanned(span, "expected comparison operator"));
            }
        };
        self.bump();
        Ok(op)
    }

    fn expect_kind(&mut self, kind: TokenKind, msg: &str) -> ImportResult<()> {
        // Compare discriminant only for unit-like kinds
        let ok = std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(&kind);
        if ok {
            self.bump();
            Ok(())
        } else {
            Err(ImportError::spanned(self.span(), msg))
        }
    }

    fn expect_string(&mut self) -> ImportResult<String> {
        match &self.peek().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.bump();
                Ok(s)
            }
            _ => Err(ImportError::spanned(self.span(), "expected string")),
        }
    }

    fn expect_number(&mut self) -> ImportResult<i32> {
        match &self.peek().kind {
            TokenKind::Number(n) => {
                let n = *n;
                self.bump();
                Ok(n)
            }
            TokenKind::Minus => {
                // allow explicit negative via lexer already; leftover
                Err(ImportError::spanned(self.span(), "expected number"))
            }
            _ => Err(ImportError::spanned(self.span(), "expected number")),
        }
    }
}
