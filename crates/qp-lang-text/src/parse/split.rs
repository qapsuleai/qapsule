//! Cutting the token stream into statements and parsing each on its own.

use super::{stmt, Cursor};
use crate::ast::Located;
use crate::diag::Diag;
use crate::lex::{Tok, Token};
use crate::names::KEYWORDS;

/// The result of parsing a file.
pub(crate) struct Parsed {
    /// The statements that parsed.
    pub stmts: Vec<Located>,
    /// One error per statement that did not.
    pub diags: Vec<Diag>,
    /// Names the failed statements seem to define, with their line: lowering treats
    /// them as known so that one syntax error does not cascade into unknown names.
    pub poisoned: Vec<(String, u32)>,
}

/// Parses every statement; expressions nest at most `max_depth` levels.
pub(crate) fn parse(tokens: &[Token], max_depth: usize) -> Parsed {
    let mut p = Parsed {
        stmts: Vec::new(),
        diags: Vec::new(),
        poisoned: Vec::new(),
    };
    for toks in split(tokens) {
        let mut cur = Cursor {
            toks: &toks,
            pos: 0,
            depth: 0,
            max_depth,
        };
        match stmt::statement(&mut cur) {
            Ok(s) => p.stmts.push(s),
            Err(d) => {
                p.diags.push(*d);
                if let Some(n) = declared(&toks) {
                    p.poisoned.push(n);
                }
            }
        }
    }
    p
}

/// The name a statement defines, judged from its first tokens only.
fn declared(toks: &[Token]) -> Option<(String, u32)> {
    match (toks.first().map(|t| &t.tok), toks.get(1)) {
        (
            Some(Tok::Ident(p)),
            Some(Token {
                tok: Tok::Ident(n),
                span,
            }),
        ) if p == "param" => Some((n.clone(), span.line)),
        (
            Some(Tok::Ident(n)),
            Some(Token {
                tok: Tok::Assign,
                span,
            }),
        ) => Some((n.clone(), span.line)),
        _ => None,
    }
}

/// Cuts the stream at line ends outside braces. Inside an open brace a line that starts
/// with a statement keyword ends the statement too (resynchronization).
fn split(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut out = Vec::new();
    let mut cur: Vec<Token> = Vec::new();
    let mut depth = 0usize;
    for (i, t) in tokens.iter().enumerate() {
        match &t.tok {
            Tok::Newline => {
                let next_kw = matches!(
                    tokens.get(i + 1).map(|t| &t.tok),
                    Some(Tok::Ident(s)) if KEYWORDS.contains(&s.as_str())
                ) && tokens.get(i + 2).map(|t| &t.tok) != Some(&Tok::Assign);
                if depth == 0 || next_kw {
                    depth = 0;
                    if !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                    }
                }
            }
            Tok::LBrace => {
                depth += 1;
                cur.push(t.clone());
            }
            Tok::RBrace => {
                depth = depth.saturating_sub(1);
                cur.push(t.clone());
            }
            _ => cur.push(t.clone()),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
