# E2 — Veri paralelliği (SIMD, şeritler) ve bit-eşitlik

Dal: `rp1/e2-simd` (taban `caab6e3`). Yürüten: orkestratör (alt ajan başlatılamadı, bkz. REPORT).

## Hipotezler (denemeden önce yazıldı)

**H2.1 — RSI x86'da bölme çıkış hızıyla sınırlı.** RSI kararlı durumda bar başına varyasyon
başına 4 bölme yapıyor (`avg_gain/pf`, `avg_loss/pf`, `ag/al`, `100/(1+rs)`). Profil A'da
1.000 × 10.000 × 4 = 40 M bölme. Zen 4'te skaler `divsd` ≈ 4–5 döngüde bir çıkıyor; 3,7 GHz'de
≈ 50–55 ms. CI ölçümü 62,9 ms; yani RSI'nin süresinin çoğu bölme. M4'te bölme çıkışı daha
yüksek (22 ms).
Beklenti: SoA şerit durumu (`[f64; L]`) ve dallanmasız kararlı döngü ile LLVM 2 genişlikli
`divpd` (SSE2/NEON/simd128) üretir; x86'da RSI ≈ 2 kat hızlanır (≈ 30 ms). AVX2 (4 genişlik)
çalışma anında seçilebilirse ≈ 4 kat (≈ 15 ms).

**H2.2 — Bit-eşitlik korunur.** IEEE `+ - * / sqrt` her şeritte doğru yuvarlanır;
SIMD ile skaler aynı işlem sırasında aynı biti verir. Bozan şey yalnızca FMA birleştirmesi
ya da yeniden sıralama. Rust/LLVM `contract` bayrağı olmadan `fmul+fadd`'i birleştirmez;
AVX2+FMA hedef özelliği açılsa da. Testle ve CI özetiyle doğrulanacak.

**H2.3 — `rsi_from_avg` dallanmasız yazılabilir ve aynı biti verir.** `al == 0, ag > 0` için
`100 - 100/(1 + ag/0)` = `100 - 100/inf` = `100` (tam). Yalnızca `ag == al == 0` (0/0 = NaN)
seçimle 50'ye çevrilmeli. Seçim (select) bit kopyalar; sonuç tek çalıştırmayla aynı.

**H2.4 — Isınma ayrı ele alınırsa kararlı döngü vektörleşir.** Bir şerit grubunda en büyük
periyoda kadar her şerit skaler (`peek1`/`commit1`) adımlanır; sonra hepsi kararlı durumda
olduğu için tek bir dallanmasız döngü kalır. Izgarada ardışık periyotlar gruplandığı için
skaler kısım küçük.

**H2.5 — EMA ve SMA gecikme zinciriyle sınırlı.** EMA bar başına `x*a + prev*(1-a)` (çarp +
topla zinciri ≈ 7–8 döngü), SMA iki toplama. Çok şerit (8) bu zincirleri örtüştürür; SoA ile
2 genişlikli vektörde 8 şerit = 4 vektör. Beklenti: EMA x86'da 21,5 → ≈ 8–10 ms; SMA küçük
kazanç (zaten 15–17 ms; girdi yükleri şerit başına farklı ofsetten).

**H2.6 — "Seri şeritleri" profil B'yi hızlandırır.** B'de her seride tek parametre kümesi var;
bugün şerit yok (RSI B = 64 ms yerel, 98 ms CI). Aynı uzunluktaki serileri (B'de her zaman
dilimi 500 seri) aynı parametreyle şeritlere koymak A'daki kazancı B'ye taşır. Bunun için
çekirdek her şerit için ayrı girdi dilimi almalı (`[&[f64]; L]`); varyasyon şeritleri bunun
özel durumu (hepsi aynı dilim).

**H2.7 — Şerit sayısı mimariye göre ayarlanmalı.** x86_64 temelinde 16 xmm yazmacı var;
aarch64'te 32. Durum alanı çok olan hesaplarda (RSI: ag, al, pm1, pf + çıktı) büyük L
x86'da taşmaya girer.

**H2.8 — wasm `simd128`.** Aynı SoA kod `-C target-feature=+simd128` ile derlenirse V8'de
`f64x2` kullanılır; RSI ve EMA WASM'da ≈ 1,5–2 kat. (Derleme ayarı `qp-wasm` / iş alanı
kapsamında; yalnızca öneri olarak ölçülür.)

## Deneyler

(aşağıda)

### Deney 2.1 — SoA şerit çekirdekleri (beş hesap) ve hedefe göre şerit sayısı

**Yapılan.** `crates/qp-compute/src/lanes.rs`: `sma`, `ema`, `rsi`, `macd`, `bollinger` için
`L` şeritli yapı-dizisi çekirdekleri. Her şeridin ısınması hesabın kendi `peek1`/`commit1`
(Bollinger'de `step1`) yoluyla skaler koşar; tüm şeritler kararlı duruma geçince tek bir
dallanmasız döngü, skaler kararlı durumun işlemlerini aynı sırayla şerit şerit uygular.
Dallar seçime (select) çevrildi: RSI'de `rsi_from_avg` (50 / 100 / formül), Bollinger'de düz
pencere (iki M2 formülü de hesaplanır, biri seçilir), sayaç (`run`, `f64` olarak; 2^53'e
kadar tam), `var > 0`, `middle == 0`. `sweep.rs`'deki eski AoS şerit kodu (`Kernel`,
`step_lanes`) kaldırıldı; tarama artık bu çekirdekleri kullanıyor. Kısa kuyruk, boş şeritle
doldurmak yerine daha az şeritle koşuyor (ilk sürümde doldurma tek varyasyonlu çağrıları
4 kat yavaşlatmıştı — ölçümle bulundu, düzeltildi).

**Şerit sayısı** hedef başına (`cfg(target_arch)`), ölçümle seçildi (`sweep_bench`'in yeni
"profile A by lane count" tablosu; CI'da da basılıyor):

| Hedef | SMA | EMA | RSI | MACD | Bollinger | Kaynak |
|-------|----:|----:|----:|-----:|----------:|--------|
| aarch64 (M4 Max) | 4 | 4 | 4 | 2 | 2 | yerel tablo |
| x86_64 | 2 | 4 | 4 | 2 | 2 | CI (AMD EPYC 9V74) tablosu |
| wasm32 (V8) | 1 | 2 | 2 | 1 | 1 | Node 24 tablosu |

8 şerit her yerde, çok alanlı hesaplarda (MACD, Bollinger) 4 şerit de vektör yazmaçlarını
taşırıyor ve 2–10 kat yavaşlatıyor (ör. x86 Rosetta MACD 4 şerit 244 ms, 2 şerit 14 ms).

**Ölçüm, profil A** (medyan ms; taban = `caab6e3` ikilisi, aynı oturumda dönüşümlü):

| Hesap | aarch64 taban → E2 | x86_64 CI taban → E2 | WASM Node 24 taban → E2 | WASM CI (Node 22) taban → E2 |
|-------|------------------:|---------------------:|------------------------:|----------------------------:|
| SMA | 15,3 → 5,1 | 17,4 → 17,1 (2 şerit 16,3) | 16,2 → 13,8 (1 şerit) | 26,4 → 26,4 |
| EMA | 9,1 → 4,4 | 21,6 → 6,4 | 26,5 → 13,2 | 37,6 → 30,6 |
| RSI | 24,6 → 17,4 | 62,9 → 47,8 | 57,3 → 34,3 | 118,5 → 70,7 |
| MACD | 41,4 → 10,0 | 69,1 → 16,4 | 76,3 → 23,7 | 159,9 → 37,1 |
| Bollinger | 44,7 → 28,9 | 72,0 → 67,8 (2 şerit) | 70,2 → 49,5 | 120,7 → 102,6 |

(x86_64 CI E2 sütunu `rp1/e2-simd` ilk gönderiminin çalışması 43620480005'ten; o çalışmada
x86 şerit sabitleri henüz [4,4,4,2,1] idi, tablo sütunları en iyi şerit değerini gösteriyor.)

**Profil B (seri şeritleri, `sweep_series_each`)**, medyan ms:

| Hesap | aarch64 taban → E2 | x86_64 CI taban → E2 |
|-------|------------------:|--------------------:|
| SMA | 15,3 → 7,7 | 17,8 → 20,4 |
| EMA | 28,5 → 6,7 | 40,4 → 12,5 |
| RSI | 64,0 → 19,9 | 98,3 → 52,1 |
| MACD | 45,1 → 51,9 (şerit 4'te; 2 ile tekrar ölçülecek) | 89,0 → 25,7 |
| Bollinger | 43,5 → 118,0 (şerit 4'te) | 114,0 → 83,7 |

B satırlarının bir kısmı şerit sabitleri ayarlanmadan önce ölçüldü; birleştirme sonrası
tablo `RESEARCH.md`'de.

**Bit-eşitlik kanıtı.** `tests/lanes.rs`: beş hesap × rastgele, karışık, tekrarlı parametre
listeleri × şerit 1/2/3/4/8 ve varsayılan × n = 0, 1, 7, 120, 900 (düz bölgeler, sıfır
fiyatlar, sıçrama) — tek çalıştırmaya karşı bit bit; `sweep_series_each` farklı uzunlukta
(0 ve ısınmadan kısa dahil) 101 seri, sıra korunuyor. x86_64 (Rosetta, SSE2 yolu) üzerinde de
koşturuldu. Profil özetleri A/B/C/D ve `cross_target_digest` = `7b1e99d40889f83c` aarch64,
x86_64 (Rosetta ve gerçek CI Linux) ve wasm32'de aynı. x86 çıktısında `divpd` var, FMA yok
(`objdump`).

**Karar: al.**

### Deney 2.2 — x86_64'te çalışma anında AVX2 (pulp), `unsafe` olmadan

**Yapılan.** `pulp` 0.21.5 hedefe özel bağımlılık olarak eklendi; çekirdek çağrısı
`pulp::Arch::new().dispatch(|| ...)` içine alındı, çekirdekler `#[inline(always)]`.

**Bulgu.** `pulp` crate'imizde `unsafe` gerektirmeden derleniyor (`forbid(unsafe_code)`
altında küçük bir karalama crate'iyle de doğrulandı). Ancak kapanış (closure) yolu hedef
özelliklerini çekirdeğe taşımıyor: V3 trampolini (`V3::vectorize::imp_fastcall`, 48 bayt)
kapanışı `FnOnce::call_once` ile satır dışı çağırıyor; ikilide **hiç VEX komutu yok**
(`objdump`: `vdivpd` 0, `ymm` 0). Küçük bir karalama çekirdeğinde AVX kodu çıkmıştı ama orada
da LLVM 4 şeridi iki `xmm` bölmesine bölmüştü (`ymm` değil).

**Karar: alma (şimdilik), sonra.** AVX2 için iki yol var: (a) çekirdekleri `pulp`'ın açık
SIMD tipleriyle (`WithSimd` + `S::f64s_*`) yeniden yazmak — güvenli ama her hesap için ayrı
bir yazım; (b) `#[target_feature]` + `unsafe` çağrı — `forbid(unsafe_code)` kuralının
değiştirilmesini gerektirir (ana akış kararı). Beklenen kazanç RSI'de ≈ 1,5–2 kat (x86'da RSI
bölme çıkışıyla sınırlı). Bağımlılık ve `Cargo.lock` değişikliği geri alındı. Rosetta AVX'i
çevirmediği için yerelde ölçülemiyor; CI'da ölçülmeli.

### Deney 2.3 — wasm `simd128`

**Yapılan.** Aynı kod `RUSTFLAGS="-C target-feature=+simd128"` ile derlendi (yalnızca ölçüm;
derleme ayarı `qp-wasm`/iş alanı kapsamında).

**Ölçüm** (Node 24, medyan ms, şerit sabitleri o sıradaki [4,4,4,…]): SMA 17,4 → 15,6;
EMA 13,9 → 13,4; RSI 34,4 → 35,6; MACD 77,1 → 78,4; Bollinger 69,6 → 71,4. Fark gürültü
düzeyinde.

**Karar: alma.** LLVM'in wasm SLP vektörleştiricisi bu döngülerde `f64x2` üretse de V8'de
kazanç yok; asıl kazanç şerit sayısının wasm'a göre ayarlanmasından geldi (2.1).

### Şimdi en yavaş olan

- x86'da RSI (47,8 ms): bar başına şerit başına 4 bölme (`ag/pf`, `al/pf`, `ag/al`,
  `100/(1+rs)`); bit-eşitlik bölmeyi çarpmaya çevirmeye izin vermiyor. SSE2'de 2 genişlikli
  `divpd` ile ≈ 1,3 kat kazanıldı; sınır bölme birimi. AVX2 (2.2) ya da algoritmik bir değişiklik
  (bölme sayısı) gerekir.
- Bollinger: bar başına 1 bölme + 1 karekök + iki M2 formülü (seçim için ikisi de hesaplanıyor)
  ve 4 çıktı yazımı; x86'da 68 ms. Burada asıl kazanç E1'in (k'ler arası paylaşım) alanı.
- SMA x86'da kazanmadı (17 ms): şerit başına farklı ofsetten yük + bir bölme; zaten yazma
  bant genişliğine yakın.
