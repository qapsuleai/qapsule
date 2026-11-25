//! Deterministic generators of random programs for the round-trip properties (no new
//! dependency): random *text* (fully parenthesized, with comments, blank lines, odd
//! spacing and a multi-line `state` block) and random *IR* (with or without `_` ids).

use serde_json::{json, Value};

/// xorshift64* with a fixed seed.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn chance(&mut self, pct: u64) -> bool {
        self.next() % 100 < pct
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

const FIELDS: &[&str] = &["open", "high", "low", "close", "volume"];
const TFS: &[&str] = &["4h", "1d", "1w", "15m"];
const WINDOW: &[&str] = &[
    "sma",
    "ema",
    "rsi",
    "highest",
    "lowest",
    "stdev",
    "percentrank",
];
const BIN: &[&str] = &[
    "+", "-", "*", "/", "<", "<=", ">", ">=", "==", "!=", "and", "or",
];
const AB_FUNCS: &[&str] = &["min", "max", "cross_above", "cross_below", "rearm"];
const X_FUNCS: &[&str] = &["abs", "defined", "align", "streak", "bars_since", "once"];

/// Random text programs.
pub struct TextGen<'r> {
    r: &'r mut Rng,
    ints: Vec<String>,
    floats: Vec<String>,
    bools: Vec<String>,
    binds: Vec<String>,
}

impl TextGen<'_> {
    pub fn new(r: &mut Rng) -> TextGen<'_> {
        TextGen {
            r,
            ints: vec![],
            floats: vec![],
            bools: vec![],
            binds: vec![],
        }
    }

    fn sp(&mut self) -> &'static str {
        const SPACES: &[&str] = &["", " ", "  ", "\t"];
        SPACES[self.r.below(SPACES.len())]
    }

    fn int(&mut self) -> String {
        if !self.ints.is_empty() && self.r.chance(30) {
            let i = self.r.below(self.ints.len());
            return self.ints[i].clone();
        }
        let n = self.r.below(60) as i64 - if self.r.chance(5) { 70 } else { 0 };
        n.to_string()
    }

    fn float(&mut self) -> String {
        if !self.floats.is_empty() && self.r.chance(30) {
            let i = self.r.below(self.floats.len());
            return self.floats[i].clone();
        }
        let v = *self
            .r
            .pick(&["2", "0.5", "1.25", "100", "0.0001", "3.75", "-1.5", "0"]);
        v.to_owned()
    }

    fn leaf(&mut self) -> String {
        match self.r.below(9) {
            0 => {
                let f = *self.r.pick(FIELDS);
                if self.r.chance(30) {
                    format!("{f}@{}", self.r.pick(TFS))
                } else {
                    f.to_owned()
                }
            }
            1 => (*self.r.pick(&["bar_index", "time", "true", "false"])).to_owned(),
            2 => (*self
                .r
                .pick(&["1", "30", "2.5", "(-3)", "0.125", "(-0.5)", "70"]))
            .to_owned(),
            3 | 4 if !self.binds.is_empty() => {
                let i = self.r.below(self.binds.len());
                self.binds[i].clone()
            }
            5 => {
                let all: Vec<String> = self
                    .ints
                    .iter()
                    .chain(&self.floats)
                    .chain(&self.bools)
                    .cloned()
                    .collect();
                if all.is_empty() {
                    "close".to_owned()
                } else {
                    self.r.pick(&all).clone()
                }
            }
            _ => (*self.r.pick(FIELDS)).to_owned(),
        }
    }

    fn call(&mut self, d: usize) -> String {
        match self.r.below(9) {
            0 | 1 => {
                let f = *self.r.pick(WINDOW);
                format!("{f}({}, {})", self.expr(d - 1), self.int())
            }
            2 => {
                let sel = *self.r.pick(&["", ".macd", ".signal", ".histogram"]);
                let (x, a, b, c) = (self.expr(d - 1), self.int(), self.int(), self.int());
                format!("macd({x}, {a}, {b}, {c}){sel}")
            }
            3 => {
                let sel = *self
                    .r
                    .pick(&["", ".upper", ".middle", ".lower", ".bandwidth"]);
                let (x, p, k) = (self.expr(d - 1), self.int(), self.float());
                format!("bollinger({x},{}{p}, {k}){sel}", self.sp())
            }
            4 => {
                let p = self.int();
                if self.r.chance(50) {
                    format!("atr({p}, {})", self.r.pick(TFS))
                } else {
                    format!("atr({p})")
                }
            }
            5 => {
                let f = *self.r.pick(AB_FUNCS);
                format!("{f}({}, {})", self.expr(d - 1), self.expr(d - 1))
            }
            6 => {
                let f = *self.r.pick(X_FUNCS);
                format!("{f}({})", self.expr(d - 1))
            }
            7 => format!("cooldown({}, {})", self.expr(d - 1), self.int()),
            _ => {
                if self.r.chance(50) {
                    let (t1, p1, t2, p2) = (self.int(), self.float(), self.int(), self.float());
                    format!("line({t1}, {p1}, {t2}, {p2})")
                } else {
                    format!(
                        "bucket({}, [{}, {}])",
                        self.expr(d - 1),
                        self.float(),
                        self.float()
                    )
                }
            }
        }
    }

    /// A fully parenthesized expression of depth at most `d`.
    pub fn expr(&mut self, d: usize) -> String {
        if d == 0 || self.r.chance(25) {
            return self.leaf();
        }
        match self.r.below(6) {
            0 => format!("(-{})", self.expr(d - 1)),
            1 => format!("(not {})", self.expr(d - 1)),
            2 | 3 => {
                let op = *self.r.pick(BIN);
                let (a, b) = (self.expr(d - 1), self.expr(d - 1));
                let (s1, s2) = (self.sp(), self.sp());
                format!("({a}{s1} {op} {s2}{b})")
            }
            4 => format!("({})[{}]", self.expr(d - 1), self.int()),
            _ => self.call(d),
        }
    }

    fn params(&mut self, lines: &mut Vec<String>) {
        for i in 0..self.r.below(4) {
            let name = format!("p{i}");
            let line = match self.r.below(3) {
                0 => {
                    self.ints.push(name.clone());
                    format!(
                        "param {name} int = {} in 1..{}",
                        1 + self.r.below(9),
                        10 + self.r.below(90)
                    )
                }
                1 => {
                    self.floats.push(name.clone());
                    let unit = if self.r.chance(50) {
                        " unit \"ratio\""
                    } else {
                        ""
                    };
                    format!("param {name} float = 1.5 in -2..2.5{unit} \"a \\\"k\\\" \\\\ değer\"")
                }
                _ => {
                    self.bools.push(name.clone());
                    format!("param {name} bool = {}", self.r.chance(50))
                }
            };
            lines.push(line);
        }
    }

    fn output(&mut self, i: usize) -> String {
        match self.r.below(5) {
            0 | 1 => format!("series o{i} = {}", self.expr(3)),
            2 => format!("event o{i} ={}{}", self.sp(), self.expr(3)),
            3 => {
                let mut cl = vec![
                    format!("side {}", self.r.pick(&["al", "sat"])),
                    format!("max_bars {}", self.int()),
                    "source \"st\" \"2\"".to_owned(),
                ];
                if self.r.chance(50) {
                    cl.push(format!("target {}%", self.float()));
                }
                if self.r.chance(50) {
                    cl.push(format!("stop {}", self.float()));
                }
                for k in (1..cl.len()).rev() {
                    let j = self.r.below(k + 1);
                    cl.swap(k, j);
                }
                format!("signal o{i} = {} {}", self.expr(3), cl.join(" "))
            }
            _ => {
                let a = format!("bucket({}, [1, 2, 3])", self.expr(2));
                let b = self.expr(2);
                if self.r.chance(50) {
                    format!("state o{i} {{\n  m = {a},\n  n = {b}\n}}")
                } else {
                    format!("state o{i} {{ m = {a}, n = {b}, }}")
                }
            }
        }
    }

    /// A whole program text.
    pub fn program(&mut self) -> String {
        let mut lines = vec!["# generated".to_owned()];
        self.params(&mut lines);
        let (nb, no) = (self.r.below(6), 1 + self.r.below(4));
        let (mut b, mut o) = (0, 0);
        while b < nb || o < no {
            if o < no && (b == nb || self.r.chance(30)) {
                lines.push(self.output(o));
                o += 1;
            } else {
                let e = self.expr(3);
                let name = format!("b{b}");
                lines.push(format!(
                    "{name} = {e}{}",
                    if self.r.chance(20) { "  # note" } else { "" }
                ));
                self.binds.push(name);
                b += 1;
            }
            if self.r.chance(15) {
                lines.push(String::new());
            }
        }
        lines.join("\n") + "\n"
    }
}

/// Random IR programs; `underscore` gives some nodes `_k` ids (used once, twice or not).
pub fn ir_program(r: &mut Rng, underscore: bool) -> Value {
    let mut params = vec![];
    let mut keys = vec![];
    for i in 0..r.below(3) {
        let k = format!("k{i}");
        params.push(match r.below(3) {
            0 => json!({"key": k, "kind": "int", "min": 1, "max": 50, "default": 3}),
            1 => json!({"key": k, "kind": "float", "unit": "pct", "min": 0.5, "max": 9.75,
                        "default": 1.0, "description": "çizgi \"seviye\"\nikinci satır"}),
            _ => json!({"key": k, "kind": "bool", "default": false}),
        });
        keys.push(k);
    }
    let mut nodes: Vec<Value> = vec![];
    let mut ids: Vec<(String, &str)> = vec![];
    let n = 1 + r.below(10);
    for i in 0..n {
        let id = if underscore && r.chance(60) {
            format!("_{}", 1 + i * 2)
        } else {
            format!("n{i}")
        };
        let (node, kind) = ir_node(r, &ids, &keys);
        let mut node = node;
        node["id"] = json!(id);
        ids.push((id, kind));
        nodes.push(node);
    }
    let mut outputs = vec![];
    for j in 0..1 + r.below(3) {
        let node = ir_ref(r, &ids);
        outputs.push(match r.below(4) {
            0 => json!({"name": format!("o{j}"), "kind": "series", "node": node}),
            1 => json!({"name": format!("o{j}"), "kind": "event", "node": node}),
            2 => json!({"name": format!("o{j}"), "kind": "signal", "node": node, "side": "sat",
                        "rule": {"stop_pct": 2.5, "max_bars": 10},
                        "source": {"strategy": "s\\t", "version": "1.0"}}),
            _ => json!({"name": format!("o{j}"), "kind": "state",
                        "dims": [{"name": "a", "node": node}, {"name": "b", "node": ir_ref(r, &ids)}]}),
        });
    }
    let mut p = json!({"version": "qp-ir/0", "nodes": nodes, "outputs": outputs});
    if !params.is_empty() {
        p["params"] = json!(params);
    }
    p
}

fn ir_ref(r: &mut Rng, ids: &[(String, &str)]) -> String {
    let (id, kind) = &ids[r.below(ids.len())];
    match *kind {
        "macd" if r.chance(50) => format!("{id}.{}", r.pick(&["signal", "histogram", "macd"])),
        "bollinger" if r.chance(50) => format!("{id}.{}", r.pick(&["upper", "lower"])),
        _ => id.clone(),
    }
}

fn int_v(r: &mut Rng, keys: &[String]) -> Value {
    if !keys.is_empty() && r.chance(25) {
        json!({"param": r.pick(keys)})
    } else {
        json!(r.below(40) as i64 - 3)
    }
}

fn float_v(r: &mut Rng, keys: &[String]) -> Value {
    if !keys.is_empty() && r.chance(25) {
        json!({"param": r.pick(keys)})
    } else {
        json!(*r.pick(&[2.0, 0.5, -1.25, 1e-7, 123_456.789, -0.0, 3.0]))
    }
}

fn ir_node(r: &mut Rng, ids: &[(String, &str)], keys: &[String]) -> (Value, &'static str) {
    if ids.is_empty() || r.chance(20) {
        let v = match r.below(5) {
            0 => json!({"op": "input", "field": r.pick(FIELDS), "tf": r.pick(TFS)}),
            1 => json!({"op": "const", "value": float_v(r, keys)}),
            2 => json!({"op": "const", "value": r.chance(50)}),
            3 => json!({"op": *r.pick(&["bar_index", "time"])}),
            _ => json!({"op": "input", "field": r.pick(FIELDS)}),
        };
        return (v, "leaf");
    }
    let x = ir_ref(r, ids);
    let y = ir_ref(r, ids);
    match r.below(12) {
        0 => (
            json!({"op": "macd", "x": x, "fast": int_v(r, keys), "slow": 26, "signal": int_v(r, keys)}),
            "macd",
        ),
        1 => (
            json!({"op": "bollinger", "x": x, "period": 20, "k": float_v(r, keys)}),
            "bollinger",
        ),
        2 => (
            json!({"op": *r.pick(WINDOW), "x": x, "period": int_v(r, keys)}),
            "w",
        ),
        3 => (
            json!({"op": *r.pick(&["add", "sub", "mul", "div", "gt", "le", "eq", "and", "or", "min", "cross_below"]), "a": x, "b": y}),
            "b",
        ),
        4 => (
            json!({"op": *r.pick(&["neg", "abs", "not", "defined", "align"]), "x": x}),
            "u",
        ),
        5 => (json!({"op": "lag", "x": x, "n": int_v(r, keys)}), "u"),
        6 => (
            json!({"op": *r.pick(&["streak", "bars_since", "once"]), "c": x}),
            "u",
        ),
        7 => (json!({"op": "rearm", "trigger": x, "reset": y}), "u"),
        8 => (
            json!({"op": "cooldown", "trigger": x, "bars": int_v(r, keys)}),
            "u",
        ),
        9 => (
            json!({"op": "atr", "period": int_v(r, keys), "tf": "1d"}),
            "u",
        ),
        10 => (
            json!({"op": "line", "a": {"t": 1_704_067_200_000_i64, "price": float_v(r, keys)},
                      "b": {"t": int_v(r, keys), "price": 101.5}}),
            "u",
        ),
        _ => (
            json!({"op": "bucket", "x": x, "edges": [float_v(r, keys), 10.0]}),
            "u",
        ),
    }
}
