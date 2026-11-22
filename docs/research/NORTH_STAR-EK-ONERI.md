# NORTH_STAR.md için önerilen ekler — Arkin'in onayını bekliyor

`NORTH_STAR.md` yalnız Arkin'in onayıyla değişir. Aşağıdaki metin K69–K83'ün kuzey yıldızına
yansımasıdır. Onaylanırsa `DECISIONS.md`'ye bunu anan bir karar satırı (K84) eklenir ve metin
`NORTH_STAR.md`'ye işlenir. Var olan cümleler silinmez; ekler ve iki netleştirme vardır.

## 1. "Ne yapıyoruz" bölümüne ek

> **AI-native'in üç basamağı.** (1) *Ajan anlatır:* motorun bulduğunu okur, yorumlar, haber
> verir. (2) *Ajan yönlendirir:* neyin hesaplanacağını, neyin hangi eşikle izleneceğini kurar;
> motor çalıştırır ve kanıtlar. (3) *Ajan kurar:* kime neyin hangi biçimde gösterileceğini
> seçer. Her basamakta hesap deterministik motorda kalır; yapay zekâ karar verir ve yönlendirir.

## 2. "Kimin için" bölümüne ek

> **Profesyonelin oyun alanı kalır.** Alışkın kullanıcı çizimini kendi yapar, göstergesini
> kendi koyar, yerleşimini kendi kurar; sistem bu alanı elinden almaz. Üstüne üç şey ekler:
> çizim bir niyettir (ajan için okunur bir beklenti ifadesi); ajan çizimi ve beklentiyi
> yorumlar; izleme arkadaki ajanlara devredilir. İnsanın çizimi insanındır: ajan ona onaysız
> dokunmaz.
>
> Sistem tek bir ülke için tasarlanmaz; piyasa, aracı kurum ve dil takılabilirdir.

## 3. "İlkeler" bölümüne ekler

İlke 4'ün sonuna:

> Ajan hesabı yazabilir: yazdığı her hesap tipli bir ara gösterime iner; motor koşar, maliyeti
> önceden bilir ve her değeri kökenine bağlar.

Yeni ilkeler:

> 11. **Sessizce yanlış kurmaz.** Ajan en olası okumayı kurar ve hemen gösterir; tek
>     dokunuşla düzeltilir. Güven kazanıldıkça sormaz; yalnız okumalar yön olarak ayrışıyorsa
>     sorar. İstenen şey kurulamıyorsa uyarır ve seçenek önerir.
> 12. **Geri bildirim kanalı hep açıktır.** Sistem ne yaptığını ve izlemenin sağlığını
>     sürekli gösterir.
> 13. **Tek sicil.** "Şu olacak" diyen her şey sonuç kuralıyla doğar ve sonucu ileriye
>     bakmadan kapanır. Canlı kayıt ile yeniden oynatma (geriye dönük test) kaydı hiçbir
>     zaman karışmaz. İhtimal, ölçülmüş sıklıktır.
> 14. **Ortak iş bir kez yapılır.** Herkes için aynı olan analiz bir kez üretilir, kullanıcılar
>     üye olur.
> 15. **Yapı da kabul ölçütüdür.** Kod düzeni, katman sınırları, parametre ve profil yönetimi
>     kuralla ve makine denetimiyle korunur.

## 4. "AI-native testi" tablosuna iki soru

> | 6 | Ajan neyin hesaplanacağını ve izleneceğini kurabiliyor mu? |
> | 7 | İnsanın çizimi ajan için okunur bir niyet mi; ajan sessizce yanlış kurmuyor mu? |

## 5. "Ne değiliz" bölümünde iki netleştirme

- "TradingView kopyası değiliz…" satırının sonuna:
  > Alışılmış grafik dili (fiyat ekseni, OHLCV lejantı, son değer etiketi, imleç) kopya değil,
  > alışkın kullanıcının alfabesidir; farkımız üstüne gelen anlam katmanıdır.
- "Yatırım tavsiyesi motoru değiliz…" satırının sonuna:
  > "Al / sat / tut" bir göstergenin ya da stratejinin sinyali olarak, kaynağı ve siciliyle
  > söylenir; ajan kendi tavsiyesini vermez. Sinyalden otomatik işlem varsayılan olarak kapalıdır.
