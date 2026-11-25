//! A position in one statement's tokens, and the "expected ..., found ..." errors.

use qp_schema::protocol::Span;

use crate::diag::{Diag, Reason};
use crate::lex::{LexError, Tok, Token};
use crate::msg::{Found, Msg, What};

/// A parse error (boxed: results stay small).
pub(crate) type PErr = Box<Diag>;

/// A position in one statement's tokens.
pub(crate) struct Cursor<'a> {
    pub(super) toks: &'a [Token],
    pub(super) pos: usize,
}

impl Cursor<'_> {
    pub(crate) fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.tok)
    }

    pub(crate) fn peek_at(&self, k: usize) -> Option<&Tok> {
        self.toks.get(self.pos + k).map(|t| &t.tok)
    }

    /// The span of the next token, or a one-character span just after the last one.
    pub(crate) fn span(&self) -> Span {
        match self.toks.get(self.pos) {
            Some(t) => t.span,
            None => self.toks.last().map_or(
                Span {
                    line: 1,
                    col: 1,
                    len: 1,
                },
                |t| Span {
                    line: t.span.line,
                    col: t.span.col + t.span.len,
                    len: 1,
                },
            ),
        }
    }

    /// The span of the token just consumed.
    pub(crate) fn prev_span(&self) -> Span {
        self.toks
            .get(self.pos.wrapping_sub(1))
            .map_or_else(|| self.span(), |t| t.span)
    }

    pub(crate) fn bump(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    pub(crate) fn at_end(&self) -> bool {
        self.pos >= self.toks.len()
    }

    /// Consumes the identifier `word` if it is next.
    pub(crate) fn eat_word(&mut self, word: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Ident(s)) if s == word) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Consumes `tok` if it is next.
    pub(crate) fn eat(&mut self, tok: &Tok) -> bool {
        if self.peek() == Some(tok) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Consumes `tok` or fails with "expected `what`".
    pub(crate) fn expect(&mut self, tok: &Tok, what: What) -> Result<Span, PErr> {
        if self.eat(tok) {
            Ok(self.prev_span())
        } else {
            Err(self.unexpected(what))
        }
    }

    /// Consumes an identifier or fails with "expected `what`".
    pub(crate) fn ident(&mut self, what: What) -> Result<(String, Span), PErr> {
        match self.peek() {
            Some(Tok::Ident(s)) => {
                let s = s.clone();
                self.pos += 1;
                Ok((s, self.prev_span()))
            }
            _ => Err(self.unexpected(what)),
        }
    }

    /// Fails unless the statement is over.
    pub(crate) fn end(&self) -> Result<(), PErr> {
        match self.peek() {
            None => Ok(()),
            Some(Tok::Question) => Err(Box::new(
                Diag::new(Reason::Unsupported, Msg::Ternary, self.span())
                    .hint(Msg::HintNotExpressible),
            )),
            Some(_) => Err(self.unexpected(What::EndOfLine)),
        }
    }

    /// "Expected `what`, found <next>"; a lexical error token reports itself instead.
    pub(crate) fn unexpected(&self, what: What) -> PErr {
        let span = self.span();
        Box::new(match self.toks.get(self.pos) {
            Some(Token {
                tok: Tok::Error(e), ..
            }) => lex_diag(e, span),
            Some(t) => Diag::new(
                Reason::Parse,
                Msg::Expected {
                    what,
                    found: Found::Text(show(&t.tok)),
                },
                span,
            ),
            None => Diag::new(
                Reason::Parse,
                Msg::Expected {
                    what,
                    found: Found::Eol,
                },
                span,
            ),
        })
    }
}

/// The error of a lexical error token.
fn lex_diag(e: &LexError, span: Span) -> Diag {
    match e {
        LexError::Char(c) => {
            let d = Diag::new(Reason::Parse, Msg::Char(*c), span);
            match c {
                '!' => d.hint(Msg::HintNot),
                '&' | '|' => d.hint(Msg::HintAndOr),
                _ => d,
            }
        }
        LexError::Unterminated => Diag::new(Reason::Parse, Msg::Unterminated, span),
        LexError::Escape(c) => Diag::new(Reason::Parse, Msg::Escape(*c), span),
    }
}

/// A token as written.
fn show(t: &Tok) -> String {
    let s = match t {
        Tok::Ident(s) | Tok::Number(s) | Tok::Interval(s) => return s.clone(),
        Tok::Str(s) => return format!("\"{s}\""),
        Tok::LParen => "(",
        Tok::RParen => ")",
        Tok::LBracket => "[",
        Tok::RBracket => "]",
        Tok::LBrace => "{",
        Tok::RBrace => "}",
        Tok::Comma => ",",
        Tok::Dot => ".",
        Tok::DotDot => "..",
        Tok::At => "@",
        Tok::Assign => "=",
        Tok::EqEq => "==",
        Tok::Ne => "!=",
        Tok::Lt => "<",
        Tok::Le => "<=",
        Tok::Gt => ">",
        Tok::Ge => ">=",
        Tok::Plus => "+",
        Tok::Minus => "-",
        Tok::Star => "*",
        Tok::Slash => "/",
        Tok::Percent => "%",
        Tok::Question => "?",
        Tok::Newline => "\\n",
        Tok::Error(_) => "?",
    };
    s.to_owned()
}
