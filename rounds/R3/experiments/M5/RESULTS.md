# M5 — Sonuç tanımı: sonuçlar ve seçim

Düzenek `harness-sim` (`176f77e`, alternatiflerden önce). Alternatifler ayrı dallarda, ayrı
alt ajanlarca, birbirini görmeden: **M5a `barrier`** (`r3/m5a-barrier` `ab4f689`), **M5b
`semantic`** (`r3/m5b-semantic` `60ad74e`). Değerlendirme dalı `r3/eval-sim`; ajanların ham
kayıtları orkestratörce yeniden üretildi (bayt eşit). Bütünleşik yeniden koşum (SPEC "Ortak
ilkeler"): tema `kinddir` (M4 seçimi) ile `raw-chosen/`.

## Ham sonuçlar (başvuru teması → `raw/`; seçilen tema `kinddir` → `raw-chosen/`)

| Ölçü | barrier | semantic |
|---|---|---|
| İsabet, sentetik s1–s3 (kinddir) | 0,470 / 0,457 / 0,454 | 0,467 / 0,455 / 0,457 |
| Süresi dolan payı, sentetik | **0,083–0,089** | 0,093–0,103 |
| Gerçek − yalancı eş, sentetik (beklenen ≈ 0) | **0,024 / 0,004 / 0,003** | 0,172 / 0,156 / 0,158 |
| Gerçek − yalancı eş, gerçek günlük / saatlik | 0,088 / 0,043 | 0,220 / 0,185 |
| Hüküm uyuşması (±%20 ufuk, ±%10 eşik) | 0,954–0,980 | 0,95–0,975 |
| maks \|p(100) − p(son)\|, sentetik | 0,070–0,151 | 0,070–0,090 |
| İleriye bakma (3 kesim × 3 tohum + gerçek) | geçti, 0 fark, 0 koruma ihlali | geçti, 0 fark, 0 koruma ihlali |
| Sıkışma (yönsüz) isabeti, gerçek günlük | 0,68 (koşulsuz taban ≈ 0,66) | 0,77–0,95 (geçersizlik yok) |

## Seçim: **M5a — oynaklık bariyeri**

- Anlamlılık testi yalnız onda geçerli: simetrik bariyerde rastgele yürüyüşte gerçek − yalancı
  eş farkı ≈ 0 (yanlış başarı göstermiyor), gerçek veride pozitif. `semantic`'in farkı sentetik
  veride de 0,16: kendi savunucusunun bildirdiği gibi yalancı eş yalnız yönü çevirip kanıtı
  çevirmediği için yapısal; taban düzeltilince gerçek günlükte ~+0,06, saatlikte ~+0,02
  (bariyerin farkıyla aynı düzey). Aşırılıkta yalancı eş isabeti 0,93–0,97, sıkışma çok kolay.
- Daha az süresi dolan; yönsüz sıkışma için bilgi taşıyan tanım.
- Kararlılık ve duyarlılık iki tanımda eşdeğer; ileriye bakma ikisinde de sıfır.

## Sınırlar

- Sentetik veride karne bir başarı oranı kanıtı değildir (TASKS); gerçek veride yönlü isabet
  0,50–0,54, karne kullanıcıya düz görünecek.
- Bariyer aşırılıkta "uzama sürdü"yü gerçekleşti sayar; cümle bunu açıkça söylemeli (S'de
  şablona eklendi). Sabitler (0,3·√h, 0,65·√h) yalnız sentetik veride ayarlandı.
- Bariyer kırılımı kırılan seviyeden değil doğum kapanışından ölçer; kapanışla karar verir.
