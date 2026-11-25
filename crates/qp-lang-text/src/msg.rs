//! Messages of text errors in English and Turkish (PR §11.1: `lang`, codes are language
//! independent). One variant per situation; `text` writes the sentence.

use qp_schema::protocol::Language;

use crate::names::Missing;

/// What the parser expected at a point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum What {
    Expression,
    Name,
    Assign,
    RParen,
    RBracket,
    LBrace,
    RBrace,
    ParamKind,
    Value,
    Range,
    EndOfLine,
    Statement,
    Str,
    Side,
    Interval,
    Code,
    OutputName,
}

impl What {
    fn text(self, tr: bool) -> &'static str {
        match (self, tr) {
            (What::Expression, false) => "an expression",
            (What::Expression, true) => "bir ifade",
            (What::Name, false) => "a name",
            (What::Name, true) => "bir ad",
            (What::Assign, _) => "`=`",
            (What::RParen, _) => "`)`",
            (What::RBracket, _) => "`]`",
            (What::LBrace, _) => "`{`",
            (What::RBrace, _) => "`}`",
            (What::ParamKind, _) => "`int`, `float`, `bool`",
            (What::Value, false) => "a number, `true` or `false`",
            (What::Value, true) => "bir sayı, `true` ya da `false`",
            (What::Range, false) => "`<min>..<max>`",
            (What::Range, true) => "`<en küçük>..<en büyük>`",
            (What::EndOfLine, false) => "the end of the line",
            (What::EndOfLine, true) => "satır sonu",
            (What::Statement, false) => {
                "a statement (`param`, `name = ...`, `series`, `event`, `signal`, `state`, `reject`)"
            }
            (What::Statement, true) => {
                "bir deyim (`param`, `ad = ...`, `series`, `event`, `signal`, `state`, `reject`)"
            }
            (What::Str, false) => "a quoted string",
            (What::Str, true) => "tırnaklı bir dizgi",
            (What::Side, _) => "`al`, `sat`",
            (What::Interval, false) => "a timeframe (`4h`, `1d`)",
            (What::Interval, true) => "bir periyot (`4h`, `1d`)",
            (What::Code, false) => "a sub-code (`E_LOOKAHEAD`, `E_UNSUPPORTED`, ...)",
            (What::Code, true) => "bir alt kod (`E_LOOKAHEAD`, `E_UNSUPPORTED`, ...)",
            (What::OutputName, false) => "an output name",
            (What::OutputName, true) => "bir çıktı adı",
        }
    }
}

/// What the parser found instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Found {
    Text(String),
    Eol,
}

impl Found {
    fn text(&self, tr: bool) -> String {
        match (self, tr) {
            (Found::Text(s), _) => format!("`{s}`"),
            (Found::Eol, false) => "the end of the line".to_owned(),
            (Found::Eol, true) => "satır sonu".to_owned(),
        }
    }
}

fn missing(m: Missing, tr: bool) -> &'static str {
    match (m, tr) {
        (Missing::Loop, false) => "loops",
        (Missing::Loop, true) => "döngü",
        (Missing::Conditional, false) => "conditional expressions",
        (Missing::Conditional, true) => "koşullu ifade",
        (Missing::Function, false) => "user functions",
        (Missing::Function, true) => "kullanıcı işlevi",
        (Missing::VarState, false) => "state by assignment (`var`)",
        (Missing::VarState, true) => "atamayla durum (`var`)",
        (Missing::Order, false) => "orders",
        (Missing::Order, true) => "emir",
        (Missing::MultiSymbol, false) => "other symbols",
        (Missing::MultiSymbol, true) => "çoklu sembol",
    }
}

/// A message (error sentence or hint).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Msg {
    Char(char),
    Unterminated,
    Escape(char),
    Expected {
        what: What,
        found: Found,
    },
    Chained,
    Unsupported(Missing, String),
    Ternary,
    ParamAfterBinding,
    RejectAlone,
    RejectCode(String),
    BadName(String),
    Reserved(String),
    Duplicate(String),
    UnknownName(String),
    UnknownFunc(String),
    MissingArg {
        func: &'static str,
        arg: &'static str,
    },
    TooMany {
        func: &'static str,
        max: usize,
    },
    IntArg(String),
    FloatArg(String),
    UnknownParam(String),
    ListOnly,
    IntervalOnly,
    TfOnlyField,
    ParamKind(String),
    DupClause(String),
    UnknownClause(String),
    UnclosedBrace,
    HintNot,
    HintAndOr,
    HintTurkish,
    HintLater(u32),
    HintSignature(String),
    HintNumberOrParam,
    HintNotExpressible,
}

impl Msg {
    /// The sentence in `lang`.
    pub(crate) fn text(&self, lang: Language) -> String {
        let tr = lang == Language::Tr;
        if tr {
            self.tr()
        } else {
            self.en()
        }
    }

    fn en(&self) -> String {
        match self {
            Msg::Char(c) => format!("unexpected character `{c}`"),
            Msg::Unterminated => "string without a closing quote".to_owned(),
            Msg::Escape(c) => format!("unknown escape `\\{c}` (use `\\\"`, `\\\\`, `\\n`)"),
            Msg::Expected { what, found } => {
                format!("expected {}, found {}", what.text(false), found.text(false))
            }
            Msg::Chained => "comparisons do not chain; join them with `and`".to_owned(),
            Msg::Unsupported(m, w) => format!("`{w}`: the language has no {}", missing(*m, false)),
            Msg::Ternary => "the language has no conditional expression (`?:`)".to_owned(),
            Msg::ParamAfterBinding => "parameters are written before bindings".to_owned(),
            Msg::RejectAlone => "a `reject` must be the only statement of the file".to_owned(),
            Msg::RejectCode(c) => format!("`{c}` is not a sub-code a refusal can carry"),
            Msg::BadName(n) => {
                format!("`{n}` is not a valid name (letter first, then letters, digits, `_`)")
            }
            Msg::Reserved(n) => format!("`{n}` is a reserved word"),
            Msg::Duplicate(n) => format!("`{n}` is already defined"),
            Msg::UnknownName(n) => format!("unknown name `{n}`"),
            Msg::UnknownFunc(n) => format!("unknown function `{n}`"),
            Msg::MissingArg { func, arg } => format!("`{func}` is missing its argument `{arg}`"),
            Msg::TooMany { func, max } => format!("`{func}` takes at most {max} arguments"),
            Msg::IntArg(c) => format!("{c} must be an integer literal or an `int` parameter"),
            Msg::FloatArg(c) => format!("{c} must be a number or a `float` parameter"),
            Msg::UnknownParam(n) => format!("unknown parameter `{n}`"),
            Msg::ListOnly => "a list `[...]` is only the edges of `bucket`".to_owned(),
            Msg::IntervalOnly => {
                "a bare timeframe is only the second argument of `atr`; write `close@4h`".to_owned()
            }
            Msg::TfOnlyField => "`@` follows a bar field only (`close@1d`)".to_owned(),
            Msg::ParamKind(k) => format!("unknown parameter kind `{k}`"),
            Msg::DupClause(c) => format!("`{c}` is written twice"),
            Msg::UnknownClause(c) => format!("unknown signal clause `{c}`"),
            Msg::UnclosedBrace => "`{` is not closed".to_owned(),
            Msg::HintNot => "write `not` for negation and `!=` for inequality".to_owned(),
            Msg::HintAndOr => "write `and` / `or`".to_owned(),
            Msg::HintTurkish => {
                "Turkish keywords are not accepted; use the English name".to_owned()
            }
            Msg::HintLater(l) => {
                format!("it is defined on line {l}; only earlier lines can be used")
            }
            Msg::HintSignature(s) => format!("signature: {s}"),
            Msg::HintNumberOrParam => {
                "a bound name cannot stand in a number field; declare a `param`".to_owned()
            }
            Msg::HintNotExpressible => {
                "if the request cannot be expressed, write `reject E_UNSUPPORTED \"reason\"`"
                    .to_owned()
            }
        }
    }

    fn tr(&self) -> String {
        match self {
            Msg::Char(c) => format!("beklenmeyen karakter `{c}`"),
            Msg::Unterminated => "kapanış tırnağı olmayan dizgi".to_owned(),
            Msg::Escape(c) => format!("bilinmeyen kaçış `\\{c}` (`\\\"`, `\\\\`, `\\n` kullanın)"),
            Msg::Expected { what, found } => {
                format!(
                    "{} bekleniyordu, {} bulundu",
                    what.text(true),
                    found.text(true)
                )
            }
            Msg::Chained => "karşılaştırmalar zincirlenmez; `and` ile birleştirin".to_owned(),
            Msg::Unsupported(m, w) => format!("`{w}`: dilde {} yok", missing(*m, true)),
            Msg::Ternary => "dilde koşullu ifade (`?:`) yok".to_owned(),
            Msg::ParamAfterBinding => "parametreler bağlamalardan önce yazılır".to_owned(),
            Msg::RejectAlone => "`reject` dosyanın tek deyimi olmalı".to_owned(),
            Msg::RejectCode(c) => format!("`{c}` bir retin taşıyabileceği alt kod değil"),
            Msg::BadName(n) => {
                format!("`{n}` geçerli bir ad değil (önce harf, sonra harf, rakam, `_`)")
            }
            Msg::Reserved(n) => format!("`{n}` ayrılmış bir sözcük"),
            Msg::Duplicate(n) => format!("`{n}` zaten tanımlı"),
            Msg::UnknownName(n) => format!("bilinmeyen ad `{n}`"),
            Msg::UnknownFunc(n) => format!("bilinmeyen işlev `{n}`"),
            Msg::MissingArg { func, arg } => format!("`{func}` işlevinin `{arg}` argümanı eksik"),
            Msg::TooMany { func, max } => format!("`{func}` en çok {max} argüman alır"),
            Msg::IntArg(c) => format!("{c} tamsayı yazımı ya da `int` parametre olmalı"),
            Msg::FloatArg(c) => format!("{c} sayı ya da `float` parametre olmalı"),
            Msg::UnknownParam(n) => format!("bilinmeyen parametre `{n}`"),
            Msg::ListOnly => "liste `[...]` yalnız `bucket` kenarlarıdır".to_owned(),
            Msg::IntervalOnly => {
                "yalın periyot yalnız `atr`'nin ikinci argümanıdır; `close@4h` yazın".to_owned()
            }
            Msg::TfOnlyField => "`@` yalnız bar alanından sonra gelir (`close@1d`)".to_owned(),
            Msg::ParamKind(k) => format!("bilinmeyen parametre türü `{k}`"),
            Msg::DupClause(c) => format!("`{c}` iki kez yazılmış"),
            Msg::UnknownClause(c) => format!("bilinmeyen sinyal yan tümcesi `{c}`"),
            Msg::UnclosedBrace => "`{` kapanmamış".to_owned(),
            Msg::HintNot => "değil için `not`, eşit değil için `!=` yazın".to_owned(),
            Msg::HintAndOr => "`and` / `or` yazın".to_owned(),
            Msg::HintTurkish => {
                "Türkçe anahtar sözcük kabul edilmez; İngilizce adı kullanın".to_owned()
            }
            Msg::HintLater(l) => {
                format!("{l}. satırda tanımlı; yalnız önceki satırlar kullanılabilir")
            }
            Msg::HintSignature(s) => format!("imza: {s}"),
            Msg::HintNumberOrParam => {
                "sayı alanına bağlama adı yazılamaz; `param` tanımlayın".to_owned()
            }
            Msg::HintNotExpressible => {
                "istek ifade edilemiyorsa `reject E_UNSUPPORTED \"gerekçe\"` yazın".to_owned()
            }
        }
    }
}
