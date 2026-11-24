# Denetim — `rf1/q-grammar-docs` @ 1faf04b6 (belge yorumu düzeltmeleri)

**Hüküm: GEÇTİ**

Denetçi bağımsız alt ajan; dal değiştirilmedi, birleştirilmedi. Taban: `main` = ae3709e3 (dalın birleşme tabanı).

## Kapsam

`git diff main...rf1/q-grammar-docs --stat`: 14 dosya, +47/−42; yalnız `qp-capsule` (3), `qp-designs` (2),
`qp-grammar` (7), `qp-look` (2). `qp-schema`, oturum crate'leri (`qp-plane`, `qp-ops`, `qp-session`),
README ve kod dışı dosya yok. Commit'ler: bf69d065, 06d6a0cd, 1faf04b6.

## Kod değişmedi

- Her dosya için `main:` ve dal sürümü `scripts/archcheck/rustscan.py` `strip()` (yorum + dize içi
  boşaltılır) ile soyuldu, boşluk normalleştirilip karşılaştırıldı: 14/14 eşit.
- Dize içeriğini de kapsamak için ikinci kontrol: `//` ile başlamayan satırlar iki sürümde
  birebir aynı (14/14). Diff'te `//` ile başlamayan değişmiş satır sayısı: 0.

## Metin doğruluğu (örneklem, dal üzerinde)

| İddia | Kanıt |
|---|---|
| `outcome.atr_fast` / `outcome.atr_slow` anahtarları | `qp-schema/src/params/system.rs:141, 152` |
| `look/registry.rs` içinde `LOOKS` | `qp-look/src/look/registry.rs:7` |
| `qp_look::look::FontAsset::width` pub | `look/fonts.rs:78` `pub fn width`; `look.rs:48` `pub use fonts::{…FontAsset…}` |
| `FONTS` `look/fonts.rs`'te, font dosyaları `crates/qp-render/assets/fonts/` | `fonts.rs:102`; klasör var |
| `qp_designs::faces::FACES`, `motion::MOTIONS`, `qp_glyph::outcome::STYLES` | `faces.rs:91`, `motion.rs:74`, `outcome.rs:114` |
| `qp-wasm` `bind/shell.rs` `face_scene` | `shell.rs:477` |
| `contract.rs` yeniden ihraç ediliyor | `qp-capsule/src/lib.rs:61-62` |
| `qp_designs::open::text`, `open::tesir`, `look/ornek.rs` | `open.rs:323`, `open.rs:33`; dosya var |
| `CompileOptions::face` | `compile.rs:91` `pub face: Option<String>` |

Yanlış iddia bulunmadı. Köşeli bağlantıların çözüldüğü ayrıca rustdoc ile doğrulandı.

## Derleme ve yapı

Ayrı worktree (`.claude/worktrees/audit-q`, `CARGO_TARGET_DIR=target/wt/audit-q`):

- `cargo doc --no-deps -p qp-grammar -p qp-designs -p qp-look -p qp-capsule -p qp-glyph -p qp-voice`:
  dalda **0 uyarı**; aynı komut `main`'de 14 uyarı (düzeltmeler gerçek).
- `scripts/arch-check`: 0 hata, taban çizgisi içinde (`file.soft 36, fn.lines 176, lint.allow 18, …`);
  2 adlı istisna (önceden onaylı), bekleyen yok.

## Bulgular

- Engelleyici bulgu yok.
- Not: `qp-glyph` ve `qp-voice` görevde anılıyor ama dalda bu crate'lere değişiklik yok;
  ikisi de rustdoc'ta zaten uyarısız.

## Temizlik

Worktree kaldırıldı (`git worktree remove`; kalan `.DS_Store` ile boş klasör silindi),
`target/wt/audit-q` silindi, geçici doc günlüğü silindi. `pkill`/`killall` kullanılmadı.
