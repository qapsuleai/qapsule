# Qapsule gösterimi — web kabuğu (R4)

## Açmak

```sh
scripts/demo.sh                 # yerel veri varsa onunla, yoksa sentetik pazar
scripts/demo.sh --synthetic     # her zaman sentetik
scripts/demo.sh --from 2025-06-02 --speed 8
```

Tarayıcıda **http://localhost:8390/** adresini açın. İlk koşuda kabuğun çekirdeği
`wasm-pack` ile derlenir (bir dakika kadar; `cargo install wasm-pack`,
`rustup target add wasm32-unknown-unknown`). Yerel veri yoksa sayfanın üstünde
**"Sentetik veri"** yazar.

## Telefondan açmak

```sh
scripts/demo.sh --lan
```

Konsol ve sayfanın **Ayarlar** sekmesi "telefondan aç: http://192.168.x.y:8390/" adresini
yazar; telefon bilgisayarla **aynı ağda** olmalı. **Uyarı:** `--lan` ile yerel ağdaki
cihazlar gösterimi görebilir (gerçek veriyle açıyorsanız aklınızda olsun). `--lan` olmadan
gösterim yalnız bu bilgisayardan açılır (`127.0.0.1`). Telefonda ölçüm için **Ölçüm**
sekmesini açın, akışı kaydırın, bir grafiği sürükleyin, **Özeti kopyala**'ya dokunun ve
metni bize iletin.

## Kullanım

- **Akış:** kapsüller otoritenin sırasıyla; dokununca kapsül açılır, bir daha dokununca
  grafik, işarete ya da seviyeye dokununca (ya da **Kanıt**) kanıt. Grafikte sürükleyin,
  iki parmakla kıstırın (masaüstünde tekerlek). **‹ Geri** bir derinlik geri.
- **Oynatma çubuğu:** ▶/❚❚, +1 adım, hız, tarihe (`YYYY-AA-GG`) ya da adım numarasına git.
  Geri gitmek otoriteyi baştan kurar; ayarlardaki izleme listesi ve pozisyonlar silinir.
- **Ayarlar:** dil (TR/EN), tema, hassasiyet, işaret alfabesi (`siluet`/`rozet`), yüz
  tasarımı, "veri yetkisi yok" kipi (kanıtta "şu yetkiyi gerektirir"), izleme listesi ve
  pozisyonlar (**Uygula**).
- **Ölçüm:** kare süresi (durağan, kaydırırken, sürüklerken), tik maliyeti, açılış
  zamanları, paket boyutu; kopyalanabilir JSON özet.

## Testler

```sh
node shells/web/arch-check.mjs                        # mimari testi (ARCH.md)
cd shells/web && npm ci
node tests/parity.mjs ../../target/release/qp /tmp/p  # ağ + kabuksuz eşitlik
node screens.mjs ../../target/release/qp /tmp/s       # sabit anda ekran görüntüleri
node tests/metrics.mjs ../../target/release/qp 4      # ölçüm (4× işlemci kısıtı)
```

Gerçek veriyle ekran görüntüsü: `node screens.mjs <qp> ../../fixtures/local/screens --data
<bars dizini> --at 2025-06-02` (çıktı yalnız `fixtures/local/` altına; depoya girmez).
