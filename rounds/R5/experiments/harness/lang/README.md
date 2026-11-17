# M17 dil düzeneği — nasıl koşulur

Şartname ve seçim kuralı: `SPEC.md`. Kod: `tool/` (ana çalışma alanının dışında ayrı bir Cargo projesi,
kendi `Cargo.lock`'u; ürün crate'lerine yol bağımlılığı). Ham büyük çıktılar `out/` altında (izlenmez).

```sh
export CARGO_TARGET_DIR=/Users/arkns/Works/Mchart/qapsule/target/wt/t2-lang   # worktree başına ayrı alt klasör
cd rounds/R5/experiments/harness/lang
(cd tool && cargo build --release --offline && cargo test --release --offline)
B=$CARGO_TARGET_DIR/release/langh

# 1) Döküm + tablo + deterministik taramalar (saniyeler; ücretsiz)
$B dump --lang r4    --out out/r4      # başvuru (bugünkü dil)
$B dump --lang bozuk --out out/bozuk   # düzeneğin bozuk dili
#   out/<ad>/TABLE.md   dedektör × aşama × geçerlilik × dil tablosu (docs/LANGUAGE.md'ye girer)
#   out/<ad>/SCAN.md    tarama bulgu sayıları (hedef: hepsi 0) ve örnekler; findings.json tam liste

# 2) Kör okuyucu + eleştirmen planı (yalnız dosya yazar)
$B plan --run runs/<koşu> --langs r4,bozuk,<aday1>,<aday2>,<aday3> --readers 3 --reps 3 --seed <s> --model haiku
DRY_RUN=1 ./run_readers.sh runs/<koşu>   # yalnız istem körlük denetimi, çağrı yok

# 3) Koşu (claude -p; boş geçici dizin, araçsız, tek tur) ve puan
TMPDIR=<scratchpad> ./run_readers.sh runs/<koşu>
$B score --run runs/<koşu> --seed <s>    # runs/<koşu>/RESULTS.md, scores.json
```

## Yeni bir dil (M17 alternatifi) eklemek

1. Alternatif dalında şablonları değiştir (`crates/qp-capsule/`, gerekiyorsa yüz başlık şablonu).
2. `tool/src/adapters/<ad>.rs`: `Language` uygula — `surfaces(&Situation, Lang) -> Surfaces` yalnız ürün
   işlevlerini çağırır (başlık, ayrıntı, karne cümlesi, o ekranın sözlük girdileri). Örnek: `adapters/r4.rs`.
3. `tool/src/adapters/mod.rs`'ye `pub mod <ad>;`, `tool/src/lang.rs::registry()`'ye bir satır.
4. `langh dump --lang <ad>` → taramalar; `langh plan --langs r4,bozuk,<ad>,…` → okuyucular.
Başka düzenek dosyası değişmez. `<ad>` sıradan bir sözcük olmamalı (istemde geçmemeli; `check-prompt`
dil adlarını yasaklar).

## Maliyet

Kuru koşu (`runs/dry2`): 2 dil × 2 persona × 2 koşul × 2 sayfa × 3 okuyucu = 48 okuyucu + 1 çift × 3 tekrar
× 2 sıra = 6 eleştirmen; `haiku`; **0,158 $** (≈ 0,003 $/koşu), ≈ 3 dk (5 eşzamanlı).
Tam ölçüm (3 aday + `r4` + `bozuk` = 5 dil): okuyucu 5 × 24 = 120, eleştirmen C(5,2) × 3 × 2 = 60 →
≈ 180 koşu ≈ **0,55 $** `haiku` ile; `sonnet` ile ≈ 10 kat (≈ 5–6 $).

## Kuru koşu ve başvuru taraması

- `results/r4/` — başvurunun `TABLE.md`, `SCAN.md`, `scan.json` (531 durum × 2 dil; bulgu sayıları SPEC §2).
- `results/bozuk/` — bozuk dilin taraması.
- `runs/dry2/` — okuyucu/eleştirmen kuru koşusu: istemler, ham cevaplar, `key.json` (kod → ad; okuyucuya
  gitmedi), `RESULTS.md`. Sonuç SPEC §6.
