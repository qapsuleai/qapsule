# RP2 — Son denetim (T11)

Denetçi: bağımsız alt ajan (bu round'un hiçbir işini yazmadı). Denetlenen: `rp2/int` @
`5319cf0` (`main..5319cf0`, 85 commit, 187 dosya). Ayrı worktree
`.claude/worktrees/rp2-audit-final`, dal `rp2/audit-final`, temiz hedef klasörü (sıfırdan
derleme). Bu dosya dışında hiçbir şey commit'lenmedi; denetim sırasında harness'e eklenen
geçici izleme satırları geri alındı (`git checkout`), ölçümler ve senaryolar
scratchpad'de. Ortam: Apple M4 Max, Darwin 27.0.0, rustc 1.92.0, Node 24.3.0.

## Hüküm: **koşullu kabul**

Mühendislik yüzü (veri bağı, barsız anlık görüntü, artımlı canlı yol, yetki süzgeci,
dondurulmuş kanıt, yerel ↔ WASM yansı) istenen işi yapıyor ve bütün sayıları bağımsız
olarak yeniden ürettim: `QP_VERIFY_BROWSER=1 scripts/verify.sh` **15/15 PASS**; 36 döküm ×
3 taşıma; hesap özeti ve altın görseller farksız; tik maliyeti 54 hücrenin hepsinde
T5-fixes'in ±%5,8'i içinde ve hedefte (en kötü 2,54); 10.000 barlı anlık görüntü 2.744 B;
yansı tapesi 4 gecikmede yerel = WASM. Tutarlılıkta seçilen yaklaşım (`b`, `bs`) 24 hazır
senaryoda ve benim yazdığım 3 senaryoda **yanlış/erken gösterilen bulgu 0**.

Koşullar (round kapanışından önce):

1. **K1 — Kapanış belgeleri eksik (ölçüt 12).** `rounds/RP2/REPORT.md` yok (PROTOCOL §15 ve
   DECISIONS "Önerilen (RP2)" ona başvuruyor); `STATUS.md` ("RP2 koşulmayı bekliyor") ve
   `ROADMAP.md` ("Hazır, koşulacak") güncellenmemiş. CLAUDE.md "Round kapanışı" 2–3.
2. **K2 — O1 REPORT'ta açıkça yazılmalı** (aşağıda): seçilen B'nin karşılaştırmadaki
   üstünlüğünün bir parçası yansı → otorite `dispute`/`check` yoluydu; `qp/0` v0'da bu yol
   yok (§19.8) ve düzenekteki `bs` otoritesi gerçek `qp_session` otoritesi değil, B'nin
   düzenek otoritesi. Benim senaryomda bu, otoritenin kendi bozuk beslemesinden çıkardığı
   yanlış bir **kesin** iddiayı hiç geri çekmemesine ve ~20 dakika susmasına yol açtı
   (yansı doğru biçimde `disputed` gösterdi; yanlış gösterim yok). Karar ya da R3 işi olarak
   `DECISIONS.md` Açık bölümüne girmeli.

## VERIFY tablosu

| # | Kontrol | Sonuç | Kanıt |
|---|---------|-------|-------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | **PASS** | Çıkış 0, 15/15 (özet aşağıda), toplam 428 s |
| 2 | preflight + CI | **PASS / yok** | `scripts/preflight.sh` → "preflight: TAMAM" (origin, ssh, qgh hesabı, yazma izni, e-posta). `rp2/int` itilmediği için CI koşusu yok (`qgh run list --branch rp2/int` boş). `main`'in son koşusu: `completed success` "R2 report: CI runs on r2/int and main" (43660409673, 2026-11-15T00:53Z) |
| 3 | `qp conform` üç taşıma; değişen fikstürler | **PASS** | verify adım 14: 36 döküm × inproc, serve, mcp; ayrıca `qp conform … --transport serve` 36/36. `git diff --diff-filter=M main..5319cf0 -- fixtures`: yalnız `11-capability-depths.json` (bir `note` metni) ve `fixtures/views/d2-chart.explain.txt` (katman adları `l0/l1/l2` → `candles/line/level`); ikisi de K25 (4) ve (3) gereği, Ö27'de gerekçeli. Diğer 12 dosya yeni (döküm 28–36, iki doküman) |
| 4 | Hesap özeti, altın görseller | **PASS** | `git diff main..5319cf0 -- fixtures/golden fixtures/compute` boş (0 B); `crates/qp-compute` farksız; verify adım 12 `7b1e99d40889f83c` arm64 / x86_64 / wasm32; adım 6, 7, 10 altınlarla bayt eşit |
| 5 | Artımlı = baştan özellik testi | **PASS** | `cargo test -p qp-session --test live` 3/3 (inline tohum 1–6, bound 11–16, 600 adım; açık `seq`'li geri alma, kapsül yeniden canlanması, `claims` aboneliği dahil); `--test t5_fixes` 4/4 |
| 6 | Tik maliyeti yeniden ölçümü | **PASS** | `OUT=… BEFORE=0 WATCHES=t6d,base,all ./run-live.sh` (yerel + WASM opt-s + WASM release, iki koşu). 54 hücrenin hepsi T5-fixes §6'nın ±%20'si içinde: **en büyük sapma %5,8** (yerel bound N=500 `base` 9,05 / 8,55 µs). Hedef (payda: bağımlı bütün hesapları tam sütunda ilerletmek + aynı mesajı çözmek): 54/54 ≤ 3, en kötü **2,54** (WASM opt-s N=500 `all`, inline ve bound). Her koşuda `session_eq_core true` |
| 7 | 10.000 barlı bağlı doküman anlık görüntüsü | **PASS** | `cargo run --release -p qp-session --example bound_snapshot -- 10000 15`: **2.744 B** (T4 ile bayt aynı; örnek barsızlığı `assert` eder), hazır olma 0,481 ms (T4 0,50) |
| 8 | Yetkisiz veri bağı | **PASS** | Döküm 31 üç taşımada; kendi denemem (aşağıda "Yetki"): `study`/`hit`/`verify_evidence`/koşul aboneliği/`add_fact` tanımlı `no_entitlement`, belge içi görünüm ve panel çalışıyor, karışık panelde ölçek yetkisiz için yalnız belge içi seriden |
| 9 | `verify_evidence` | **PASS** | Döküm 32; kendi denemem: rsi14 bar 100 ve px bar 50 olguları `verified`; bar 101'in kapanışı düzeltilince ikisi de `verified` (önek aralığı dışında); bar 100 düzeltilince `changed` ("data digest 9a54…→6d2b…: a bar in t=…1757548800000 changed") |
| 10 | Tutarlılık senaryoları, seçilen yaklaşım | **PASS** | `harness run --approach b|bs --scenario all` (varsayılan tohumlar): 24 senaryoda M1 = M2 = M3f = M3p = 0. `bs` kaçırmaları T8 ile aynı (S16 284, S18 1.448: `qp/0` koşulu olmayan dedektörler, belgeli) |
| 11 | Denetçinin kendi senaryosu | **yapıldı** | AUD1–AUD3 (aşağıda): `b`, `bs`, `a` sert hata 0; `c` 278–313 yanlış kesin; `d` M3f 5 ve M3p 12–34. Bulgu O1 buradan |
| 12 | Otorite (yerel) ↔ yansı (WASM) | **PASS** | verify adım 15; ayrıca `qp replica-tape --lag 0/3/25/100` + `node crates/qp-wasm/tests/replica_tape.mjs`: WASM = yerel (doküman, görünüm, görüntü) 186/186, 189/189, 211/211, 286/286; lag 0'da WASM = otorite 186/186; ihlal 0, 9 kesin iddianın 9'u sonda kesin |
| 13 | Etkilenmeyen görünüme mesaj yok | **PASS** | `a_tick_wakes_only_affected_views_and_watchers` geçer; döküm 33 (THYAO olayları yalnız `a`'ya, ASELS olayı yalnız `b`'ye `view_changed`/`condition_met`); kendi denemem: yetkisiz `viewer`'ın `main` görünüm aboneliğine tikte hiçbir şey gitmedi |
| 14 | Yelpaze, `screen` ve `hit` | **PASS** | Döküm 29: bakış 80..219 (son bar 199), `screen` bant katmanları `items: 21`, `hit` (1052, 201) → bar 210, `b30` bandı |
| 15 | Bulgu belirtilmeden `present`, iki canlı bulgu | **PASS** | Döküm 28: `ambiguous`, adaylar `fnd-overbought`, `fnd-momentum` (artan sıra) |
| 16 | Gizli bilgi; eski klasörler | **PASS** | Diff'te `.env`/anahtar/token adı ya da içeriği yok (desen taraması 0 eşleşme; 40+ karakterli dizgeler yalnız profil sembol adları). `../Docs`, `../Services`: bugün değişen tek dosya `.DS_Store` (Finder, 17:57 / 18:03); depoda oraya yazan bir yol yok |

`verify.sh` özeti:

```
1 fmt PASS · 2 clippy PASS · 3 test PASS · 4 schema PASS (26 schemas) · 5 fixtures PASS
6 golden PASS · 7 views PASS (8 views x2) · 8 wasm PASS (gzip 598362 B) · 9 hygiene PASS
10 browser PASS · 11 compute PASS · 12 digest PASS (arm64, x86_64, wasm32: 7b1e99d40889f83c)
13 protocol PASS (61 examples) · 14 conform PASS (36 x inproc, serve, mcp)
15 replica PASS (native authority = native replica = wasm32 replica, lag 0, 25)
verify: OK
```

## GOAL kabul ölçütleri

| # | Ölçüt | Durum | Kanıt |
|---|-------|-------|-------|
| 1 | Veri bağı şemada ve şartnamede, kavramlar önce; sağlayıcıyla çözüm; `no_entitlement`/`no_data`; döküm | ✅ | `qp-schema::data`, PROTOCOL §18; sözlük commit'i `fdd755c` (11:54) şemadan `0e209be` (12:03) önce; döküm 30, 31 |
| 2 | Barsız anlık görüntü, boyut, RD1/T6d karşılaştırması, hazır olma | ✅ | 2.744 B (gzip 985) ↔ 749.600 / 210.638 B; hazır olma yeniden ölçtüm 0,48 ms (T4.md §4.2 yerel 0,50, WASM 1,1–1,3) |
| 3 | Alan düzeyinde olay, sentetik kaynak, Matriks eşlenebilirliği | ✅ | `qp-source` (`bar`/`patch`, `bar_patch_diff`), `crates/qp-source/MATRIKS_MAPPING.md`; T1-T3-T9 denetimi |
| 4 | Kopyasız tik, özellik testi, ölçüm, 3× hedef, önce/sonra | ✅ (notlu) | Yeniden ölçüm 54/54 ≤ 3 (en kötü 2,54). Not: payda round'un kendi önerisi (Ö23, tam sütun + çözme); T1 tipi yalın paydayla N=500 `all` ~5 ve 100 niyet + 100 kapsüllü `ic` düzeninde N=5/50'de 3,4–7,2 — ikisi de T5-fixes'te dürüstçe raporlanmış |
| 5 | Yalnız etkilenen uyanır; test | ✅ | VERIFY 13 |
| 6 | Dondurulmuş kanıt + doğrulama sorgusu + düzeltmede fark | ✅ | VERIFY 9, döküm 32 |
| 7 | ≥3 yaklaşım, ayrı dal/ajan; senaryolar; seçilen uygulanmış, şartnamede, önerilen | ⚠️ | Dört yaklaşım ayrı dallarda (aşağıda); senaryolar gecikme, sıra bozulması, düzeltme, geride kalma içeriyor; §19 ve Ö25. Ama: O1 (gerçek otorite düzenekte yok; B'nin itiraz yolu uygulanmadı) |
| 8 | Yerel ↔ WASM özetler aynı; geride kalan yansı yanlış göstermez | ✅ | VERIFY 12 (lag 0/3/25/100), `bs` sert hata 0 |
| 9 | K25 (a)(b)(c), değişen dökümler gerekçeli | ✅ | Döküm 28, 29, `d2-chart.explain.txt`, döküm 11 notu; Ö27. Gerekçeli liste REPORT'ta olmalı (K1) |
| 10 | R2 kalanları | ✅ | `9aa5503` (summary alan sırası), `fe67c6d` (seçicili komut tek kopya), `2706ab0` (sunucu kopyaları), `1ba46d2` (ham iletim); MACD/RSI etiketleri yeni altın gerektirdiği için öneri (T9.md §5) — kurala uygun |
| 11 | Üç taşımada dökümler; yeni dökümler | ✅ | 36 × 3; 28–36 veri bağı, yetki, kanıt, tutarlılık, belirsiz sunum, geleceğe bakış |
| 12 | REPORT, RESEARCH, AUDIT; STATUS, ROADMAP; DECISIONS ayrı bölüm | ❌ (bekliyor) | RESEARCH ✅, AUDIT (bu dosya), DECISIONS "Önerilen (RP2)" ✅; **REPORT.md yok, STATUS/ROADMAP eski** (K1) |

## Süreç kontrolleri

- **Üç yaklaşım ayrı dallarda, birbirini görmeden.** `rp2/t7a…d-approach` dördü de aynı
  düzenek commit'inden (`bf37dee`, 12:40) dallanıyor ve her biri yalnız kendi dosyalarını
  değiştiriyor (`approaches/<x>.rs`, `T7<x>.md`, `mod.rs` tek satır); hiçbir dal bir
  diğerinin dosyasını içermiyor, raporlar birbirine atıf yapmıyor (grep 0). Commit'ler
  13:17–13:29 arasında, birleştirme ancak `rp2/t7-compare`'de. Ortak bilgi kaynağı yalnız
  RESEARCH.md (A–E adayları) — tasarım gereği. "Ayrı alt ajan" git'ten kanıtlanamaz (aynı
  yazar), yapı bununla tutarlı. Düzenek yaklaşımlardan önce ve bağımsız. ✅
- **Kavramlar koddan önce.** Veri düzlemi (`fdd755c` → `0e209be`) ve tutarlılık
  (`6a2aa8f` 15:34 → T8 kodu 15:42'den itibaren) önce sözlükte. İstisna: `view` konusu
  T5 kodunda (`5b58001`) önce, sözlüğe T5 denetimi D3'ten sonra (`f569d9d`) girdi; kapandı. ✅
- **DECISIONS.** Diff yalnız yeni "Önerilen (RP2) — ana akış onayı bekliyor" bölümünü
  (Ö18–Ö28) ve Açık A15–A19'u ekliyor; "Verilmiş kararlar"a satır yok, K-satırları
  değişmemiş; NORTH_STAR, CLAUDE.md farksız. ✅ (§18–§19 normatif metin önerileri
  uyguluyor; GOAL bunu istiyor.)
- **Önceki denetim koşulları.** T1-T3-T9 O1 (`RunStep` `data`) → kapandı (döküm 30–36 veri
  adımıyla üç taşımada). T2-T6 Y1 (kalıcı çapa) → Ö19 + `anchor_t`; Y2 (`seq` hizası) →
  `bea7a7a`; Y3 (türetilmiş veri yetkisi) → §18.3 kural + Açık A15; O5 (özet algoritması) →
  Açık A19. T4 Y1 (koşul kehaneti) → kod + döküm 34 (yeniden denedim: yetkisiz `editor`
  `sm = sma(px)` üstüne koşul aboneliği → `no_entitlement`); Y2 (çapa geri alma) → döküm 34
  adım 9. T5 O1 (kapsül yeniden canlanması) → `t5_fixes.rs`, özellik testi, döküm 36; O2
  (niyet/kapsül maliyeti) → `LiveIndex`, `ic` ölçümü (küçük N'de hedef dışı, raporlu);
  D1 ısınma → ısınma geçişi; D3 → kapandı. Hepsi kapanmış ya da Açık karara bağlanmış. ✅

## Bağımsız yeniden üretim

### Tik maliyeti (µs/tik, iki koşu ortalaması, uzun döngü medyanı; oran = (oturum − çekirdek+çözme) / çekirdek+çözme)

| N | yol | yerel `t6d` / `base` / `all` | oran `all` | WASM opt-s `all` (oran) | WASM release `all` (oran) |
|---:|---|---|---:|---|---|
| 5 | inline | 2,07 / 2,05 / 2,11 | 2,25 | 3,12 (2,25) | 2,60 (2,05) |
| 5 | bound | 0,76 / 0,80 / 0,91 | 1,29 | 1,61 (1,33) | 1,25 (1,18) |
| 50 | inline | 2,69 / 2,66 / 3,55 | 1,39 | 5,69 (2,31) | 4,57 (2,02) |
| 50 | bound | 1,28 / 1,31 / 2,14 | 0,68 | 4,13 (1,89) | 3,12 (1,60) |
| 500 | inline | 10,46 / 10,66 / 21,44 | 1,64 | 36,20 (2,54) | 27,73 (2,12) |
| 500 | bound | 8,86 / 9,05 / 20,18 | 1,54 | 35,14 (2,54) | 26,42 (2,10) |

T5-fixes §6 ile 54 hücrede en büyük sapma %5,8; hiçbiri ±%20 dışında. Payda ve yöntem
T5-fixes'teki gibi. (İkinci yerel koşu sırasında kısa bir derleme çalıştı; iki koşu
arasındaki fark yine de ≤ %2.)

### Tutarlılık — hazır senaryolar

`b` (57 s) ve `bs` (250 s), 24 senaryo, varsayılan tohumlar: sert hata (M1, M2, M3f, M3p)
**her senaryoda 0**. `b` kaçırma yalnız S4'te 2 (T7-COMPARE ile aynı); `bs` S4 2, S16 284,
S18 1.448 (T8.md ile aynı). Belirleyicilik: aynı tohumlarla sayılar raporlarla birebir.
§T7-yeniden üretim: aşağıdaki "T7 karşılaştırması" bölümü.

### Kendi senaryolarım (VERIFY 11)

`harness spec S9`'dan türetildi (S9: 1.200 geçmiş bar, düzeltmeler). Ortak (AUD1):
70'te yönlendirme + kapanış yarışı (margin 1, kapanıştan 300 ms önce, %50); 50 canlı bar;
**otorite beslemesi** 20 + U(0,60) ms, %2 olasılıkla ~800 ms sıçrama, **%15 sıra bozma**
(≤600 ms), %5 yineleme, barı kapatan tikin %5 kaybı; **yansı beslemesi 1,5 s + U(0,2,5) s
geride** (%3 ~4 s sıçrama), **%25 sıra bozma** (≤1,5 s), %5 yineleme, barı kapatan tikin
%10 kaybı; **kanal** her yönde 300 + U(0,1200) ms, %5 ~3 s sıçrama, %20 sıra bozma (≤2 s),
%10 yineleme; **8 düzeltme** (canlı bar 5, 9, 13, 17, 22, 27, 33, 38: düşüren, doğuran,
koruyan, 800 bar önce) ve **bar 30'da bütün geçmiş ×0,5 (bölünme)**.

- AUD2 (düşmanca 1): AUD1 + yansı beslemesi 540–780 s kesik ve **dönüşte `REFRESH` yok**;
  kanal 600–640 s kopuk; düzeltmeler ve ×2 bölünme kesintinin içinde (bar 10–12).
- AUD3 (düşmanca 2): AUD1 + yansı fiyatları **f32** ve 250 ms **birleştirme** (yeniden
  numaralanan `seq`), kapanış yarışı margin 0,2.

100 tohum, `harness run --approach <x> --spec aud<k>.json`:

| | Senaryo | M1 | M2 | M3f | M3p | M5 kaçan/uygun | M6f p50/p95 ms | M13 uyuşmazlık s |
|---|---|---:|---:|---:|---:|---|---|---:|
| n | AUD1 / 2 / 3 | 4.596 / 5.033 / 4.661 | 5.361 / 5.320 / 5.354 | 57 / 147 / 74 | 733 / 1.025 / 808 | 70 / 148 / 81 | 1.224/4.231 · 1.219/4.356 · 1.203/4.189 | 0 |
| a | AUD1 / 2 / 3 | 0 | 0 | 0 | 0 | 0 / 143 / 0 | 3.538/5.582 · 3.843/158.192 · 4.106/6.404 | 0 |
| **b** | AUD1 / 2 / 3 | **0** | **0** | **0** | **0** | 0 / 112 / 0 | 3.335/5.887 · 3.596/63.746 · 3.292/5.060 | 32 / 4 / 39 |
| c | AUD1 / 2 / 3 | 0 | 0 | **278 / 313 / 301** | 68 / 89 / 65 | 0 / 1.308 / 2 | 3.213/4.759 · 3.596/62.811 · 3.582/4.815 | 1.064 / 2.083.801 / 4.345 |
| d | AUD1 / 2 / 3 | 0 | 0 | 0 / **5** / 0 | 34 / 12 / 27 | 0 / 49 / 0 | 3.958/6.138 · 4.168/9.947 · 4.617/6.846 | 7 / 1 / 7 |
| **bs** | AUD1 / 2 / 3 | **0** | **0** | **0** | **0** | **7** / 49 / **7** | 3.248/4.993 · 3.430/7.518 · 3.248/4.902 | **1.268** / 4 / **1.269** |

Okuma:

- **Seçilen yaklaşım (`b`, `bs`) üç senaryoda da yanlış ya da erken gösterim 0.** AUD2'deki
  kaçırmalar (`b` 112, `bs` 49) dürüst beklemedir: yansının verisi kesik ve `REFRESH`
  yok; iddia `pending` kalıyor. AUD2'de 64 tohumda Δ_stale'i (5 s) aşan bayat gösterim
  bölümü var (en uzunu ~22 s; kesinti içinde kaybolan düzeltme), `a`'da 25 — sert hata
  değil ama bilgi olarak REPORT'a.
- **C** hazır senaryolarda sert hatada 0 iken benim senaryolarımda **278–313 yanlış kesin**
  gösteriyor; **D** AUD2'de 5 yanlış kesin. Seçimi değiştirmiyor (C ve D seçilmedi), ama
  24 hazır senaryonun her yaklaşımın zayıf yanını kapsamadığını gösteriyor (sıra bozma +
  yineleme + düzeltme + bölünme bir arada hiçbir hazır senaryoda yok). D2.
- **`bs` AUD1/AUD3'te tohum 76:** O1 (aşağıda).

## Bulgular (önem sırasıyla)

### Yüksek

Yok.

### Orta

**O1 — Seçilen tasarımın kendini onarma yolu uygulanmadı; gerçek otorite düzenekte hiç
ölçülmedi.** Yeniden üretim: `harness log --approach bs --spec aud1.json --seed 76`
(AUD1 = yukarıdaki spec). Bölünme anında (canlı bar 30, `t=1700001840000`) otoritenin
**kendi** beslemesine bölünme öncesinden sıra dışı bir tik geliyor; otorite RSI'ı 4,33
hesaplayıp **`rsi_cross level=30 down` için kesin (`final`) iddia** gönderiyor (gerçekte
RSI ~71, 70'in yukarı kesişimi). Yansı kendi verisinde sınayıp doğru biçimde `disputed`
gösteriyor (yanlış gösterim yok). Sonra:

- `b`'de yansı `dispute` gönderiyor, otorite kendi aralığını yeniden çekip 2,3 s içinde
  geri çekiyor ve doğru 70-yukarı iddiayı kesin gönderiyor; sonraki 7 iddia normal.
- `bs`'de (`qp/0`'da `dispute`/`check` yok, §19.8) otorite **iddiayı hiç geri çekmiyor ve
  1.861 s'den koşu sonuna (3.090 s) kadar hiçbir iddia göndermiyor** (izleme satırlarıyla
  doğruladım: otorite bağdaştırıcısından 1.861 s'den sonra çıkan iş yok). Yansı 1.226 s
  `disputed` gösteriyor, 7 kesin iddia kaçıyor (M5), 8 uzun engelleme (M8).

Bölünme olmadan, sıra bozma/yineleme olmadan ya da yalnız bölünmeyle sorun yok
(`v_norescale`, `v_noreorder`, `v_rescaleonly`: `bs` M5 = 0); tek tek bozulma türleriyle
de yok — birleşimin sonucu. Neden önemli:

1. T7-COMPARE B'yi "kaçırma en az" diye seçti; T7b.md (yansı → otorite mesajları,
   S14 açıklaması) bu sonucun bir kısmını `dispute`/`check` + otoritenin kesin bar
   doğrulamasına bağlıyor. Uygulanan §19'da
   bu yol yok, ama Ö25 kanıt olarak B'nin sayılarını gösteriyor.
2. `bs` yaklaşımı yansı tarafında gerçek `qp_session::Replica`'yı sürüyor, otorite tarafı
   ise **B'nin düzenek otoritesi** (`approaches/bs.rs` başlığı açıkça söylüyor). Gerçek
   otoritenin `Claims::after_bars` yaşam döngüsü ağ bozulması altında hiç ölçülmedi;
   yalnız döküm 35, `claims.rs` ve `replica-tape` (bozulmasız tek akış) var. Gerçek
   otoritenin beslemesi katı (`seq` boşluğu → `stale`), o yüzden bu yolu muhtemelen aynı
   biçimde yaşamaz — ama ölçülmedi ve "kendi beslemesi bozuk otorite" (S14 sınıfı) için
   gerçek otoritede kesin bar doğrulaması da yok.

Düzeltme: (a) REPORT'ta ve Ö25'in kanıt sütununda "B'nin `dispute`/`check` yolu
uygulanmadı; S14 sınıfı hatada otorite kendini düzeltmez" açıkça yazılsın; (b) DECISIONS
Açık'a bir satır: yansı → otorite itirazı (`dispute`) `qp/1`'e mi, yoksa otoritede kesin
bar doğrulaması mı; (c) R3'te düzeneğe `bsr` yaklaşımı: otorite tarafında da gerçek
`qp_session::Session` (`--host-frames` ile düzeneğin beslemesi), AUD1 ve S14'le.

### Düşük

**D1 — Kanıt başvurusundaki veri özeti yetkisiz kişiye gidiyor; `binding` sorgusu aynı
özeti gizliyor.** Yetkisiz `viewer`: `binding` → `position: $absent` (döküm 31), ama aynı
FNV-1a 64 özeti `Fact.evidence_ref.digest` olarak `change`, `get`, `present`, `log` ve
`verify_evidence`'in `stored` alanında alıyor (kendi denememde `63b0d3e4a98c2c2d`; bu
`binding`'in yetkiliye verdiği `position.digest`'in aynısı). Özet açık bir algoritma ve
kriptografik değil; elinde gecikmeli veri olan biri bir aday kümesini sınayabilir (kapanış
fiyatı onaylama kehaneti; hacim de özette olduğu için pratikte zor). Ya `evidence_ref`
özeti de yetkisize gizlensin, ya da §18.3 tablosuna "kanıt başvurusu (özet dahil) verilir"
ve gerekçesi yazılsın; A15/A19 ile birlikte karara bağlansın.

**D2 — Hazır senaryolar karışık bozulmayı kapsamıyor.** C'nin 0 → 278–313 ve D'nin 0 → 5
yanlış kesin sıçraması yalnız birleşik senaryoda görünüyor. Öneri: AUD1–AUD3'ün
eşdeğerleri (sıra bozma + yineleme + gecikmeli yansı + düzeltme + bölünme; `REFRESH`siz
kesinti; f32 + birleştirme) hazır ayar olarak eklensin (S20–S22). Ayrıca `bs`'nin
AUD1'deki M10'u (iş/olay 47,5; AUD3'te 1.857) ve hazırlarda S12/S12s'deki ~2.460 iş/olay
`b`'nin 50–100 katı — gerçek yansının yeniden çözüm maliyeti; R4 cihaz ölçümüne not.

**D3 — Bir bitlik iddia sızıntısı yetkisize kesinlikle ve zamanla birlikte gidiyor
(bilinen, açık).** Yetkisiz `viewer` ve `editor` `claims` aboneliğinde kapsül iddiasını
`provisional` sonra `final` olarak, `t` ile alıyor; `finding_state` ve `change` da aynı
(değerler doğru biçimde çıkarılmış). §18.3 "Açık soru"da ve A15'te yazılı; yeni değil,
yalnız `maturity` bilgisinin de (barın eşiğin üstünde kapandığı) sızdığı açık soruya
eklenmeli.

**D4 — Özellik testinin gerekli olay listesinde `claim` yok.** `live.rs` `claims`'e abone
oluyor ama `property()` yalnız `ack…result`'un üretildiğini zorunlu tutuyor; `claim`
olayı hiç üretilmese test geçer. Listeye `"claim"` eklenmeli. Tohum sayısı (6 + 6 × 600
adım) küçük; CI dışı bir uzun koşu (ör. 200 tohum) önerilir.

### Bilgi

- `../Docs/.DS_Store` ve `../Services/.DS_Store` bugün değişmiş (Finder meta verisi);
  depoda oraya yazan bir yol yok.
- T7 karşılaştırması ve düzenek ayrı denetlenmemişti ("audit with T11"); aşağıda yeniden
  üretildi.

## T7 karşılaştırması (yeniden üretim)

`harness run --approach {a,b,c,d} --scenario all` (varsayılan tohumlar, S17 20) bu
worktree'de sıfırdan derlenmiş düzenekle: dört tablonun senaryo satırları
`rounds/RP2/experiments/raw/t7/table-{a,b,c,d}.txt` ile **bayt bayt aynı** (`diff` boş).
T7-COMPARE'deki sert hata = 0 (A–D), kaçırma ve gecikme sayıları doğrulandı. Karşılaştırmanın
sonucu (B) hazır senaryolarda yeniden üretilebilir; sınırı O1 ve D2'de.

## Yetki — son kırma denemesi

`31-no-entitlement` init'i (`viewer`, `editor` veri yetkisiz; `arkin` THYAO'ya yetkili),
`qp serve --host-frames`:

| Yol | Sonuç |
|-----|-------|
| `editor` `put_view` mix (aynı panelde belge içi `idx` + bağlı `px`) → `screen`, `hit` | Yetkisizin ölçeği `[9789.08, 10025.76]` (yalnız `idx`); yetkilinin `[-404.5, 10511.2]`. `hit` değerleri mix ve yalnız-`idx` görünümünde bayt aynı → ölçek üzerinden sızıntı yok |
| renk koşulu ve `y2` `px`'e başvuran belge içi katman | yetkiside katman çizilmiyor (`binding: no_entitlement`, `items: 0`), panel ölçeği yok |
| `editor` `put_series sm = sma(px)` → `study sm`, `add_fact` sm, `subscribe condition sm gt 1` | seri eklenebiliyor (veri okumaz); okuma, olgu ve abonelik `no_entitlement` (özyinelemeli girdi denetimi) |
| `viewer` `subscribe view main` + THYAO tiki | hiçbir şey gitmedi |
| `viewer` `subscribe claims`, `finding`; yetkili kapsül geçersizleşmesi | kapsül iddiası değerleri çıkarılmış (`values: [{series, t}]`), ama durum, `maturity`, `t` gidiyor (D3, açık soru) |
| `log`, `document`, `list full`, `get` | bağlı barlar yok; çapa damgası yalnız `anchor_t`; belge içi `idx` barları (beklenen) |
| `present`, `verify_evidence` | kıvılcım/görünüm yok; `no_entitlement` olgu durumu; ama `evidence_ref.digest` var (D1) |
| `binding` | durum, neden, çapa; `position` yok |

Sonuç: yetkisiz kişiye **bar değeri** veren bir yol bulamadım. Kalan sızıntılar anlam
düzlemindeki bir bitlik sonuçlar (açık soru A15) ve özet (D1).

## Kullanılan komutlar (özet)

- `QP_VERIFY_BROWSER=1 scripts/verify.sh`; `scripts/preflight.sh`; `scripts/qgh run list`.
- `cd rounds/RP2/experiments/tick-bench && OUT=<scratch>/tb BEFORE=0 WATCHES=t6d,base,all ./run-live.sh` (6 dk 38 s).
- `cargo run --release -p qp-session --example bound_snapshot -- 10000 15 snap.json`.
- `qp replica-tape --lag {0,3,25,100}`; `node crates/qp-wasm/tests/replica_tape.mjs target/wasm-replica/pkg …`.
- `cargo test -p qp-session --test live|replica|t5_fixes`; `qp conform fixtures/protocol/conformance --transport serve`.
- `harness run --approach {n,a,b,c,d,bs} --scenario all|--spec aud{1,2,3}.json`;
  `harness log --approach {b,bs} --spec aud1.json --seed 76`; daraltma için `v_*`/`w_*` türevleri.

AUD1 spec'i (S9'a göre farklar; eksik alanlar S9'unki):

```json
{"name":"AUD1","live_bars":50,
 "shape":{"kind":"steer","plan":[{"level":70,"bars":1000,"amp":2,"osc":3,"close_spread":2,"noise":0.15}],
          "race":{"margin":1,"before_close_ms":300,"cross_p":0.5}},
 "authority_feed":{"delay":{"base_ms":20,"jitter_ms":60,"spike_p":0.02,"spike_ms":800},
   "reorder_p":0.15,"reorder_ms":600,"dup_p":0.05,"dup_ms":500,"drop_last_tick_p":0.05},
 "replica_feed":{"delay":{"base_ms":1500,"jitter_ms":2500,"spike_p":0.03,"spike_ms":4000},
   "reorder_p":0.25,"reorder_ms":1500,"dup_p":0.05,"dup_ms":1000,"drop_last_tick_p":0.1},
 "a2r":{"delay":{"base_ms":300,"jitter_ms":1200,"spike_p":0.05,"spike_ms":3000},
   "reorder_p":0.2,"reorder_ms":2000,"dup_p":0.1,"dup_ms":1500,"dup_on_reconnect_p":0.3},
 "r2a":"(a2r ile aynı)",
 "corrections":[{"at_bar":5,"kind":"drop_claim"},{"at_bar":9,"kind":"create_claim"},
   {"at_bar":13,"kind":"keep_claim"},{"at_bar":17,"kind":"drop_claim"},
   {"at_bar":22,"kind":"create_claim"},{"at_bar":27,"kind":"old","bars_back":800},
   {"at_bar":33,"kind":"create_claim"},{"at_bar":38,"kind":"drop_claim"}],
 "rescales":[{"at_bar":30,"factor":0.5}]}
```

AUD2: + `replica_feed.outages [{540000, 780000}]`, `refresh_on_reconnect false`,
`disconnects [{600000, 640000}]`, düzeltmeler bar 10 (drop), 11 (create), 12 (keep),
25 (drop), bölünme bar 11 ×2. AUD3: + `replica_feed.price_repr "f32"`, `conflate_ms 250`,
`race.margin 0.2`.
