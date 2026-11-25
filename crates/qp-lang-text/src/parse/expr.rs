//! Expressions (PR §12.1), precedence high to low: `x[n]` / `.out` (postfix), `-x`,
//! `* /`, `+ -`, comparisons (no chaining), `not`, `and`, `or`.

use qp_schema::protocol::Span;

use super::{Cursor, PErr};
use crate::ast::{join, BinOp, Expr, ExprKind};
use crate::diag::{Diag, Reason};
use crate::lex::Tok;
use crate::msg::{Msg, What};
use crate::names::{hard_keyword, unsupported_call, unsupported_word, Missing, FIELDS};

type R = Result<Expr, PErr>;

/// Builds an expression, refusing one taller than the limit: lowering and printing
/// recurse over the tree, so its height is bounded here (`E_TOO_LARGE`).
fn mk(c: &Cursor, kind: ExprKind, span: Span) -> R {
    let e = Expr::new(kind, span);
    if e.depth > c.max_depth {
        return Err(c.too_deep(span));
    }
    Ok(e)
}

fn bin(c: &Cursor, op: BinOp, a: Expr, b: Expr) -> R {
    let span = join(a.span, b.span);
    let kind = ExprKind::Bin {
        op,
        a: Box::new(a),
        b: Box::new(b),
    };
    mk(c, kind, span)
}

/// A full expression (one level of parser recursion).
pub(crate) fn expr(c: &mut Cursor) -> R {
    c.enter()?;
    let r = or(c);
    c.leave();
    r
}

fn or(c: &mut Cursor) -> R {
    let mut a = and(c)?;
    while c.eat_word("or") {
        let b = and(c)?;
        a = bin(c, BinOp::Or, a, b)?;
    }
    Ok(a)
}

fn and(c: &mut Cursor) -> R {
    let mut a = not(c)?;
    while c.eat_word("and") {
        let b = not(c)?;
        a = bin(c, BinOp::And, a, b)?;
    }
    Ok(a)
}

fn not(c: &mut Cursor) -> R {
    if c.eat_word("not") {
        let start = c.prev_span();
        c.enter()?;
        let x = not(c);
        c.leave();
        let x = x?;
        let span = join(start, x.span);
        return mk(c, ExprKind::Not(Box::new(x)), span);
    }
    cmp(c)
}

fn cmp_op(t: Option<&Tok>) -> Option<BinOp> {
    Some(match t? {
        Tok::Lt => BinOp::Lt,
        Tok::Le => BinOp::Le,
        Tok::Gt => BinOp::Gt,
        Tok::Ge => BinOp::Ge,
        Tok::EqEq => BinOp::Eq,
        Tok::Ne => BinOp::Ne,
        _ => return None,
    })
}

fn cmp(c: &mut Cursor) -> R {
    let a = add(c)?;
    let Some(op) = cmp_op(c.peek()) else {
        return Ok(a);
    };
    c.bump();
    let b = add(c)?;
    if cmp_op(c.peek()).is_some() {
        return Err(Box::new(Diag::new(Reason::Parse, Msg::Chained, c.span())));
    }
    bin(c, op, a, b)
}

fn add(c: &mut Cursor) -> R {
    let mut a = mul(c)?;
    loop {
        let op = match c.peek() {
            Some(Tok::Plus) => BinOp::Add,
            Some(Tok::Minus) => BinOp::Sub,
            _ => return Ok(a),
        };
        c.bump();
        let b = mul(c)?;
        a = bin(c, op, a, b)?;
    }
}

fn mul(c: &mut Cursor) -> R {
    let mut a = unary(c)?;
    loop {
        let op = match c.peek() {
            Some(Tok::Star) => BinOp::Mul,
            Some(Tok::Slash) => BinOp::Div,
            _ => return Ok(a),
        };
        c.bump();
        let b = unary(c)?;
        a = bin(c, op, a, b)?;
    }
}

/// `-x`; a minus directly before a number literal (not followed by a postfix) is a
/// negative literal, a `const` in the IR (surface-a rule).
pub(crate) fn unary(c: &mut Cursor) -> R {
    if !c.eat(&Tok::Minus) {
        return postfix(c);
    }
    let start = c.prev_span();
    if let Some(Tok::Number(text)) = c.peek() {
        let postfix_follows = matches!(c.peek_at(1), Some(Tok::LBracket | Tok::Dot | Tok::At));
        if !postfix_follows {
            let text = text.clone();
            c.bump();
            return mk(
                c,
                ExprKind::Num { text, neg: true },
                join(start, c.prev_span()),
            );
        }
    }
    c.enter()?;
    let x = unary(c);
    c.leave();
    let x = x?;
    let span = join(start, x.span);
    mk(c, ExprKind::Neg(Box::new(x)), span)
}

fn postfix(c: &mut Cursor) -> R {
    let mut x = primary(c)?;
    loop {
        match c.peek() {
            Some(Tok::LBracket) => {
                c.bump();
                let n = expr(c)?;
                let end = c.expect(&Tok::RBracket, What::RBracket)?;
                let span = join(x.span, end);
                let kind = ExprKind::Lag {
                    x: Box::new(x),
                    n: Box::new(n),
                };
                x = mk(c, kind, span)?;
            }
            Some(Tok::Dot) => {
                c.bump();
                let (out, end) = c.ident(What::OutputName)?;
                let span = join(x.span, end);
                x = mk(
                    c,
                    ExprKind::Select {
                        x: Box::new(x),
                        out,
                    },
                    span,
                )?;
            }
            Some(Tok::At) => {
                return Err(Box::new(Diag::new(
                    Reason::Parse,
                    Msg::TfOnlyField,
                    c.span(),
                )))
            }
            _ => return Ok(x),
        }
    }
}

fn primary(c: &mut Cursor) -> R {
    let span = c.span();
    let kind = match c.peek().cloned() {
        Some(Tok::Number(text)) => ExprKind::Num { text, neg: false },
        Some(Tok::Interval(s)) => ExprKind::Interval(s),
        Some(Tok::LParen) => {
            c.bump();
            let mut e = expr(c)?;
            let end = c.expect(&Tok::RParen, What::RParen)?;
            e.span = join(span, end);
            return Ok(e);
        }
        Some(Tok::LBracket) => return list(c),
        Some(Tok::Ident(name)) => return ident(c, name),
        _ => return Err(c.unexpected(What::Expression)),
    };
    c.bump();
    mk(c, kind, span)
}

fn list(c: &mut Cursor) -> R {
    let start = c.span();
    c.bump();
    let mut items = Vec::new();
    if !c.eat(&Tok::RBracket) {
        loop {
            items.push(expr(c)?);
            if c.eat(&Tok::Comma) {
                continue;
            }
            c.expect(&Tok::RBracket, What::RBracket)?;
            break;
        }
    }
    mk(c, ExprKind::List(items), join(start, c.prev_span()))
}

fn ident(c: &mut Cursor, name: String) -> R {
    let span = c.span();
    // `if` mid-expression, `request.security(...)`, `strategy.entry(...)`: coded refusals.
    let dotted = c.peek_at(1) == Some(&Tok::Dot)
        && matches!(
            unsupported_word(&name),
            Some(Missing::MultiSymbol | Missing::Order)
        );
    if let Some(m) =
        hard_keyword(&name).or_else(|| dotted.then(|| unsupported_word(&name)).flatten())
    {
        let d = Diag::new(Reason::Unsupported, Msg::Unsupported(m, name), span);
        return Err(Box::new(d.hint(Msg::HintNotExpressible)));
    }
    let kind = match name.as_str() {
        "true" => ExprKind::Bool(true),
        "false" => ExprKind::Bool(false),
        "bar_index" => ExprKind::Builtin("bar_index"),
        "time" => ExprKind::Builtin("time"),
        "and" | "or" | "not" => return Err(c.unexpected(What::Expression)),
        f if FIELDS.contains(&f) => {
            c.bump();
            let tf = if c.eat(&Tok::At) {
                match c.peek().cloned() {
                    Some(Tok::Interval(s)) => {
                        c.bump();
                        Some(s)
                    }
                    _ => return Err(c.unexpected(What::Interval)),
                }
            } else {
                None
            };
            return mk(
                c,
                ExprKind::Field { field: name, tf },
                join(span, c.prev_span()),
            );
        }
        _ if c.peek_at(1) == Some(&Tok::LParen) => return call(c, name),
        _ => ExprKind::Name(name),
    };
    c.bump();
    mk(c, kind, span)
}

fn call(c: &mut Cursor, name: String) -> R {
    let name_span = c.span();
    if let Some(m) = unsupported_call(&name) {
        let d = Diag::new(Reason::Unsupported, Msg::Unsupported(m, name), name_span);
        return Err(Box::new(d.hint(Msg::HintNotExpressible)));
    }
    c.bump();
    c.bump();
    let mut args = Vec::new();
    if !c.eat(&Tok::RParen) {
        loop {
            args.push(expr(c)?);
            if c.eat(&Tok::Comma) {
                continue;
            }
            c.expect(&Tok::RParen, What::RParen)?;
            break;
        }
    }
    let kind = ExprKind::Call {
        name,
        name_span,
        args,
    };
    mk(c, kind, join(name_span, c.prev_span()))
}
