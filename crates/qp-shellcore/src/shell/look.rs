//! The look (RG1: moved out of `shell.rs`): fonts, choices, gloss, CSS, the reader's
//! language, look, instruments and audience, and their setters.

use super::*;

/// (R5/T3c) The `FontFace` registrations of the embedded `files` (`(id, asset)`; an id
/// without an asset is the paint path's default family at 400): each file under its own
/// CSS family and weight, and a file of another weight than 400 also under the CSS family
/// of the upright 400 file of the same published family in `assets` (if that family is a
/// different CSS family), so that family's `font-weight` uses the real file.
#[must_use]
pub fn font_faces(
    files: &[(&str, Option<&qp_grammar::look::FontAsset>)],
    assets: &[qp_grammar::look::FontAsset],
) -> Vec<Value> {
    let mut out = Vec::new();
    for (id, asset) in files {
        let Some(f) = asset else {
            out.push(
                json!({"id": id, "family": crate::shell::paint::PAINT_FONT_FAMILY,
                "weight": "400"}),
            );
            continue;
        };
        out.push(json!({"id": id, "family": f.css_family, "weight": f.weight.to_string()}));
        if f.weight != 400 && !f.italic {
            if let Some(base) = assets.iter().find(|g| {
                g.family == f.family && g.weight == 400 && !g.italic && g.css_family != f.css_family
            }) {
                out.push(json!({"id": id, "family": base.css_family,
                    "weight": f.weight.to_string()}));
            }
        }
    }
    out
}

impl ShellCore {
    /// (R4/S1) The `style` gesture of the presentation's defaults (RP4/T7: and the rows'
    /// time, `clock`): feed layout, depth
    /// transition and outcome mark picked in M9–M11 (`qp_grammar::feeds::PRESENTED_FEED`,
    /// `transitions::PRESENTED_TRANSITION`, `glyph::outcome::PRESENTED_OUTCOME`) and the
    /// face (`qp_grammar::faces::PRESENTED_FACE`). (R5/S) R5's picks: the look and its
    /// companion face (M16, `look::PRESENTED_LOOK`), the voice (M17), the story chart
    /// (M18, `stories::PRESENTED_STORY`) and the open capsule design (M20,
    /// `open::PRESENTED_OPEN`). The protocol's own defaults (a bare connection) stay `r3`
    /// / `r4`. (RD2, K2) The presentation is a phone surface: the presented look's touch
    /// profile (`look::phone_profile`, `tavek-tel`) and its companion face.
    #[must_use]
    pub fn presented_look() -> Value {
        let look = qp_grammar::look::phone_profile(qp_grammar::look::PRESENTED_LOOK);
        let face =
            qp_grammar::look::companion_face(look).unwrap_or(qp_grammar::faces::PRESENTED_FACE);
        json!({"kind":"style",
               "face": face,
               "feed": qp_grammar::feeds::PRESENTED_FEED,
               // (R5/S) The presented motion design's opening (M21, `surek`).
               "transition": qp_grammar::motion::by_name(Some(qp_grammar::motion::PRESENTED_MOTION)).transition,
               "outcome": qp_grammar::glyph::outcome::PRESENTED_OUTCOME,
               // (R5/m17-int) M17's voice (`qp_session::voices::PRESENTED_VOICE`).
               "voice": qp_session::voices::PRESENTED_VOICE,
               "look": look,
               "story": qp_grammar::stories::PRESENTED_STORY,
               "open": qp_grammar::open::PRESENTED_OPEN,
               // (RP4/T7) Rows carry the event's time (`?clock=0`: as before).
               "clock": true})
    }

    /// (R4/H-M8910) The names each setting accepts, from the registries (the settings
    /// page lists them; nothing is hard-coded in the shell): `{face, alphabet, feed,
    /// transition, outcome, look, voice}`, the default first.
    #[must_use]
    pub fn choices_json() -> Value {
        json!({
            "face": qp_grammar::faces::names(),
            "alphabet": qp_grammar::glyph::ALPHABETS,
            "feed": qp_grammar::feeds::names(),
            "transition": qp_grammar::transitions::names(),
            "outcome": qp_grammar::glyph::outcome::names(),
            "look": qp_grammar::look::names(),
            // (R5/M19) The feed orders (feed layouts) and the host's feed rules M19
            // measured, best first.
            // `layout`: no order chosen, the feed layout orders the feed.
            "feed_order": std::iter::once("layout").chain(qp_grammar::feeds::FEED_ORDERS).collect::<Vec<_>>(),
            "feed_rule": qp_capsule::feedrule::FEED_RULES.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            "voice": qp_session::voices::names(),
            "story": qp_grammar::stories::names(),
            // (R5/M20) The open capsule designs (`r4`: today's).
            "open": qp_grammar::open::names(),
            // (R5/M21) Motion designs (the default first).
            "motion": qp_grammar::motion::names(),
        })
    }

    /// (R5/m17-int, K49) The glossary explanation to show: `{term, text, target}` (the
    /// target tapped, `face:<capsule>` or `scene`), or `null` when none is open. The page
    /// shows it as it is; the texts are the voice's (`docs/LANGUAGE.md`).
    #[must_use]
    pub fn gloss_json(&self) -> Value {
        match &self.gloss {
            Some((target, t)) => json!({"term": t.term, "text": t.text, "target": target}),
            None => Value::Null,
        }
    }

    /// (R5/m17-int) Closes the glossary explanation (the page's dismiss).
    pub fn close_gloss(&mut self) {
        self.gloss = None;
    }

    /// (R5/T3) The chrome's CSS custom properties for the connection's look and theme
    /// (`qp_grammar::look::css_vars`) as a `{name: value}` object, or `None` when they did
    /// not change since the last hand-out (the page sets them verbatim on its root
    /// element; its stylesheet holds no colour, font, size or radius of its own).
    /// (R5/T3b) Also the frames the page lays out: `--qp-phone-w`/`-h` (the phone
    /// surface), `--qp-face-w`/`-h` and `--qp-scene-w`/`-h` (a canvas's CSS size before
    /// the page sizes it from [`Self::face_size_json`] or the open scene).
    pub fn take_css(&mut self) -> Option<Value> {
        let (fw, fh) = self.frames["face"];
        let (sw, sh) = self.frames["scene"];
        let key = (
            self.look.look.clone(),
            self.look.theme.clone(),
            [fw, fh, sw, sh],
        );
        if self.css_shown.as_ref() == Some(&key) {
            return None;
        }
        let look = qp_session::viewer::look_of(&self.look);
        let mut vars = qp_grammar::look::css_vars(look, self.look.theme.as_deref());
        for (k, v) in [
            ("--qp-phone-w", PHONE_SCREEN.0),
            ("--qp-phone-h", PHONE_SCREEN.1),
            ("--qp-face-w", fw),
            ("--qp-face-h", fh),
            ("--qp-scene-w", sw),
            ("--qp-scene-h", sh),
        ] {
            vars.push((k.to_owned(), format!("{v}px")));
        }
        self.css_shown = Some(key);
        let obj: serde_json::Map<String, Value> = vars
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        Some(Value::Object(obj))
    }

    /// (R5/T3) The embedded font assets a paint path registers (`FontFace`): `[{id,
    /// family, weight}]`, `family` being the CSS family the core's font strings name and
    /// `weight` the file's CSS weight (the `FontFace` descriptor, a string). Every asset
    /// is listed, so a look change needs no new download; the bytes come from
    /// [`ShellCore::font_bytes`] (no network). (R5/T3c) A file of another weight than 400
    /// whose published family has a 400 file is listed once more under that file's CSS
    /// family with its weight ([`font_faces`]): the chrome's body family then has a real
    /// semibold face and the browser does not synthesize one.
    #[must_use]
    pub fn fonts_json() -> Value {
        let files: Vec<(&str, Option<&qp_grammar::look::FontAsset>)> = qp_render::fonts::FILES
            .iter()
            .map(|(id, _)| (*id, qp_grammar::look::font(id)))
            .collect();
        Value::Array(font_faces(&files, qp_grammar::look::FONTS))
    }

    /// (R5/T3) The bytes of embedded font asset `id` (empty if unknown).
    #[must_use]
    pub fn font_bytes(id: &str) -> Vec<u8> {
        qp_render::fonts::bytes(id)
            .map(<[u8]>::to_vec)
            .unwrap_or_default()
    }

    /// The language the connection reads in (`tr`, `en`).
    #[must_use]
    pub fn lang(&self) -> &'static str {
        lang_name(self.lang)
    }

    /// The connection's view state.
    #[must_use]
    pub fn look(&self) -> &ViewState {
        &self.look
    }

    /// (R5/M19) The view state as JSON with the presentation's feed order and feed rule
    /// (`feed_order`, `feed_rule`; absent: none chosen) — what the settings page shows.
    #[must_use]
    pub fn look_json(&self) -> Value {
        let mut v = serde_json::to_value(&self.look).unwrap_or(Value::Null);
        if let Some(o) = &self.feed_order {
            v["feed_order"] = json!(o);
        }
        if let Some(r) = &self.feed_rule {
            v["feed_rule"] = json!(r);
        }
        // (R5/M21) The motion design (absent: the default) and reduced motion.
        if let Some(m) = &self.mo.design {
            v["motion"] = json!(m);
        }
        if self.mo.reduced {
            v["reduced_motion"] = json!(true);
        }
        v
    }

    /// Symbols of the document's bar series (document order): the settings page's
    /// watchlist and positions.
    #[must_use]
    pub fn instruments(&self) -> Vec<String> {
        if let Some(v) = &self.viewer {
            return v.instruments.clone();
        }
        let Some(doc) = self.replica.document() else {
            return Vec::new();
        };
        doc.series
            .iter()
            .filter_map(|s| s.instrument.as_ref().map(|i| i.symbol.clone()))
            .collect()
    }

    /// The reader's own audience as the document gives it to them (`null` if none).
    #[must_use]
    pub fn audience(&self) -> Value {
        if let Some(v) = &self.viewer {
            return v.audience.as_ref().map_or(Value::Null, |a| {
                serde_json::to_value(a).unwrap_or(Value::Null)
            });
        }
        self.replica
            .document()
            .and_then(|d| d.audiences.get(&self.principal))
            .map_or(Value::Null, |a| {
                serde_json::to_value(a).unwrap_or(Value::Null)
            })
    }

    /// `set_sensitivity` (`important`, `balanced`, `all`; anything else: the group's
    /// default).
    pub fn set_sensitivity(&mut self, level: &str) {
        let s = match level {
            "important" | "balanced" | "all" => Value::String(level.to_owned()),
            _ => Value::Null,
        };
        self.ask(
            "cmd",
            Asked::Command,
            json!({"type":"command","ops":[{"op":"set_sensitivity","sensitivity":s}]}),
        );
    }

    /// `set_audience` from the settings page's choices: `{group, sensitivity?, watch:
    /// [symbol], long: [symbol], short: [symbol]}` (a position is one unit long or short).
    /// (R4/S1) A list may also be one comma-separated string, as the page's URL gives it
    /// (`?watch=SYN01,SYN02&long=SYN09`); (R4/S2) such a list keeps only symbols of the
    /// host's feeds, each once (a typo or a foreign symbol is dropped, not watched).
    pub fn set_audience(&mut self, choice: &Value) {
        let group = choice
            .get("group")
            .and_then(Value::as_str)
            .filter(|g| *g == "future")
            .unwrap_or("habituated");
        let mut aud = json!({"group": group});
        if let Some(s) = choice.get("sensitivity").and_then(Value::as_str) {
            if ["important", "balanced", "all"].contains(&s) {
                aud["sensitivity"] = json!(s);
            }
        }
        let list = |k: &str| -> Vec<String> {
            match choice.get(k) {
                Some(Value::Array(a)) => a
                    .iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect(),
                Some(Value::String(t)) => {
                    let mut out: Vec<String> = Vec::new();
                    for x in t.split(',').map(str::trim) {
                        if self.symbols.contains(x) && !out.iter().any(|o| o == x) {
                            out.push(x.to_owned());
                        }
                    }
                    out
                }
                _ => Vec::new(),
            }
        };
        let watch = list("watch");
        if !watch.is_empty() {
            aud["watchlist"] = json!(watch);
        }
        let mut pos = serde_json::Map::new();
        for s in list("long") {
            pos.insert(s, json!(1.0));
        }
        for s in list("short") {
            pos.insert(s, json!(-1.0));
        }
        if !pos.is_empty() {
            aud["positions"] = Value::Object(pos);
        }
        self.ask(
            "cmd",
            Asked::Command,
            json!({"type":"command","ops":[{"op":"set_audience","audience":aud}]}),
        );
    }

    /// The `style` gesture: face design, alphabet, theme (`""`: unchanged).
    pub fn set_style(&mut self, face: &str, alphabet: &str, theme: &str) {
        let mut g = json!({"kind":"style"});
        for (k, v) in [("face", face), ("alphabet", alphabet), ("theme", theme)] {
            if !v.is_empty() {
                g[k] = json!(v);
            }
        }
        self.ask("in", Asked::Input, json!({"type":"input","gesture":g}));
    }

    /// (R4/H-M8910) The `style` gesture from a settings object `{face?, alphabet?, theme?,
    /// feed?, transition?, outcome?}` (absent or `""`: unchanged).
    pub fn set_look(&mut self, choice: &Value) {
        let mut g = json!({"kind":"style"});
        // (R5/M21) The motion design (a presentation setting): it names the transition it
        // opens with; a transition given in the same choice still wins.
        if let Some(v) = choice.get("motion").and_then(Value::as_str) {
            if let Some(t) = self.set_motion(v) {
                g["transition"] = json!(t);
            }
        }
        for k in [
            "face",
            "alphabet",
            "theme",
            "feed",
            "transition",
            "outcome",
            "look",
            "voice",
            "story",
            "open",
        ] {
            if let Some(v) = choice.get(k).and_then(Value::as_str) {
                if !v.is_empty() {
                    g[k] = json!(v);
                }
            }
        }
        // (R5/M19) The feed order and the feed rule (presentation settings, not protocol
        // values): a known name is kept; `""` leaves it unchanged.
        if let Some(v) = choice.get("feed_order").and_then(Value::as_str) {
            if v == "layout" || qp_grammar::feeds::FEED_ORDERS.contains(&v) {
                self.feed_order = (v != "layout").then(|| v.to_owned());
                // The page repaints the feed at its next paint pass.
                self.pending.insert("feed".to_owned(), json!(true));
            }
        }
        if let Some(v) = choice.get("feed_rule").and_then(Value::as_str) {
            if qp_capsule::feedrule::FEED_RULES
                .iter()
                .any(|(n, _)| *n == v)
            {
                self.feed_rule = Some(v.to_owned());
            }
        }
        // (R5/M18, K54) Show or hide the closed stories of the story chart.
        if let Some(b) = choice.get("hide_closed").and_then(Value::as_bool) {
            g["hide_closed"] = json!(b);
        }
        // (RP4/T7) Rows carry the event's time: `true`/`false`, or the URL's `"1"`/`"0"`
        // (`""`: unchanged).
        match choice.get("clock") {
            Some(Value::Bool(b)) => g["clock"] = json!(b),
            Some(Value::String(v)) if v == "1" || v == "0" => g["clock"] = json!(v == "1"),
            _ => {}
        }
        // (R5/S) A look brings its companion face, unless the same choice names a face.
        if g.get("face").is_none() {
            if let Some(f) = g
                .get("look")
                .and_then(Value::as_str)
                .and_then(qp_grammar::look::companion_face)
            {
                g["face"] = json!(f);
            }
        }
        if g.as_object().is_some_and(|o| o.len() > 1) {
            self.ask("in", Asked::Input, json!({"type":"input","gesture":g}));
        }
    }
}

#[cfg(test)]
mod font_tests {
    use super::*;
    use qp_grammar::look::FontAsset;

    fn asset(id: &'static str, family: &'static str, weight: u16, css: &'static str) -> FontAsset {
        FontAsset {
            id,
            family,
            weight,
            css_family: css,
            ..qp_grammar::look::FONTS[0]
        }
    }

    /// (R5/T3c) A semibold file is also the body family's `font-weight: 600` face; the
    /// embedded set lists each file with its weight.
    #[test]
    fn semibold_is_registered_under_the_body_family() {
        let assets = [
            qp_grammar::look::FONTS[0],
            asset("sans", "Some Sans", 400, "QpSans"),
            asset("sans-600", "Some Sans", 600, "QpSansSemi"),
        ];
        let files: Vec<(&str, Option<&FontAsset>)> =
            assets.iter().map(|a| (a.id, Some(a))).collect();
        let faces = font_faces(&files, &assets);
        let got: Vec<(String, String, String)> = faces
            .iter()
            .map(|f| {
                (
                    f["id"].as_str().unwrap().to_owned(),
                    f["family"].as_str().unwrap().to_owned(),
                    f["weight"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let row = |a: &str, b: &str, c: &str| (a.to_owned(), b.to_owned(), c.to_owned());
        assert_eq!(
            got,
            vec![
                row("plex-mono", "QpMono", "400"),
                row("sans", "QpSans", "400"),
                row("sans-600", "QpSansSemi", "600"),
                row("sans-600", "QpSans", "600"),
            ]
        );
        let real = ShellCore::fonts_json();
        let real = real.as_array().unwrap();
        assert!(real.len() >= qp_render::fonts::FILES.len());
        for f in real {
            assert!(f["weight"].is_string() && f["family"].is_string(), "{f}");
        }
    }
}
