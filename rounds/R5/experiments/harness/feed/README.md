# M19 akış düzeneği — nasıl koşulur

Şartname ve seçim kuralı: `SPEC.md`. Kod: `tool/` (ayrı Cargo projesi; ürün crate'lerine yol bağımlılığı;
ürün kodu değişmez). Dil modeli yok; ücretsiz.

```sh
export CARGO_TARGET_DIR=/Users/arkns/Works/Mchart/qapsule/target/wt/t2-lang
cd rounds/R5/experiments/harness/feed/tool
cargo test --release --offline        # no_look_ahead, hidden_still_count_in_ledger, rules_judge (≈ 3 s)
cargo build --release --offline
$CARGO_TARGET_DIR/release/feedh run --out ../results/dry-run --real    # ≈ 2 dk
#   --real: fixtures/local/bars (ya da $QAPSULE_LOCAL_BARS) varsa gerçek günlük barlar; yalnız toplu
#           istatistik yazılır. Yoksa yalnız sentetik.
#   --seeds 1,2,3 --n 30 --days 500: sentetik pazar
#   --json <yol>: bütün ölçüler (büyük; depoya girmez, out/ altına yaz)
```

Çıktı `results/dry-run/RESULTS.md`: kural tablosu (kalan/gün, gizlenen/gün, boş gün payı, az örnekli payı,
kalanların / gizlenenlerin / az örneklilerin sonradan tutma oranı, kalan Brier becerisi ve aralığı, T2),
N duyarlılığı, seçim kuralının adım adım uygulanışı, sıralama × K42 tablosu (T1/T2). Aynı komut aynı
baytları verir.

**Alternatif (M19 ürün uygulaması) bağlama:** ürün kuralı burada adı geçen bir `Rule` + `Order` biçimidir;
alternatif kendi testinde aynı günlerde `feedh::feed_on` ile aynı kararları verdiğini gösterir. Yeni bir kural
gerekirse `Rule`'a bir kol ve `rule_grid`'e bir satır eklenir (SPEC §7'ye tarihli ek).
