# RG1 taşıma alt ajanı — ortak kurallar

Bu round'da **davranış değişmez**. Sen kodu yalnız **taşırsın**. Önce oku: `CLAUDE.md`,
`docs/ENGINEERING.md` §2–§4, `rounds/RG1/GOAL.md`, `rounds/RG1/PLAN.md` (özellikle §2 ve §13 —
§13 önceliklidir).

## Ortam
- Kendi worktree'nde ve dalında çalışırsın. Her cargo komutundan önce:
  `export CARGO_TARGET_DIR=/Users/arkns/Works/Mchart/qapsule/target` (ortak derleme klasörü,
  K56). Worktree içinde ayrı `target/` oluşturma. Eşzamanlı derlemeler kilit bekler; normaldir.
- **Yalnız sana verilen crate(ler)e yazarsın.** Başka crate'e dokunman gerekiyorsa dur ve
  raporla (orkestratör çözer). `Cargo.lock` değişirse olur.
- `.env.local` açılmaz; `../refs` salt okunur; `git push` yok; `main`'e birleştirme yok.

## İzinli değişiklikler (yalnız bunlar)
1. Bir öğeyi (fn, impl bloğu, tip, sabit, makro, test) **bütünüyle** başka dosyaya taşımak;
   büyük parça için `git mv`.
2. `mod` / `use` / `pub use` satırları; görünürlük (`pub(super)`, `pub(crate)`, `pub`) — en dar
   olanı seç.
3. Bir `impl X { … }` bloğunu öğe sınırında birden çok `impl X` bloğuna ayırmak.
4. Yol düzeltmesi (yalnız yol önekleri; son ad aynı kalır): `crate::a::f` → `crate::a::b::f`.
   Mümkünse yol düzeltmesi yerine eski yerde `pub(crate) use` ile yeniden bildirim kullan.
5. Yeni dosyada `//!` modül belgesi (bir iki satır: ne var, neden ayrı).
6. Klasör `README.md` (3–6 satır, Türkçe): ne var, hangi konu, ne değildir.
7. Yeni bir clippy uyarısı yalnız taşıma yüzünden çıkarsa (ör. `module_inception`,
   görünürlük genişleyince) o öğeye `#[allow(clippy::…)]` — raporda listele.
8. `include_str!`/`include_bytes!` göreli yol dizgesi, dosya derinleşince — raporda listele.

## Yasak
Fonksiyon gövdesi, ad, imza (görünürlük dışında), mantık, sabit değeri, biçim dizgesi, belge
yorumu (`///`), öznitelik (`#[serde]`, `#[arg]`, `#[cfg]` …) değişikliği; fonksiyon bölmek;
test beklentisi değiştirmek; "küçük iyileştirme". Fark ettiğin hatayı **düzeltme**, raporuna
"Fark edilenler" olarak yaz. Bir dosya ancak fonksiyon bölünerek 800'ün altına inebiliyorsa
(tek fonksiyon > 800), o fonksiyonu tek başına bir dosyaya koy ve raporla (istisna isteği).

## Hedefler (senin crate'inde)
- `src/` altında **800 satırı geçen dosya yok** (ham satır, `wc -l`); hedef ≤ 600.
- `src/` altında bir klasörde **≤ 15** `.rs` dosyası.
- `lib.rs` ve `mod.rs`: yalnız `mod`, `use`, `pub use`, öznitelik, `//!` (fn/impl/struct/enum/
  const/static/trait/macro_rules!/satır içi `mod x { … }` yok). Mantığı konuyu adlandıran bir
  kardeş dosyaya taşı. Bölme kalıbı: `foo.rs` + `foo/` (2018 düzeni; `foo.rs` mantık taşıyabilir).
- `util`, `utils`, `common`, `misc`, `helpers` adlı dosya/klasör yok (testlerde
  `tests/common` → `tests/support`; gövdelerdeki `common::` → `support::` yol düzeltmesidir).
- Dışa açık yollar (`qp_x::…`) değişmez: `pub use` ile koru.
- Yeni açtığın her `src/` alt klasöründe `README.md`.

## Kendi denetimin (bitirmeden önce; asıl denetimi ayrı denetçi yapar)
```
export CARGO_TARGET_DIR=/Users/arkns/Works/Mchart/qapsule/target
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -q 2>&1 | grep -E "test result|FAILED|panicked" | sort | uniq -c | tail
python3 scripts/rg1-moved.py main HEAD --list        # CHANGED 0 olmalı (izinli listelilerin dışında)
scripts/arch-check --json > /tmp/x.json              # kendi crate'inin bulgularına bak
scripts/rg1-compare.sh                               # 225 kalem bayt eşit (uzun sürer; en sonda bir kez)
```
Küçük commit'ler (dosya ya da konu başına bir), ileti Türkçe, "RG1 <görev>: …" ile başlar,
sonunda `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. `cargo fmt` biçim
değişikliği gövdeyi değiştirmez (girinti); sorun değil.

## Rapor (son cevabın; Türkçe, kısa)
Dal adı ve son commit; önce/sonra dosya tablosu (800 üstü dosyalar ve yeni yerleri); yeni
klasörler; `rg1-moved` özeti (PATH/ALLOW/izinli CHANGED listesi); kendi denetim sonuçları;
istisna istekleri; Fark edilenler; çözemediklerin.
