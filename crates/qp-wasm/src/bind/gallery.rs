use wasm_bindgen::prelude::*;

use crate::gallery::GalleryCore;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    /// `Date.now()` (ms): the summary's date line (not compute: the device's clock).
    #[wasm_bindgen(js_namespace = Date, js_name = now)]
    fn date_now() -> f64;
}

/// The gallery page's core.
#[wasm_bindgen]
pub struct Gallery {
    inner: GalleryCore,
}

#[wasm_bindgen]
impl Gallery {
    /// From the saved state (local storage; `""`: none) and the address parameters
    /// `lang`, `theme`, `pick`, `now` (as they are; `""`: absent).
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(saved: &str, lang: &str, theme: &str, pick: &str, now: &str) -> Gallery {
        Self {
            inner: GalleryCore::new(saved, lang, theme, pick, now),
        }
    }

    /// The layout's address for the current language and theme.
    #[must_use]
    pub fn layout_url(&self) -> String {
        self.inner.layout_url()
    }

    /// The host's layout (JSON text).
    ///
    /// # Errors
    /// A malformed layout.
    pub fn load(&mut self, layout_json: &str) -> Result<(), JsError> {
        self.inner.load(layout_json).map_err(|e| JsError::new(&e))
    }

    /// Everything the page paints (JSON text; `GalleryCore::view`).
    #[must_use]
    pub fn view(&self) -> String {
        self.inner.view().to_string()
    }

    /// `{"topic/cand": picked}` (JSON text).
    #[must_use]
    pub fn marks(&self) -> String {
        self.inner.marks().to_string()
    }

    /// "I'd pick this" tapped.
    pub fn pick(&mut self, topic: &str, cand: &str) {
        self.inner.pick(topic, cand);
    }

    /// A candidate's note as typed.
    pub fn note(&mut self, topic: &str, cand: &str, text: &str) {
        self.inner.note(topic, cand, text);
    }

    /// The summary to copy. In the browser the date line reads the device's clock
    /// unless the address gave `now`.
    #[must_use]
    pub fn summary(&mut self) -> String {
        #[cfg(target_arch = "wasm32")]
        if !self.inner.has_now() {
            #[allow(clippy::cast_possible_truncation)]
            return self.inner.summary_at(date_now() as i64);
        }
        self.inner.summary()
    }

    /// A relative link with the same picks.
    #[must_use]
    pub fn share(&self) -> String {
        self.inner.share()
    }

    /// The state to keep in local storage.
    #[must_use]
    pub fn state(&self) -> String {
        self.inner.state()
    }

    /// The device line, from the page's raw values.
    pub fn set_device(&mut self, ua: &str, w: f64, h: f64, dpr: f64) {
        self.inner.set_device(ua, w, h, dpr);
    }

    /// Türkçe ↔ English.
    pub fn toggle_lang(&mut self) {
        self.inner.toggle_lang();
    }

    /// The theme select's value (`dark`, `light`).
    pub fn set_theme(&mut self, theme: &str) {
        self.inner.set_theme(theme);
    }

    /// `tr` or `en`.
    #[must_use]
    pub fn lang(&self) -> String {
        self.inner.lang().to_owned()
    }

    /// `dark` or `light`.
    #[must_use]
    pub fn theme(&self) -> String {
        self.inner.theme().to_owned()
    }

    /// The chrome's CSS custom properties (JSON `{name: value}`).
    #[must_use]
    pub fn css(&self) -> String {
        self.inner.css().to_string()
    }

    /// The embedded font assets `[{id, family, weight}]` (JSON text).
    #[must_use]
    pub fn fonts(&self) -> String {
        crate::shell::ShellCore::fonts_json().to_string()
    }

    /// The bytes of font asset `id`.
    #[must_use]
    pub fn font_bytes(&self, id: &str) -> Vec<u8> {
        crate::shell::ShellCore::font_bytes(id)
    }
}
