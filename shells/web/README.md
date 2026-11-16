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
gösterim yalnız bu bilgisayardan açılır (`127.0.0.1`). Telefonda ölçüm adım adım:
`rounds/R4/DEVICE.md` "Telefon yönergesi" (Ölçüm sekmesinde **Sıfırla** → durağan 10 sn →
akışı 10 sn kaydır → grafiği 10 sn sürükle/kıstır → **Özeti kopyala**).

## Kullanım

- **Akış:** kapsüller otoritenin sırasıyla; dokununca kapsül açılır, bir daha dokununca
  grafik, işarete ya da seviyeye dokununca (ya da **Kanıt**) kanıt. Grafikte sürükleyin,
  iki parmakla kıstırın (masaüstünde tekerlek). **‹ Geri** bir derinlik geri.
- **Oynatma çubuğu:** ▶/❚❚, +1 adım, hız, tarihe (`YYYY-AA-GG`) ya da adım numarasına git.
  Geri gitmek otoriteyi baştan kurar; ayarlardaki izleme listesi ve pozisyonlar silinir.
- **Ayarlar:** dil (TR/EN), tema, hassasiyet, işaret alfabesi (`siluet`/`rozet`), yüz
  tasarımı, akış düzeni, geçiş, sonuç işareti (seçenekler çekirdeğin kayıtlarından; URL ile de:
  `?alphabet= &face= &feed= &transition= &outcome=`), "veri yetkisi yok" kipi (kanıtta "şu yetkiyi gerektirir"),
  **Son bakışımı şimdi işaretle** (oynatma saatinde; akış düzeni "son baktığından beri"yi buna
  göre gösterir), izleme listesi ve pozisyonlar (**Uygula**).
- **Varsayılan görünüm (R4/S1, S2):** hiçbir şey seçilmezse gösterim M8–M11 seçimleriyle açılır:
  yüz `vadrel`, akış `elikov`, geçiş `zumlak`, sonuç işareti `tg5`. Öteki kayıtlı tasarımlar
  (yüz: `r3`, `fotolay`, `tavrik`) Ayarlar'dan ya da URL'den seçilir; R3 görünümü:
  `?face=r3&feed=r3&transition=r3&outcome=r3`.
- **İnsan testi için URL (R4/S1):** `?seen=now` (açılışta son bakış o anki oynatma saati) ya da
  `?seen=600` (oynatma 600. adımda ya da ötesinde **ilk durduğu** anın saatiyle: doğrudan 609'a
  atlanırsa son bakış 609 olur, bu yüzden önce `600`'e gidin); `?watch=SYN02,SYN03&long=SYN09&short=SYN12`
  (`&group=future` isteğe bağlı) izleme listesini ve pozisyonları kurar (R4/S2: listede
  yalnız gösterimin enstrümanları kalır, her biri bir kez; yanlış yazılan sembol atılır). Bunlar her başlangıçta
  yeniden uygulanır: geriye sarmak otoriteyi baştan kurar, URL'deki kitle ve son bakış yine
  gelir (Ayarlar'dan elle girilenler gelmez). Örnek:
  `http://localhost:8390/?watch=SYN02,SYN03&long=SYN09,SYN12&seen=600`, oynatma çubuğundan
  önce `600`, sonra `609`.
- **Ölçüm:** kare süresi (durağan, kaydırırken, sürüklerken, yakınlaştırırken; p50/p95/p99,
  16,7 ve 33,3 ms'yi aşan kare oranı), tik maliyeti, yüz ve sahne boyama, açılış zamanları,
  paket boyutu, bellek (JS yığını — yalnız Chromium —, WASM belleği), cihaz bilgisi (çekirdek
  sayısı, ekran, DPR); **Sıfırla** kare ve süre serilerini baştan başlatır; **Özeti kopyala**
  JSON özeti panoya koyar (pano izni yoksa metin kutusunda seçili gelir).

## Testler

```sh
node shells/web/arch-check.mjs                        # mimari testi (ARCH.md)
cd shells/web && npm ci
node tests/parity.mjs ../../target/release/qp /tmp/p  # ağ + kabuksuz eşitlik
node tests/settings.mjs ../../target/release/qp       # (R4/S1) Ayarlar, varsayılanlar, ?seen= ?watch= ?long= (verify adım 18)
node screens.mjs ../../target/release/qp /tmp/s       # sabit anda ekran görüntüleri
node tests/metrics.mjs ../../target/release/qp 4      # ölçüm (4× işlemci kısıtı)
node tests/device.mjs ../../target/release/qp          # (R4/D) cihaz ölçümü: 1×/4×/6× kısıt, DPR 1/3, varsayılan ve r3, 3 tekrar, medyan (~20 dk)
```

Gerçek veriyle ekran görüntüsü: `node screens.mjs <qp> ../../fixtures/local/screens --data
<bars dizini> --at 2025-06-02` (çıktı yalnız `fixtures/local/` altına; depoya girmez).
