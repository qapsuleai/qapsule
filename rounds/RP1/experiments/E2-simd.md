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
