//! The fixed vocabulary of the text language (PR §12.1, §12.2): function signatures,
//! reserved words, accepted refusal codes, the Turkish → English suggestion table, and
//! `did_you_mean` ranking (`docs/PROTOCOL.md` §13: at most five names).

/// The kind of one function argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arg {
    /// An expression (a node reference in the IR).
    Expr,
    /// An integer field: a number literal or an `int` parameter name.
    Int,
    /// A float field: a number literal or a `float` parameter name.
    Float,
    /// `[e1, ...]`: bucket edges (floats).
    Edges,
    /// A timeframe written bare (`4h`).
    Tf,
}

/// A function: name, IR field per argument, required argument count.
pub(crate) struct Func {
    pub name: &'static str,
    pub args: &'static [(&'static str, Arg)],
    pub required: usize,
}

const XP: &[(&str, Arg)] = &[("x", Arg::Expr), ("period", Arg::Int)];
const AB: &[(&str, Arg)] = &[("a", Arg::Expr), ("b", Arg::Expr)];
const X: &[(&str, Arg)] = &[("x", Arg::Expr)];
const C: &[(&str, Arg)] = &[("c", Arg::Expr)];

const fn f(name: &'static str, args: &'static [(&'static str, Arg)]) -> Func {
    Func {
        name,
        args,
        required: args.len(),
    }
}

/// Every function of PR §12.1, in the spec's order.
pub(crate) const FUNCS: &[Func] = &[
    f("sma", XP),
    f("ema", XP),
    f("rsi", XP),
    f("highest", XP),
    f("lowest", XP),
    f("stdev", XP),
    f("percentrank", XP),
    f(
        "macd",
        &[
            ("x", Arg::Expr),
            ("fast", Arg::Int),
            ("slow", Arg::Int),
            ("signal", Arg::Int),
        ],
    ),
    f(
        "bollinger",
        &[("x", Arg::Expr), ("period", Arg::Int), ("k", Arg::Float)],
    ),
    Func {
        name: "atr",
        args: &[("period", Arg::Int), ("tf", Arg::Tf)],
        required: 1,
    },
    f("min", AB),
    f("max", AB),
    f("abs", X),
    f("cross_above", AB),
    f("cross_below", AB),
    f("streak", C),
    f("bars_since", C),
    f("once", C),
    f("rearm", &[("trigger", Arg::Expr), ("reset", Arg::Expr)]),
    f("cooldown", &[("trigger", Arg::Expr), ("bars", Arg::Int)]),
    f("defined", X),
    f("align", X),
    f(
        "line",
        &[
            ("t1", Arg::Int),
            ("p1", Arg::Float),
            ("t2", Arg::Int),
            ("p2", Arg::Float),
        ],
    ),
    f("bucket", &[("x", Arg::Expr), ("edges", Arg::Edges)]),
];

/// The canonical function names (PR §12.1), for `expected` and suggestions.
pub const FUNCTIONS: &[&str] = &[
    "sma",
    "ema",
    "rsi",
    "highest",
    "lowest",
    "stdev",
    "percentrank",
    "macd",
    "bollinger",
    "atr",
    "min",
    "max",
    "abs",
    "cross_above",
    "cross_below",
    "streak",
    "bars_since",
    "once",
    "rearm",
    "cooldown",
    "defined",
    "align",
    "line",
    "bucket",
];

/// Looks a function up by its canonical name.
pub(crate) fn func(name: &str) -> Option<&'static Func> {
    FUNCS.iter().find(|f| f.name == name)
}

/// Bar fields (PR §4.1).
pub(crate) const FIELDS: &[&str] = &["open", "high", "low", "close", "volume"];

/// Statement keywords.
pub(crate) const KEYWORDS: &[&str] = &["param", "series", "event", "signal", "state", "reject"];

/// Words that cannot name a parameter or binding. Statement keywords can (`param signal`
/// in PR §14): they are keywords only at the start of a statement, before a name.
const RESERVED: &[&str] = &[
    "open",
    "high",
    "low",
    "close",
    "volume",
    "bar_index",
    "time",
    "true",
    "false",
    "and",
    "or",
    "not",
];

/// Whether `s` is a reserved word.
pub(crate) fn reserved(s: &str) -> bool {
    RESERVED.contains(&s)
}

/// Whether `s` can name a parameter or binding in text: `^[A-Za-z][A-Za-z0-9_]{0,63}$`,
/// not reserved (PR §12.1).
pub(crate) fn text_name(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(|c| c.is_ascii_alphabetic())
        && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && s.len() <= 64
        && !reserved(s)
}

/// What a construct the language leaves out is (PR §12.1 "Yok", §16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Missing {
    Loop,
    Conditional,
    Function,
    VarState,
    Order,
    MultiSymbol,
}

/// Statement-leading words of constructs the language leaves out (`E_UNSUPPORTED`).
pub(crate) fn unsupported_word(s: &str) -> Option<Missing> {
    Some(match s {
        "for" | "while" | "loop" | "do" | "break" | "continue" => Missing::Loop,
        "if" | "else" | "switch" | "match" | "case" => Missing::Conditional,
        "def" | "fn" | "function" | "func" | "return" | "lambda" | "method" => Missing::Function,
        "var" | "varip" | "let" | "mut" => Missing::VarState,
        "strategy" | "order" | "buy" | "sell" | "entry" | "exit" => Missing::Order,
        "security" | "request" | "symbol" | "ticker" | "import" => Missing::MultiSymbol,
        _ => return None,
    })
}

/// Function-call names of constructs the language leaves out.
pub(crate) fn unsupported_call(s: &str) -> Option<Missing> {
    Some(match s {
        "iff" | "if" | "ifelse" | "select" | "where" => Missing::Conditional,
        "security" | "request" | "symbol" | "ticker" | "quote" => Missing::MultiSymbol,
        "buy" | "sell" | "order" | "entry" | "exit" | "strategy" => Missing::Order,
        _ => return None,
    })
}

/// Sub-codes a `reject` line may carry (PR §11.2).
pub const REJECT_CODES: &[&str] = &[
    "E_PARSE",
    "E_VERSION",
    "E_UNKNOWN_OP",
    "E_UNKNOWN_FIELD",
    "E_TYPE",
    "E_MISSING_ARG",
    "E_UNKNOWN_TF",
    "E_CLOCK_MISMATCH",
    "E_OUTPUT_CLOCK",
    "E_NO_OUTPUT",
    "E_UNKNOWN_REF",
    "E_UNKNOWN_PARAM",
    "E_DUPLICATE_NAME",
    "E_BAD_PARAM",
    "E_PARAM_RANGE",
    "E_NO_OUTCOME_RULE",
    "E_BAD_RULE",
    "E_NOT_BAR_SERIES",
    "E_PROGRAM_SERIES",
    "E_TRY_ONLY",
    "E_EVENT_OPERAND",
    "E_WINDOW_LIMIT",
    "E_TOO_MANY_NODES",
    "E_TOO_LARGE",
    "E_COST_LIMIT",
    "E_BUDGET",
    "E_LOOKAHEAD",
    "E_UNSUPPORTED",
];

/// Turkish names people write for English canonical words (PR §11.1, §12.2): suggested,
/// never accepted. Keys are ASCII-folded (`ı→i`, `ş→s`, ...), lower case.
const TURKISH: &[(&str, &str)] = &[
    ("ortalama", "sma"),
    ("basit_ortalama", "sma"),
    ("hareketli_ortalama", "sma"),
    ("ussel_ortalama", "ema"),
    ("ussel", "ema"),
    ("gorece_guc", "rsi"),
    ("gog", "rsi"),
    ("en_yuksek", "highest"),
    ("enyuksek", "highest"),
    ("en_dusuk", "lowest"),
    ("endusuk", "lowest"),
    ("standart_sapma", "stdev"),
    ("std_sapma", "stdev"),
    ("sapma", "stdev"),
    ("yuzdelik_sira", "percentrank"),
    ("yuzde_sira", "percentrank"),
    ("bollinger_bandi", "bollinger"),
    ("bant", "bollinger"),
    ("ortalama_gercek_aralik", "atr"),
    ("gercek_aralik", "atr"),
    ("en_kucuk", "min"),
    ("en_buyuk", "max"),
    ("mutlak", "abs"),
    ("yukari_kesisim", "cross_above"),
    ("kesisim_yukari", "cross_above"),
    ("yukari_kesme", "cross_above"),
    ("asagi_kesisim", "cross_below"),
    ("kesisim_asagi", "cross_below"),
    ("asagi_kesme", "cross_below"),
    ("ardisik", "streak"),
    ("seri_sayisi", "streak"),
    ("beri", "bars_since"),
    ("bar_beri", "bars_since"),
    ("bir_kez", "once"),
    ("ilk_kez", "once"),
    ("yeniden_kur", "rearm"),
    ("bekleme", "cooldown"),
    ("soguma", "cooldown"),
    ("tanimli", "defined"),
    ("hizala", "align"),
    ("cizgi", "line"),
    ("kova", "bucket"),
    ("kutu", "bucket"),
    ("kutula", "bucket"),
    ("acilis", "open"),
    ("yuksek", "high"),
    ("dusuk", "low"),
    ("kapanis", "close"),
    ("hacim", "volume"),
    ("zaman", "time"),
    ("bar_sirasi", "bar_index"),
    ("dogru", "true"),
    ("yanlis", "false"),
    ("ve", "and"),
    ("veya", "or"),
    ("degil", "not"),
    ("parametre", "param"),
    ("seri", "series"),
    ("olay", "event"),
    ("sinyal", "signal"),
    ("durum", "state"),
    ("ret", "reject"),
    ("reddet", "reject"),
];

fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ı' | 'İ' => 'i',
            'ş' | 'Ş' => 's',
            'ğ' | 'Ğ' => 'g',
            'ü' | 'Ü' => 'u',
            'ö' | 'Ö' => 'o',
            'ç' | 'Ç' => 'c',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// The English word for a Turkish name, if the table knows it.
pub(crate) fn turkish(s: &str) -> Option<&'static str> {
    let k = fold(s);
    TURKISH.iter().find(|(tr, _)| *tr == k).map(|(_, en)| *en)
}

/// Levenshtein distance over Unicode scalar values.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// At most five candidates close to `given`: the Turkish table's word first (when it is
/// a candidate), then names within edit distance ⌈len/3⌉ (1…3, case folded) or
/// containing it (or contained in it), by distance then name.
pub(crate) fn did_you_mean<'a, I>(given: &str, candidates: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let cands: Vec<&str> = candidates.into_iter().collect();
    let g = fold(given);
    let mut out: Vec<String> = Vec::new();
    if let Some(en) = turkish(given).filter(|en| cands.contains(en)) {
        out.push(en.to_owned());
    }
    let limit = g.chars().count().div_ceil(3).clamp(1, 3);
    let mut scored: Vec<(usize, &str)> = cands
        .iter()
        .filter(|c| **c != given && !out.iter().any(|o| o == *c))
        .filter_map(|c| {
            let fc = fold(c);
            let d = edit_distance(&g, &fc);
            let sub = (g.len() > 2 && fc.contains(g.as_str())) || (fc.len() > 2 && g.contains(&fc));
            (d <= limit || sub).then_some((d, *c))
        })
        .collect();
    scored.sort_unstable();
    scored.dedup();
    out.extend(scored.into_iter().map(|(_, c)| c.to_owned()));
    out.truncate(5);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_list_matches_signatures() {
        let names: Vec<&str> = FUNCS.iter().map(|f| f.name).collect();
        assert_eq!(names, FUNCTIONS);
    }

    #[test]
    fn turkish_names_fold() {
        assert_eq!(turkish("Üssel_Ortalama"), Some("ema"));
        assert_eq!(turkish("kapanış"), Some("close"));
        assert_eq!(turkish("close"), None);
    }
}
