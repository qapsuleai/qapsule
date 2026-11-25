//! Messages and hints of program errors in two languages (PR §11.1: `lang`, `en`
//! default, `tr`). Codes and sub-codes do not depend on the language; only the sentence
//! does. Every sentence the checker writes is a variant here.

/// The language of messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    /// English (default).
    #[default]
    En,
    /// Turkish.
    Tr,
}

impl Lang {
    /// `en` or `tr`; anything else is `None`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Lang> {
        match s {
            "en" => Some(Lang::En),
            "tr" => Some(Lang::Tr),
            _ => None,
        }
    }
}

/// A name space or a counted thing, named in both languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Space {
    Node,
    Output,
    Param,
    Dim,
    Bytes,
}

impl Space {
    const fn names(self) -> (&'static str, &'static str) {
        match self {
            Space::Node => ("node", "düğüm"),
            Space::Output => ("output", "çıktı"),
            Space::Param => ("parameter", "parametre"),
            Space::Dim => ("dimension", "boyut"),
            Space::Bytes => ("bytes", "bayt"),
        }
    }
}

/// One sentence with its data.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Msg {
    NotObject(String),
    NotArray(String),
    NotString(String),
    Version(String),
    Missing(String),
    UnknownField(String),
    UnknownOp(String),
    UnknownOutputKind(String),
    UnknownBarField(String),
    BadName(String),
    Duplicate(Space, String),
    UnknownRef(String),
    LaterRef(String),
    NoSuchOutput(String, String),
    UnknownParam(String),
    ParamKind(String, String, &'static str),
    IntExpected(String),
    NumberExpected(String),
    BoolExpected(String),
    OutOfRange(String, String, String),
    WindowLimit(String, i64, u64),
    NegativeLag(i64),
    Partial,
    ParamFieldRange(String, String, String),
    ParamValueRange(String, String, String),
    ParamMinMax(String),
    ParamBoolRange(String),
    TypeMismatch(String, &'static str, &'static str),
    ClockMismatch(String, String),
    AlignNeedsHigher(String),
    OutputClock(String, String),
    NoOutput,
    UnknownTf(String),
    NonCanonicalTf(String, String),
    TfNotAbove(String, String),
    TfUnbound(String),
    TooMany(Space, u64, u64),
    CostLimit(u64, u64),
    Side(String),
    NoRule,
    RuleKind(String),
    TryOnly(String),
    EdgesCount(usize),
    EdgesOrder,
    DimsCount(usize),
    LineOrder,
    MacdOrder(i64, i64),
    RejectCode(String),
    Budget(&'static str, u64, u64),
    ConsecutiveEvents(String),
    UnusedNode(String),
    HintCost,
    HintOmitTf,
    HintLookahead,
    HintTryOnly,
    HintRuleKind,
    HintSide,
}

impl Msg {
    /// The sentence in `lang`.
    pub(crate) fn render(&self, lang: Lang) -> String {
        let (en, tr) = self.texts();
        match lang {
            Lang::En => en,
            Lang::Tr => tr,
        }
    }

    fn texts(&self) -> (String, String) {
        self.texts_0()
            .or_else(|| self.texts_1())
            .or_else(|| self.texts_2())
            .or_else(|| self.texts_3())
            .unwrap_or_default()
    }

    fn texts_0(&self) -> Option<(String, String)> {
        use Msg::*;
        Some(match self {
            NotObject(w) => (
                format!("`{w}` must be an object"),
                format!("`{w}` bir nesne olmalı"),
            ),
            NotArray(w) => (
                format!("`{w}` must be a list"),
                format!("`{w}` bir liste olmalı"),
            ),
            NotString(w) => (
                format!("`{w}` must be a string"),
                format!("`{w}` bir dizgi olmalı"),
            ),
            Version(v) => (
                format!("unknown version `{v}`; this checker reads `qp-ir/0`"),
                format!("bilinmeyen sürüm `{v}`; bu denetçi `qp-ir/0` okur"),
            ),
            Missing(f) => (format!("missing field `{f}`"), format!("`{f}` alanı eksik")),
            UnknownField(f) => (
                format!("unknown field `{f}`"),
                format!("bilinmeyen alan `{f}`"),
            ),
            UnknownOp(o) => (
                format!("unknown function `{o}`"),
                format!("bilinmeyen işlev `{o}`"),
            ),
            UnknownOutputKind(k) => (
                format!("unknown output kind `{k}`"),
                format!("bilinmeyen çıktı türü `{k}`"),
            ),
            UnknownBarField(f) => (
                format!("unknown bar field `{f}`"),
                format!("bilinmeyen bar alanı `{f}`"),
            ),
            BadName(n) => (
                format!("`{n}` is not a valid name here"),
                format!("`{n}` burada geçerli bir ad değil"),
            ),
            Duplicate(space, n) => (
                format!("{} `{n}` is defined twice", space.names().0),
                format!("{} `{n}` iki kez tanımlı", space.names().1),
            ),
            UnknownRef(r) => (
                format!("unknown node `{r}`"),
                format!("bilinmeyen düğüm `{r}`"),
            ),
            LaterRef(r) => (
                format!("node `{r}` comes later; a node may refer only to earlier nodes"),
                format!("`{r}` düğümü sonra geliyor; düğüm yalnız önceki düğümlere başvurur"),
            ),
            NoSuchOutput(n, o) => (
                format!("node `{n}` has no output `{o}`"),
                format!("`{n}` düğümünün `{o}` çıktısı yok"),
            ),
            UnknownParam(k) => (
                format!("unknown parameter `{k}`"),
                format!("bilinmeyen parametre `{k}`"),
            ),
            _ => return None,
        })
    }

    fn texts_1(&self) -> Option<(String, String)> {
        use Msg::*;
        Some(match self {
            ParamKind(k, f, want) => (
                format!("parameter `{k}` cannot fill `{f}`: it needs a {want} parameter"),
                format!("`{k}` parametresi `{f}` alanını dolduramaz: {want} parametre gerekir"),
            ),
            IntExpected(f) => (
                format!("`{f}` must be an integer"),
                format!("`{f}` tamsayı olmalı"),
            ),
            NumberExpected(f) => (
                format!("`{f}` must be a number"),
                format!("`{f}` sayı olmalı"),
            ),
            BoolExpected(f) => (
                format!("`{f}` must be true or false"),
                format!("`{f}` true ya da false olmalı"),
            ),
            OutOfRange(f, got, range) => (
                format!("`{f}` = {got} is outside {range}"),
                format!("`{f}` = {got}, {range} dışında"),
            ),
            WindowLimit(f, got, lim) => (
                format!("`{f}` = {got} is above the window limit {lim}"),
                format!("`{f}` = {got} pencere sınırı {lim}'in üstünde"),
            ),
            NegativeLag(n) => (
                format!("lag {n} reads a future bar"),
                format!("{n} gecikmesi gelecekteki barı okur"),
            ),
            Partial => (
                "an unclosed (partial) bar cannot be read".to_owned(),
                "kapanmamış (partial) bar okunamaz".to_owned(),
            ),
            ParamFieldRange(k, f, range) => (
                format!("parameter `{k}`'s range does not fit `{f}` ({range})"),
                format!("`{k}` parametresinin aralığı `{f}` alanına sığmıyor ({range})"),
            ),
            ParamValueRange(k, got, range) => (
                format!("value {got} of parameter `{k}` is outside {range}"),
                format!("`{k}` parametresinin {got} değeri {range} dışında"),
            ),
            ParamMinMax(k) => (
                format!("parameter `{k}`: min is above max"),
                format!("`{k}` parametresi: min, max'tan büyük"),
            ),
            ParamBoolRange(k) => (
                format!("parameter `{k}` is bool and takes no min or max"),
                format!("`{k}` parametresi bool; min ya da max almaz"),
            ),
            TypeMismatch(f, want, got) => (
                format!("`{f}` must be {want}, got {got}"),
                format!("`{f}` {want} olmalı, {got} geldi"),
            ),
            ClockMismatch(a, b) => (
                format!("inputs on clocks `{a}` and `{b}` combined; bring the higher one down with `align`"),
                format!("`{a}` ve `{b}` saatli girdiler birleşti; üst periyodu `align` ile indirin"),
            ),
            AlignNeedsHigher(c) => (
                format!("`align` needs a higher-timeframe input, got clock `{c}`"),
                format!("`align` üst periyot girdisi ister, `{c}` saati geldi"),
            ),
            _ => return None,
        })
    }

    fn texts_2(&self) -> Option<(String, String)> {
        use Msg::*;
        Some(match self {
            OutputClock(n, c) => (
                format!(
                    "output `{n}` is on clock `{c}`; outputs are on the base clock (use `align`)"
                ),
                format!(
                    "`{n}` çıktısı `{c}` saatinde; çıktılar taban saatindedir (`align` kullanın)"
                ),
            ),
            NoOutput => (
                "a program needs at least one output".to_owned(),
                "programın en az bir çıktısı olmalı".to_owned(),
            ),
            UnknownTf(t) => (
                format!("unknown timeframe `{t}`"),
                format!("bilinmeyen periyot `{t}`"),
            ),
            NonCanonicalTf(t, c) => (
                format!("timeframe `{t}` is not canonical; write `{c}`"),
                format!("`{t}` periyodu kanonik değil; `{c}` yazın"),
            ),
            TfNotAbove(t, b) => (
                format!("timeframe `{t}` is not an allowed higher timeframe of the base `{b}`"),
                format!("`{t}` periyodu `{b}` tabanının izinli bir üst periyodu değil"),
            ),
            TfUnbound(t) => (
                format!("timeframe `{t}` needs a bound series with an interval"),
                format!("`{t}` periyodu aralığı olan bağlı bir seri ister"),
            ),
            TooMany(what, got, lim) => (
                format!("{got} {}, the limit is {lim}", what.names().0),
                format!("{got} {}, sınır {lim}", what.names().1),
            ),
            CostLimit(c, lim) => (
                format!("static cost {c} per bar is above the limit {lim}"),
                format!("bar başı statik maliyet {c}, sınır {lim}'in üstünde"),
            ),
            Side(s) => (
                format!("signal side `{s}` is not supported; a signal is `al` or `sat`"),
                format!("`{s}` sinyal yanı desteklenmiyor; sinyal `al` ya da `sat`tır"),
            ),
            NoRule => (
                "a signal needs an outcome rule with `max_bars`".to_owned(),
                "sinyal `max_bars`'lı bir sonuç kuralı ister".to_owned(),
            ),
            RuleKind(k) => (
                format!("rule kind `{k}` is not supported in qp-ir/0"),
                format!("`{k}` kural türü qp-ir/0'da desteklenmiyor"),
            ),
            TryOnly(n) => (
                format!("output `{n}` cannot enter a document (signal and state run in try only)"),
                format!(
                    "`{n}` çıktısı dokümana giremez (sinyal ve durum tanımı yalnız denede koşar)"
                ),
            ),
            EdgesCount(n) => (
                format!("`edges` has {n} elements; 1 to 8 are allowed"),
                format!("`edges` {n} elemanlı; 1 ile 8 arası olmalı"),
            ),
            EdgesOrder => (
                "`edges` must be strictly increasing".to_owned(),
                "`edges` kesin artan olmalı".to_owned(),
            ),
            DimsCount(n) => (
                format!("a state has {n} dimensions; 1 to 4 are allowed"),
                format!("durum tanımı {n} boyutlu; 1 ile 4 arası olmalı"),
            ),
            _ => return None,
        })
    }

    fn texts_3(&self) -> Option<(String, String)> {
        use Msg::*;
        Some(match self {
            LineOrder => (
                "`line` needs `a.t` < `b.t`".to_owned(),
                "`line` için `a.t` < `b.t` olmalı".to_owned(),
            ),
            MacdOrder(f, s) => (
                format!("`macd` needs fast < slow, got {f} and {s}"),
                format!("`macd` için fast < slow olmalı, {f} ve {s} geldi"),
            ),
            RejectCode(c) => (
                format!("`{c}` is not a program error sub-code"),
                format!("`{c}` bir program hata alt kodu değil"),
            ),
            Budget(key, used, lim) => (
                format!("`{key}`: {used} is above the limit {lim}"),
                format!("`{key}`: {used}, sınır {lim}'in üstünde"),
            ),
            ConsecutiveEvents(n) => (
                format!("event `{n}` may fire on consecutive bars; live notifications merge them"),
                format!("`{n}` olayı art arda barlarda doğabilir; canlı bildirimler birleşir"),
            ),
            UnusedNode(id) => (
                format!("node `{id}` reaches no output (its cost still counts)"),
                format!("`{id}` düğümü hiçbir çıktıya ulaşmıyor (maliyeti yine sayılır)"),
            ),
            HintCost => (
                "stdev and percentrank weigh `period` per bar; shorten windows or drop nodes".to_owned(),
                "stdev ve percentrank bar başına `period` ağırlığındadır; pencereyi kısaltın ya da düğüm çıkarın".to_owned(),
            ),
            HintOmitTf => (
                "omit `tf` to read the base series".to_owned(),
                "taban seriyi okumak için `tf` yazmayın".to_owned(),
            ),
            HintLookahead => (
                "a program reads only closed bars; refuse with `reject E_LOOKAHEAD` if the request needs the future".to_owned(),
                "program yalnız kapanmış barları okur; istek geleceği gerektiriyorsa `reject E_LOOKAHEAD` yazın".to_owned(),
            ),
            HintTryOnly => (
                "run it with try_program".to_owned(),
                "try_program ile çalıştırın".to_owned(),
            ),
            HintRuleKind => (
                "leave `kind` out for the single-stage rule".to_owned(),
                "tek aşamalı kural için `kind` yazmayın".to_owned(),
            ),
            HintSide => (
                "\"hold\" makes no record; write no signal for it".to_owned(),
                "\"tut\" kayıt doğurmaz; onun için sinyal yazmayın".to_owned(),
            ),
            _ => return None,
        })
    }
}
