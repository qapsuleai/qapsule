# T6c — Değişimlerin telde ve günlükte ifadesi

Soru: dokümandaki bir değişim telde (`notify` / `event.kind = "change"`) ve günlükte
(`LogEntry`) nasıl ifade edilmeli?

- **A. Genel doküman farkı** — iki doküman durumu arasındaki RFC 6902 JSON Patch
  (gerçek kütüphane: `json-patch` 5.0.0), ve RFC 7396 Merge Patch.
- **B. Anlamsal işlemler** — protokolün `ChangeOp`'u (ilkel ama alan tipli) ve/veya
  üst düzey `Op`'un kendisi günlük birimi olarak.
- **C. Karma** — anlamsal işlem adı + JSON Patch yükü.

## 1. Hipotez (ölçümden önce yazıldı)

H1 — **Boyut.** Tek öğelik değişimlerde (parametre, durum, aralık) A'nın elle üretilmiş
(emit) yamaları `ChangeOp`'tan küçük ya da ona yakın olacak; çünkü `ChangeOp` `put_*`
ile tam öğe taşır, JSON Patch ise yalnızca değişen alanı. Ama dizi ortasından silme
(katman, seri) gibi işlemlerde `json-patch`'in `diff` çıktısı konumsal olduğu için
kayan her öğe için `replace` üretecek ve `ChangeOp`'tan belirgin şekilde büyük olacak.
Merge Patch, dizileri bütün olarak değiştirdiği için bar eklemede ve katman işlemlerinde
en büyük olacak (10 000 barda bar eklemede yüz KB mertebesi).

H2 — **Hesap maliyeti.** Fark almak (`to_value` + `diff`) belge boyutuyla doğrusal; 10 000
barda tek canlı bar güncellemesi için milisaniye mertebesinde olacak, `ChangeOp` üretmek
mikrosaniyenin altında. 60 Hz sıcak yolda fark alma bütçenin (16,7 ms) belirgin bir
kısmını yiyecek ama tek başına imkânsız olmayacak.

H3 — **Uygulama maliyeti.** İstemci yansısında `json_patch::patch` ile `ChangeOp`
uygulaması küçük değişimlerde aynı mertebede olacak; fark, istemcinin `Value` mi yoksa
tipli `Document` mi tuttuğundan gelecek.

H4 — **Eşzamanlılık.** Dizi indeksli JSON Patch, iki eşzamanlı katman silmede ve dizi
kaymasında sessizce yanlış öğeyi değiştirecek. `test` işlemiyle korunmuş JSON Patch
yanlışı reddetmeye çevirecek. Ama protokolün kendi `remove_layer {view, index}` işlemi
de indeksli olduğu için `base` verilmezse **o da** yanlış sonuç verecek; anlamsal
yaklaşımın üstünlüğü indeks yerine kimlik kullanıldığı yerde (seri silme, `reject_finding`,
`set_params`) ortaya çıkacak.

H5 — **Geri alma.** RFC 6902 işlemleri önceki değeri taşımadığı için tersleri yalnızca
önceki durum elde varken türetilebilir; eski bir girdinin ters yaması, araya giren dizi
kaymalarından sonra yanlış yere uygulanacak. `ChangeOp` tersleri kimlikle çalıştığı için
seri/bulgu düzeyinde doğru kalacak; katman düzeyinde (`delete_layer {index}`) aynı kırılganlığı
taşıyacak ve ancak `undo_conflict` ile güvenli olacak.

H6 — **Niyet ve denetim.** "reject finding" gibi bir işlem JSON Patch'te iki-üç anonim
`replace`/`add` olarak görünecek; okuyan insan ya da ajan ne olduğunu ancak önceki
durumu açarak anlayabilecek. `Op` en okunur, `ChangeOp` ikinci, A en az okunur olacak.

H7 — **Doğrulama.** A'da otorite alan kurallarını (katman var olan seriye mi bakıyor,
seri kullanımda mı, barlar artan mı, kapsül durum geçişi geçerli mi) ancak yamayı
uygulayıp sonuç belgeyi baştan doğrulayarak kontrol edebilecek: maliyet belge boyutuyla
doğrusal, hata mesajı isteğe değil sonuca işaret eder ve geçiş kuralları (önce/sonra)
zahmetli olur. B'de her işlem kendi kuralını O(1)–O(öğe) maliyetle kontrol eder.

H8 — **Yeniden oynatma.** İki form da deterministik olacak; ancak `Op` günlüğü yeniden
oynatma için açılım kodunun **aynı sürümünü** gerektirecek (varsayılan parametre ya da
panel kuralı değişirse sonuç kayar), `ChangeOp` ve JSON Patch gerektirmeyecek.
`serde_json::Value` anahtarları alfabetik sıraladığı için JSON Patch yansısının baytları,
tipli dokümana geri çevrilmeden otoriteyle aynı olmayacak.

H9 — **Ajan yazabilirliği.** Ajan `Op`'u en kolay doğru üretir: kısa, kimlikle, durum
bilgisi istemez. JSON Patch yazmak indeks, güncel uzunluk, zaman damgası ve köken
bilgisini bilmeyi gerektirir.

Beklenen öneri: **telde ve günlükte `ChangeOp` (B), istekte `Op`; JSON Patch yalnızca
içe/dışa aktarma ve hata ayıklama aracı.** Katman işlemlerinde indeks yerine kararlı
kimlik eksikliği B'nin zayıf noktası olarak raporlanacak.
