//! Statements (PR §12.1): `param`, bindings, `series`, `event`, `signal`, `state`,
//! `reject`; words of constructs the language leaves out get `E_UNSUPPORTED`.

use super::expr::{expr, unary};
use super::{Cursor, PErr};
use crate::ast::{join, Located, OutKind, ParamLit, ParamStmt, SignalClauses, Sp, Stmt};
use crate::diag::{Diag, Reason};
use crate::lex::Tok;
use crate::msg::{Found, Msg, What};
use crate::names::{turkish, unsupported_word, KEYWORDS};

/// One statement.
pub(crate) fn statement(c: &mut Cursor) -> Result<Located, PErr> {
    let first = c.span();
    let stmt = match c.peek().cloned() {
        Some(Tok::Ident(_)) if c.peek_at(1) == Some(&Tok::Assign) => bind(c)?,
        Some(Tok::Ident(w)) => match w.as_str() {
            "param" => param(c)?,
            "series" => output(c, OutKind::Series)?,
            "event" => output(c, OutKind::Event)?,
            "signal" => signal(c)?,
            "state" => state(c)?,
            "reject" => reject(c)?,
            _ => return Err(Box::new(not_statement(c, &w))),
        },
        _ => return Err(c.unexpected(What::Statement)),
    };
    let last = c.prev_span();
    Ok(Located {
        stmt,
        span: join(first, last),
    })
}

fn not_statement(c: &Cursor, w: &str) -> Diag {
    let span = c.span();
    if let Some(m) = unsupported_word(w) {
        return Diag::new(Reason::Unsupported, Msg::Unsupported(m, w.to_owned()), span)
            .hint(Msg::HintNotExpressible);
    }
    match turkish(w).filter(|en| KEYWORDS.contains(en)) {
        Some(en) => Diag::new(
            Reason::Parse,
            Msg::Expected {
                what: What::Statement,
                found: Found::Text(w.to_owned()),
            },
            span,
        )
        .suggest(vec![en.to_owned()])
        .hint(Msg::HintTurkish),
        None => {
            let mut d = c.unexpected(What::Statement);
            d.expected = KEYWORDS.iter().map(|s| (*s).to_owned()).collect();
            *d
        }
    }
}

fn name(c: &mut Cursor, what: What) -> Result<Sp<String>, PErr> {
    let (v, span) = c.ident(what)?;
    Ok(Sp { v, span })
}

fn bind(c: &mut Cursor) -> Result<Stmt, PErr> {
    let name = name(c, What::Name)?;
    c.expect(&Tok::Assign, What::Assign)?;
    let expr = expr(c)?;
    c.end()?;
    Ok(Stmt::Bind { name, expr })
}

fn output(c: &mut Cursor, kind: OutKind) -> Result<Stmt, PErr> {
    c.bump();
    let name = name(c, What::OutputName)?;
    c.expect(&Tok::Assign, What::Assign)?;
    let expr = expr(c)?;
    c.end()?;
    Ok(Stmt::Output { kind, name, expr })
}

fn param_lit(c: &mut Cursor) -> Result<Sp<ParamLit>, PErr> {
    let start = c.span();
    let neg = c.eat(&Tok::Minus);
    let v = match c.peek().cloned() {
        Some(Tok::Number(text)) => ParamLit::Num { text, neg },
        Some(Tok::Ident(w)) if !neg && (w == "true" || w == "false") => ParamLit::Bool(w == "true"),
        _ => return Err(c.unexpected(What::Value)),
    };
    c.bump();
    Ok(Sp {
        v,
        span: join(start, c.prev_span()),
    })
}

fn string(c: &mut Cursor) -> Result<String, PErr> {
    match c.peek().cloned() {
        Some(Tok::Str(s)) => {
            c.bump();
            Ok(s)
        }
        _ => Err(c.unexpected(What::Str)),
    }
}

fn param(c: &mut Cursor) -> Result<Stmt, PErr> {
    c.bump();
    let name = name(c, What::Name)?;
    let kind = self::name(c, What::ParamKind)?;
    c.expect(&Tok::Assign, What::Assign)?;
    let default = param_lit(c)?;
    let range = if c.eat_word("in") {
        let lo = param_lit(c)?;
        c.expect(&Tok::DotDot, What::Range)?;
        Some((lo, param_lit(c)?))
    } else {
        None
    };
    let unit = if c.eat_word("unit") {
        Some(string(c)?)
    } else {
        None
    };
    let description = if matches!(c.peek(), Some(Tok::Str(_))) {
        Some(string(c)?)
    } else {
        None
    };
    c.end()?;
    Ok(Stmt::Param(ParamStmt {
        name,
        kind,
        default,
        range,
        unit,
        description,
    }))
}

fn signal(c: &mut Cursor) -> Result<Stmt, PErr> {
    c.bump();
    let name = name(c, What::OutputName)?;
    c.expect(&Tok::Assign, What::Assign)?;
    let expr = expr(c)?;
    let mut cl = SignalClauses::default();
    while let Some(tok) = c.peek().cloned() {
        let Tok::Ident(word) = tok else {
            return Err(c.unexpected(What::EndOfLine));
        };
        let span = c.span();
        let dup = Diag::new(Reason::Parse, Msg::DupClause(word.clone()), span);
        c.bump();
        match word.as_str() {
            "side" if cl.side.is_none() => cl.side = Some(self::name(c, What::Side)?),
            "target" if cl.target.is_none() => cl.target = Some(pct(c)?),
            "stop" if cl.stop.is_none() => cl.stop = Some(pct(c)?),
            "max_bars" if cl.max_bars.is_none() => cl.max_bars = Some(unary(c)?),
            "source" if cl.source.is_none() => {
                let a = source_value(c)?;
                let b = source_value(c)?;
                cl.source = Some(Sp {
                    v: (a, b),
                    span: join(span, c.prev_span()),
                });
            }
            "side" | "target" | "stop" | "max_bars" | "source" => return Err(Box::new(dup)),
            _ => {
                let d = Diag::new(Reason::Parse, Msg::UnknownClause(word), span);
                return Err(Box::new(
                    d.expected(&["side", "target", "stop", "max_bars", "source"]),
                ));
            }
        }
    }
    Ok(Stmt::Signal {
        name,
        expr,
        clauses: cl,
    })
}

/// A percentage value: a number or parameter name, `%` optional.
fn pct(c: &mut Cursor) -> Result<crate::ast::Expr, PErr> {
    let e = unary(c)?;
    c.eat(&Tok::Percent);
    Ok(e)
}

/// A strategy name or version: a string, or a number written as is.
fn source_value(c: &mut Cursor) -> Result<String, PErr> {
    match c.peek().cloned() {
        Some(Tok::Str(s) | Tok::Number(s)) => {
            c.bump();
            Ok(s)
        }
        _ => Err(c.unexpected(What::Str)),
    }
}

fn state(c: &mut Cursor) -> Result<Stmt, PErr> {
    c.bump();
    let name = name(c, What::OutputName)?;
    let open = c.span();
    c.expect(&Tok::LBrace, What::LBrace)?;
    let mut dims = Vec::new();
    loop {
        if c.eat(&Tok::RBrace) {
            break;
        }
        if c.at_end() {
            return Err(Box::new(Diag::new(Reason::Parse, Msg::UnclosedBrace, open)));
        }
        let dim = self::name(c, What::Name)?;
        c.expect(&Tok::Assign, What::Assign)?;
        dims.push((dim, expr(c)?));
        if !c.eat(&Tok::Comma) && c.peek() != Some(&Tok::RBrace) {
            if c.at_end() {
                return Err(Box::new(Diag::new(Reason::Parse, Msg::UnclosedBrace, open)));
            }
            return Err(c.unexpected(What::RBrace));
        }
    }
    c.end()?;
    Ok(Stmt::State { name, dims })
}

fn reject(c: &mut Cursor) -> Result<Stmt, PErr> {
    c.bump();
    let code = name(c, What::Code)?;
    let reason = string(c)?;
    c.end()?;
    Ok(Stmt::Reject { code, reason })
}
