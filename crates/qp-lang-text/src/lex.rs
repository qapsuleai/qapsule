//! Lexer (PR §12.1): characters to tokens with 1-based line and column in Unicode
//! characters. A bad character becomes an error token; the parser reports it and skips
//! the statement, so lexing never stops early.

use qp_schema::protocol::Span;

/// A lexical error, carried by [`Tok::Error`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LexError {
    /// A character the language does not use.
    Char(char),
    /// A string without its closing quote on the same line.
    Unterminated,
    /// A backslash escape other than `\"`, `\\`, `\n`.
    Escape(char),
    /// A name, number or string longer than the limit (the limit).
    TooLong(usize),
    /// A number in exponent notation (`1e400`), which the language does not have.
    Exponent,
}

/// A token kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    Ident(String),
    /// A number as written (`12`, `1.5`); never signed.
    Number(String),
    /// Digits followed by letters (`4h`, `1d`): a timeframe.
    Interval(String),
    Str(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Dot,
    DotDot,
    At,
    Assign,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Question,
    Newline,
    Error(LexError),
}

/// A token and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub tok: Tok,
    pub span: Span,
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
    max_token: usize,
    out: Vec<Token>,
}

/// Tokenizes `src`. Comments and blank space are dropped; every line end is a
/// [`Tok::Newline`]; a name, number or string over `max_token` characters is an error
/// token.
pub(crate) fn lex(src: &str, max_token: usize) -> Vec<Token> {
    let mut lx = Lexer {
        chars: src.chars().collect(),
        pos: 0,
        line: 1,
        col: 1,
        max_token,
        out: Vec::new(),
    };
    while let Some(c) = lx.peek(0) {
        lx.step(c);
    }
    lx.out
}

impl Lexer {
    fn peek(&self, k: usize) -> Option<char> {
        self.chars.get(self.pos + k).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek(0)?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn push(&mut self, tok: Tok, line: u32, col: u32, len: u32) {
        let long = match &tok {
            Tok::Ident(s) | Tok::Number(s) | Tok::Interval(s) | Tok::Str(s) => {
                s.chars().nth(self.max_token).is_some()
            }
            _ => false,
        };
        let tok = if long {
            Tok::Error(LexError::TooLong(self.max_token))
        } else {
            tok
        };
        self.out.push(Token {
            tok,
            span: Span { line, col, len },
        });
    }

    fn step(&mut self, c: char) {
        let (line, col) = (self.line, self.col);
        if c == '\n' {
            self.bump();
            self.push(Tok::Newline, line, col, 1);
        } else if c == '#' {
            while self.peek(0).is_some_and(|c| c != '\n') {
                self.bump();
            }
        } else if c.is_whitespace() {
            self.bump();
        } else if c.is_alphabetic() || c == '_' {
            let s = self.take_while(|c| c.is_alphanumeric() || c == '_');
            let len = count(&s);
            self.push(Tok::Ident(s), line, col, len);
        } else if c.is_ascii_digit() {
            self.number(line, col);
        } else if c == '"' {
            self.string(line, col);
        } else {
            self.punct(c, line, col);
        }
    }

    fn take_while(&mut self, f: impl Fn(char) -> bool) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek(0).filter(|&c| f(c)) {
            s.push(c);
            self.bump();
        }
        s
    }

    fn number(&mut self, line: u32, col: u32) {
        let mut s = self.take_while(|c| c.is_ascii_digit());
        if self.peek(0) == Some('.') && self.peek(1).is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            s.push('.');
            s.push_str(&self.take_while(|c| c.is_ascii_digit()));
        }
        let sign = matches!(self.peek(1), Some('+' | '-'));
        let exp_digit = self.peek(if sign { 2 } else { 1 });
        if matches!(self.peek(0), Some('e' | 'E')) && exp_digit.is_some_and(|c| c.is_ascii_digit())
        {
            s.push_str(&self.take_while(|c| c.is_ascii_alphanumeric()));
            if sign {
                self.bump();
                s.push_str(&self.take_while(|c| c.is_ascii_digit()));
            }
            let len = count(&s) + u32::from(sign);
            self.push(Tok::Error(LexError::Exponent), line, col, len);
        } else if self.peek(0).is_some_and(char::is_alphabetic) {
            s.push_str(&self.take_while(char::is_alphanumeric));
            let len = count(&s);
            self.push(Tok::Interval(s), line, col, len);
        } else {
            let len = count(&s);
            self.push(Tok::Number(s), line, col, len);
        }
    }

    fn string(&mut self, line: u32, col: u32) {
        self.bump();
        let mut s = String::new();
        loop {
            match self.peek(0) {
                None | Some('\n') => {
                    let len = self.col - col;
                    self.push(Tok::Error(LexError::Unterminated), line, col, len);
                    return;
                }
                Some('"') => {
                    self.bump();
                    let len = self.col - col;
                    self.push(Tok::Str(s), line, col, len);
                    return;
                }
                Some('\\') => {
                    let ecol = self.col;
                    self.bump();
                    match self.bump() {
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some('n') => s.push('\n'),
                        other => {
                            let e = other.unwrap_or('\\');
                            self.push(Tok::Error(LexError::Escape(e)), line, ecol, 2);
                            self.take_while(|c| c != '\n');
                            return;
                        }
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.bump();
                }
            }
        }
    }

    fn punct(&mut self, c: char, line: u32, col: u32) {
        let next = self.peek(1);
        let (tok, len) = match (c, next) {
            ('.', Some('.')) => (Tok::DotDot, 2),
            ('=', Some('=')) => (Tok::EqEq, 2),
            ('!', Some('=')) => (Tok::Ne, 2),
            ('<', Some('=')) => (Tok::Le, 2),
            ('>', Some('=')) => (Tok::Ge, 2),
            ('(', _) => (Tok::LParen, 1),
            (')', _) => (Tok::RParen, 1),
            ('[', _) => (Tok::LBracket, 1),
            (']', _) => (Tok::RBracket, 1),
            ('{', _) => (Tok::LBrace, 1),
            ('}', _) => (Tok::RBrace, 1),
            (',', _) => (Tok::Comma, 1),
            ('.', _) => (Tok::Dot, 1),
            ('@', _) => (Tok::At, 1),
            ('=', _) => (Tok::Assign, 1),
            ('<', _) => (Tok::Lt, 1),
            ('>', _) => (Tok::Gt, 1),
            ('+', _) => (Tok::Plus, 1),
            ('-', _) => (Tok::Minus, 1),
            ('*', _) => (Tok::Star, 1),
            ('/', _) => (Tok::Slash, 1),
            ('%', _) => (Tok::Percent, 1),
            ('?', _) => (Tok::Question, 1),
            _ => (Tok::Error(LexError::Char(c)), 1),
        };
        for _ in 0..len {
            self.bump();
        }
        self.push(tok, line, col, len);
    }
}

/// Length in Unicode characters.
pub(crate) fn count(s: &str) -> u32 {
    u32::try_from(s.chars().count()).unwrap_or(u32::MAX)
}
