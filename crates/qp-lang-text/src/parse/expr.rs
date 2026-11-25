//! Expressions (PR §12.1), precedence high to low: `x[n]` / `.out` (postfix), `-x`,
//! `* /`, `+ -`, comparisons (no chaining), `not`, `and`, `or`.

use super::{Cursor, PErr};
use crate::ast::{join, BinOp, Expr, ExprKind};
use crate::diag::{Diag, Reason};
use crate::lex::Tok;
use crate::msg::{Msg, What};
use crate::names::{unsupported_call, FIELDS};

type R = Result<Expr, PErr>;

fn bin(op: BinOp, a: Expr, b: Expr) -> Expr {
    let span = join(a.span, b.span);
    Expr {
        kind: ExprKind::Bin {
            op,
            a: Box::new(a),
            b: Box::new(b),
        },
        span,
    }
}

/// A full expression.
pub(crate) fn expr(c: &mut Cursor) -> R {
    let mut a = and(c)?;
    while c.eat_word("or") {
        let b = and(c)?;
        a = bin(BinOp::Or, a, b);
    }
    Ok(a)
}

fn and(c: &mut Cursor) -> R {
    let mut a = not(c)?;
    while c.eat_word("and") {
        let b = not(c)?;
        a = bin(BinOp::And, a, b);
    }
    Ok(a)
}

fn not(c: &mut Cursor) -> R {
    if c.eat_word("not") {
        let start = c.prev_span();
        let x = not(c)?;
        let span = join(start, x.span);
        return Ok(Expr {
            kind: ExprKind::Not(Box::new(x)),
            span,
        });
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
    Ok(bin(op, a, b))
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
        a = bin(op, a, b);
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
        a = bin(op, a, b);
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
            return Ok(Expr {
                kind: ExprKind::Num { text, neg: true },
                span: join(start, c.prev_span()),
            });
        }
    }
    let x = unary(c)?;
    let span = join(start, x.span);
    Ok(Expr {
        kind: ExprKind::Neg(Box::new(x)),
        span,
    })
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
                x = Expr {
                    kind: ExprKind::Lag {
                        x: Box::new(x),
                        n: Box::new(n),
                    },
                    span,
                };
            }
            Some(Tok::Dot) => {
                c.bump();
                let (out, end) = c.ident(What::OutputName)?;
                let span = join(x.span, end);
                x = Expr {
                    kind: ExprKind::Select {
                        x: Box::new(x),
                        out,
                    },
                    span,
                };
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
    Ok(Expr { kind, span })
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
    Ok(Expr {
        kind: ExprKind::List(items),
        span: join(start, c.prev_span()),
    })
}

fn ident(c: &mut Cursor, name: String) -> R {
    let span = c.span();
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
            return Ok(Expr {
                kind: ExprKind::Field { field: name, tf },
                span: join(span, c.prev_span()),
            });
        }
        _ if c.peek_at(1) == Some(&Tok::LParen) => return call(c, name),
        _ => ExprKind::Name(name),
    };
    c.bump();
    Ok(Expr { kind, span })
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
    Ok(Expr {
        kind: ExprKind::Call {
            name,
            name_span,
            args,
        },
        span: join(name_span, c.prev_span()),
    })
}
