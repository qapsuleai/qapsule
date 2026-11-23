# RF1/V bağımsız denetim — `rf1/v-voice` (qp-capsule `voice/` → `qp-voice`)

**Hüküm: GEÇTİ.**

- Denetlenen: `rf1/v-voice` @ `4e925a59` (2 commit: `717ad036` taşıma + yol düzeltmeleri,
  `4e925a59` arch-config / taban çizgisi / DEBT.md).
- Yöntem: `rounds/RF1/PLAN.md` §3.2, `rounds/RG1/PLAN.md` §2 (gövde, ad, imza, sabit,
  biçim ve test beklentisi değişmez; yalnız mod/use/görünürlük/yol, Cargo.toml, `//!`, README).
- Ortam: ayrık worktree `.claude/worktrees/audit-v` (detached), içinde `git merge --no-edit main`
  (main @ `c7ad3d6a`) — çakışma yok, birleşik ağaç `0be64b79`.
  `CARGO_TARGET_DIR=target/wt/audit-v`. Denetlenen dal değiştirilmedi, birleştirilmedi.

## Sonuçlar

| # | Adım | Sonuç |
|---|---|---|
| 1 | `git diff main...rf1/v-voice --stat` | 38 dosya, +165 −116. Yalnız: `Cargo.lock`, `crates/qp-capsule` (`lib.rs` 1 satır, 2 README, `voice/README.md` silindi), `crates/qp-voice/**` (22 dosya taşındı: 9 tanesi %100 aynı, kalanı R084–R099), `crates/qp-session` (`Cargo.toml` +1, 5 dosyada yol satırları), `docs/DEBT.md`, `scripts/arch-baseline.json`, `scripts/arch-config.json`. Kök `Cargo.toml` değişmedi (üyeler zaten `crates/*` ile). Kapsam dışı dosya yok. ✔ |
| 2 | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` | fmt temiz; clippy 0 uyarı; test 184 ikili, 1197 geçti / 0 düştü / 9 yok sayıldı. `qp-voice` 11 test (main'de `voice/` altında da 11 `#[test]`). ✔ |
| 3 | `python3 scripts/refactor/moved.py main HEAD` | 239 öğe değişmeden taşındı, 12 PATH, 0 ALLOW, **0 CHANGED**, **0 FLAG**; 8 dosyada yalnız yol içe aktarma değişikliği; 2 REEXPORT (`qp_session::voices`: `qp_capsule::voice` → `qp_voice`, ad aynı). ✔ |
| 3b | 10 rastgele öğe elle (tohum 20261123) | `selvi/tests.rs::phrasings`, `selvi/headline.rs::hl_breakout`, `marla/why.rs::now`, `selvi/tests.rs::surfaces`, `selvi/holds.rs::d_no_longer`, `selvi.rs::when`, `selvi.rs::inst_loc`, `marla/words.rs::mult` bayt bayt aynı; `marla/words.rs::words_tr` yalnız görünürlük (`pub(in crate::voice)` → `pub(crate)`); `marla/head.rs::core` yalnız 2 yol satırı (aşağıda). Ayrıca tüm taşıma diff'i (`git diff -M`) satır satır okundu: yol/use/görünürlük/doc-yolu dışında değişiklik yok. ✔ |
| 4 | `scripts/arch-check` | 0 hata; 5 adlı istisna (İ5 artık yok), onay bekleyen 0. Alan sırası `qp-source < qp-facts < qp-grammar < qp-capsule < qp-voice`. ✔ |
| 4b | `--table` | `qp-capsule` 8467 test dışı (≤ 12000 ✔), `qp-voice` 4763 test dışı / 589 test. `qp-session` 31999 (tavan 31999, değişmedi). |
| 4c | `--config-diff` | 16 değişiklik, **0 kaynaksız gevşetme**: İ5 kaldırıldı; `qp-voice` alan katmanına yerleşti; 2 `file.soft` + 10 `fn.lines` kalemi aynı değerlerle yeni yola taşındı; `lint.allow` qp-capsule 26 → 14, qp-voice 12 (toplam 252 aynı; `count` 13 → 14 crate sayısıdır, borç büyümesi değil). ✔ |
| 4d | `docs/DEBT.md` | `arch-check --debt-md` yeniden üretimi dosyayla bayt bayt aynı (eşzamanlı). ✔ |
| 5 | `scripts/refactor/compare.sh` (birleşik ağaçta) | `compare: 270 items byte-identical to the baseline` (çıkış 0). ✔ |
| 6 | WASM `--profile wasm` | ham 2 345 901 B (taban 2 345 821, +80 B, +%0,003); gzip -9 769 889 B (taban 769 836, +53 B, +%0,007). ±%2 içinde. ✔ |
| 7 | `qp-voice` crate düzeni | `Cargo.toml`'da `[lints] workspace = true` ✔; bağımlılıklar `qp-schema`, `qp-calendar`, `qp-facts`, `qp-capsule` (aşağı/alan-içi sıralı), geliştirme `qp-source`. `lib.rs` yalnız `//!` belge + `pub mod`/`mod`/`pub use`/`#[cfg(test)] mod tests;` ✔. README: amaç, katman, bağımlılıklar, dışa açık yüzey (`speak`, `Input`, `Surfaces`, `Gloss`, `text_face`, `unnumber`, kayıt adları, 4 modül), `qp_session::voices` yeniden dışa açımı, düzen, "ne değildir" — `lib.rs` ile tutarlı. ✔ |

## Yazarın rustfmt yeniden sarmasından kaçınmak için yaptığı iki düzenleme

1. **`marla/head.rs`**: `use qp_capsule::realize;` eklendi; `core` gövdesinde
   `crate::realize::tr::noun_state` / `crate::realize::en::noun_bare` →
   `realize::tr::noun_state` / `realize::en::noun_bare`. Dosyada `realize` adlı başka bir
   tanım/değişken yok; açık `use`, `super::*` glob'unu gölgeler, yani ad aynı modüle
   (`qp_capsule::realize`) çözülür. Son segmentler (`noun_state`, `noun_bare`) korunur;
   `moved.py` bunu PATH sayar. **Kabul edilebilir** (yalnız yol; davranış ve biçim aynı).
2. **`marla.rs` `engine_tests`**: beş `use crate::…` satırı üç `use qp_capsule::…` satırına
   birleştirildi (`{engine::EngineConfig, pipeline::simulate_engine, InstrumentInput}`).
   Aynı adlar, aynı hedefler; test gövdesi değişmedi. **Kabul edilebilir** (yalnız use
   düzeni). Not: `sentence::{…}` satırı yine de rustfmt tarafından sarıldı; bu salt biçim.

## Bulgular

Engelleyici bulgu yok. Engellemeyen notlar:

- **B1 (düşük, belge):** `qp_capsule::voice` / `crates/qp-capsule/src/voice/…` eski yolu hâlâ
  geçiyor: `crates/qp-schema/src/protocol/input.rs:160` (`///` — şema açıklamasına akabilir;
  saf taşımada dokunulmaması doğru, ama JSON Schema üretimi açısından ayrı bir işte
  düzeltilmeli), `crates/qp-session/README.md:19`, `crates/qp-session/src/capsule/sentences.rs:85`
  (`///`), `docs/CAPSULE.md:245`, `docs/LANGUAGE.md:14,66,121`, `docs/PROTOCOL.md:389`.
  Öneri: RF1 kapanışında ya da sonraki belge işinde `qp_voice` olarak güncellensin
  (`qp-schema` olanı şema üretimiyle birlikte).
- **B2 (bilgi):** `qp-session` net satır eklemedi: `src/` altında +10/−10 (yalnız yol),
  `lib.rs`'te yeniden dışa açım satırı rustfmt sıralaması nedeniyle iki satır aşağı kaydı
  (belge yorumuyla birlikte); `Cargo.toml` +1 bağımlılık satırı.
- **B3 (bilgi):** `DEBT.md`'de `lint.allow` özet sütunu 13 → 14 görünür; bu crate sayısıdır,
  izin toplamı 252'de sabit. Okuyucu için yanıltıcı olabilir; araç davranışı, bu dalın sorunu değil.

## Temizlik

Denetim worktree'si (`.claude/worktrees/audit-v`) ve `target/wt/audit-v` kaldırıldı;
scratchpad'deki geçici test özeti silindi.
