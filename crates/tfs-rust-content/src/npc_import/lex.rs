//! Lexer for 772 `.npc` / `.ndb` behaviour scripts.
//!
//! Domain: TFS-style offline import only. Outcome grammar from
//! `tibia-game-master` / cipsoft-772 behaviour files (`script.cc` identifier
//! folding, `crnonpl.cc` behaviour parser).

use crate::npc_import::error::{ImportError, ImportResult};
use crate::npcs::SourceSpan;

/// Lexed token with source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ident(String),
    /// Original-cased string contents (dialogue text).
    String(String),
    Number(i32),
    /// `%1` / `%2`
    Capture(u8),
    /// `@"relative/path.ndb"`
    Include(String),
    Arrow, // ->
    Bang,  // !
    Star,  // *
    Comma,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Eof,
}

pub struct Lexer<'a> {
    src: &'a str,
    file: String,
    /// Byte offset.
    pos: usize,
    line: u32,
    col: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str, file: impl Into<String>) -> Self {
        Self {
            src,
            file: file.into(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn next_token(&mut self) -> ImportResult<Token> {
        self.skip_ws_and_comments();
        let start = self.span_here();
        let Some(c) = self.peek() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: start,
            });
        };

        match c {
            ',' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::Comma,
                    span: start,
                })
            }
            '!' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::Bang,
                    span: start,
                })
            }
            '*' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::Star,
                    span: start,
                })
            }
            '+' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::Plus,
                    span: start,
                })
            }
            '-' => {
                self.bump();
                if self.peek() == Some('>') {
                    self.bump();
                    Ok(Token {
                        kind: TokenKind::Arrow,
                        span: start,
                    })
                } else {
                    // Never fuse `-` with digits: outfit `(130,78-0-49-95)` uses `-`
                    // as a separator. Unary minus is handled in the expression parser.
                    Ok(Token {
                        kind: TokenKind::Minus,
                        span: start,
                    })
                }
            }
            '(' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::LParen,
                    span: start,
                })
            }
            ')' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::RParen,
                    span: start,
                })
            }
            '{' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::LBrace,
                    span: start,
                })
            }
            '}' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::RBrace,
                    span: start,
                })
            }
            '[' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::LBracket,
                    span: start,
                })
            }
            ']' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::RBracket,
                    span: start,
                })
            }
            '=' => {
                self.bump();
                Ok(Token {
                    kind: TokenKind::Eq,
                    span: start,
                })
            }
            '<' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    Ok(Token {
                        kind: TokenKind::Le,
                        span: start,
                    })
                } else if self.peek() == Some('>') {
                    self.bump();
                    Ok(Token {
                        kind: TokenKind::Ne,
                        span: start,
                    })
                } else {
                    Ok(Token {
                        kind: TokenKind::Lt,
                        span: start,
                    })
                }
            }
            '>' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    Ok(Token {
                        kind: TokenKind::Ge,
                        span: start,
                    })
                } else {
                    Ok(Token {
                        kind: TokenKind::Gt,
                        span: start,
                    })
                }
            }
            '~' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    Ok(Token {
                        kind: TokenKind::Ne,
                        span: start,
                    })
                } else {
                    Err(ImportError::spanned(start, "expected ~= after '~'"))
                }
            }
            '"' => self.lex_string(start),
            '@' => self.lex_include(start),
            '%' => self.lex_capture(start),
            c if c.is_ascii_digit() => {
                let n = self.lex_number()?;
                Ok(Token {
                    kind: TokenKind::Number(n),
                    span: start,
                })
            }
            c if is_ident_start(c) => {
                let ident = self.lex_ident();
                Ok(Token {
                    kind: TokenKind::Ident(ident),
                    span: start,
                })
            }
            other => Err(ImportError::spanned(
                start,
                format!("unexpected character {other:?}"),
            )),
        }
    }

    pub fn tokenize_all(src: &str, file: impl Into<String>) -> ImportResult<Vec<Token>> {
        let mut lex = Lexer::new(src, file);
        let mut out = Vec::new();
        loop {
            let t = lex.next_token()?;
            let is_eof = matches!(t.kind, TokenKind::Eof);
            out.push(t);
            if is_eof {
                break;
            }
        }
        Ok(out)
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while let Some(c) = self.peek() {
                if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                    self.bump();
                } else {
                    break;
                }
            }
            if self.peek() == Some('#') {
                while let Some(c) = self.peek() {
                    self.bump();
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            break;
        }
    }

    fn lex_string(&mut self, start: SourceSpan) -> ImportResult<Token> {
        self.bump(); // "
        let mut out = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(ImportError::spanned(start, "unterminated string"));
                }
                Some('"') => {
                    self.bump();
                    return Ok(Token {
                        kind: TokenKind::String(out),
                        span: start,
                    });
                }
                Some('\\') => {
                    self.bump();
                    match self.peek() {
                        Some(c) => {
                            self.bump();
                            out.push(c);
                        }
                        None => {
                            return Err(ImportError::spanned(start, "unterminated string escape"));
                        }
                    }
                }
                Some(c) => {
                    self.bump();
                    out.push(c);
                }
            }
        }
    }

    fn lex_include(&mut self, start: SourceSpan) -> ImportResult<Token> {
        self.bump(); // @
        if self.peek() != Some('"') {
            return Err(ImportError::spanned(
                start,
                "expected @\"file\" include",
            ));
        }
        let Token {
            kind: TokenKind::String(path),
            ..
        } = self.lex_string(start.clone())?
        else {
            return Err(ImportError::spanned(start, "internal: include string"));
        };
        Ok(Token {
            kind: TokenKind::Include(path),
            span: start,
        })
    }

    fn lex_capture(&mut self, start: SourceSpan) -> ImportResult<Token> {
        self.bump(); // %
        let Some(c) = self.peek() else {
            return Err(ImportError::spanned(start, "expected digit after '%'"));
        };
        if !c.is_ascii_digit() {
            return Err(ImportError::spanned(start, "expected digit after '%'"));
        }
        self.bump();
        let slot = c.to_digit(10).unwrap() as u8;
        if slot == 0 || slot > 2 {
            return Err(ImportError::spanned(
                start,
                format!("capture slot must be 1 or 2 (got {slot})"),
            ));
        }
        Ok(Token {
            kind: TokenKind::Capture(slot),
            span: start,
        })
    }

    fn lex_number(&mut self) -> ImportResult<i32> {
        let start = self.span_here();
        let mut v: i64 = 0;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.bump();
                v = v * 10 + i64::from(c.to_digit(10).unwrap());
                if v > i64::from(i32::MAX) {
                    return Err(ImportError::spanned(start, "integer overflow"));
                }
            } else {
                break;
            }
        }
        i32::try_from(v).map_err(|_| ImportError::spanned(start, "integer overflow"))
    }

    fn lex_ident(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if is_ident_cont(c) {
                self.bump();
            } else {
                break;
            }
        }
        self.src[start..self.pos].to_string()
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        let len = c.len_utf8();
        self.pos += len;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn span_here(&self) -> SourceSpan {
        SourceSpan {
            file: self.file.clone(),
            line: self.line,
            column: self.col,
            original_file: self.file.clone(),
            original_line: self.line,
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_cont(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '\''
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_simple_rule() {
        let tokens = Lexer::tokenize_all(
            r#"ADDRESS,"hello$",! -> "hi""#,
            "t.npc",
        )
        .unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Ident(ref s) if s == "ADDRESS"));
        assert!(matches!(tokens[2].kind, TokenKind::String(ref s) if s == "hello$"));
        assert!(matches!(tokens[4].kind, TokenKind::Bang));
        assert!(matches!(tokens[5].kind, TokenKind::Arrow));
    }

    #[test]
    fn lexes_include_and_capture() {
        let tokens = Lexer::tokenize_all(r#"@"gen-bank.ndb" %1"#, "t.npc").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Include(ref s) if s == "gen-bank.ndb"));
        assert!(matches!(tokens[1].kind, TokenKind::Capture(1)));
    }

    #[test]
    fn skips_hash_comments() {
        let tokens = Lexer::tokenize_all("# comment\nName", "t.npc").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Ident(ref s) if s == "Name"));
    }
}
