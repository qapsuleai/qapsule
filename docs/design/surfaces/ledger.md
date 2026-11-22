# Yüzeyler — sicil, zaman, ortak iş (Y33, Y34, Y35)

Ortak kurallar ve `D-simüle` kalıbı: [`../SURFACES.md`](../SURFACES.md). Sicil = karne (`Ledger`),
kayıt `ledger entry` (K75). Değişmez kural: **canlı ve yeniden oynatma kaydı hiçbir toplamda
karışmaz; sıralama yalnız canlı kayıtla** (K75); yeniden oynatma kaydı "simüle" etiketlidir (K77).

### Y33 — Tutma yüzdesi, taban oran, düşük güven; canlı ↔ yeniden oynatma

**Ne:** Bir iddianın (kapsül, çizim izlemesi, strateji sinyali) siciline ilişkin sayılar: tutma
yüzdesi, program × yön taban oranı, örnek sayısı, aralık; az örnekte ya da taban orandan
ayrışmayan tespitte "düşük güven"; canlı ve simüle ayrı. · **Kademe:** 1 · **Kararlar:** K75,
K77, K86 (A54 türün kendi taban oranı, A57 düşük güven).

**Yerleşim:** Kapsül yüzünde tek satır karne; açılmış kapsülün alt sayfasında (Ç4) sicil bölümü;
ajan cevabında cümle (Y24); izleme ayrıntısında (Y08) "sicile yazıldı" bağı. Satır biçimi:
"Bu kalıpta tutma **%64** (n = 42, aralık %49–77) · taban oran %51".

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Canlı, yeterli örnek (n ≥ 20) | Yüzde + n + aralık + taban oran; yüzde tablo rakamı, kalın değil | `--fg`, `--qp-font-figure` | "Tutma %64 (n = 42, %49–77) · taban %51" |
| Az örnek (n < 20) | **Yüzde gizli, sayı görünür**; "düşük güven" sözcüğü | `--qp-warn` sözcük, sayı `--fg` | "Düşük güven · az örnek: 12 kayıttan 8'i tuttu" |
| Taban orandan ayrışmıyor (A54/A57) | Yüzde görünür + sözcük; elde tutulan hissede olumsuzsa bildirilir, olumluysa bildirilmez | `--qp-warn` sözcük | "Düşük güven · taban orandan ayrışmıyor (%52 / %51)" |
| Örnek yok | Satır yerine tek cümle | `--muted` | "Bu kalıp için henüz sicil yok" |
| Sonuç kuralı yok | Puanlanmaz | `--muted` | "Puanlanmıyor: sonuç kuralı yok" |
| Açık kayıt | İddia sürüyor; sonuç hücresi boş | `--muted` | "Açık · sonuç 3 bar içinde" |
| Kapandı: tuttu / tutmadı | Doluluk: dolu ● / boş ○ (anlam işareti alfabesi: doluluk = karne) | `--qp-valid` / `--qp-invalid` | "Tuttu" · "Tutmadı" |
| Yeniden oynatma (simüle) | **Ayrı satır**, başında bant sözcüğü; canlı satırın altında, asla toplamı yok | `--note-bg`/`--note-fg` | "Yeniden oynatma · simüle: tutma %58 (n = 310)" |
| Yalnız simüle var | Canlı satır "henüz canlı kayıt yok"; simüle satırı altta | `--muted` + `--note-fg` | "Canlı kayıt yok · simüle: %58 (n = 310)" |
| Kurumsal işlem sonrası | Kayıtta katsayı notu (K88) | `--muted` | "Düzeyler 1:2 bölünmeyle taşındı" |
| Yetki yok | Sayılar gizli, varlık söylenir | `--muted` | "Sicil var: 42 kayıt · görmek için BIST seviye 1" |

**Kurallar:** Eşik 20 parametredir (K78; Y37 "neden 20?"). Yüzde hiçbir zaman büyük başlık
olmaz, renkli dolgu almaz; yön rengi (`--qp-up`/`--qp-down`) kullanılmaz. "Sinyal, tavsiye
değil" ibaresi sicil gösteren her yüzün altında `--muted` (K77). Sıralı listeler (en iyi
kalıplar) yalnız canlı kayıtla sıralanır; simüle satırları sıralamaya girmez.

**Köken:** Sayılar motordur (noktalı alt çizgiyle kanıt bağı); ajan cümlesinde geçen sicil
sayısı da motor cümlesinden gelir (A61); ajan yüzde uydurmaz, yuvarlamaz.

**Veri durumu:** Canlı / simüle ayrımı bu yüzeyin kendisidir; bayatta sicil değişmez (kapanmış
kayıtlar), açık kayıtlar "değerlendirilemiyor".

**Akış:** Kapsül yüzünde satır → dokun → sicil bölümü: canlı satır, simüle satır, son 10 kayıt
(tarih, sonuç ●/○) → kayda dokun → zaman gezintisi (Y34) o aralığa gider.

**Masaüstü / telefon:** Masaüstünde satır ve açılır tablo; telefonda alt sayfada iki satır
(canlı, simüle), kayıt listesi "Tümü" arkasında; "düşük güven" sözcüğü kısaltılmaz.

**Uyarlanan:** VV-C1, VV-C2, VV-B3, AT-H2. **Roller:** `--fg`, `--muted`, `--qp-warn`,
`--qp-valid`, `--qp-invalid`, `--note-bg`, `--note-fg`, `--qp-font-figure`, `--qp-fs-trust`.

### Y34 — Zaman gezintisi

**Ne:** Geçmişe gidildiğinde o aralıktaki tespitler, sonuçları (gerçekleşmiş ve gerçekleşmemiş)
ve tutma yüzdesi. · **Kademe:** 2 · **Kararlar:** K75.

**Yerleşim:** Grafik hücresi; zaman ekseninde seçilen aralık parantezi ("Kanıt" parantezinin
biçimi) ve hücrenin üstünde tek satırlık gezinti şeridi: aralık + özet + "Bugüne dön".
Tespitler barın üstünde ≤ `chart.mark_max_px` işaretler.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Gezinti açık | Şerit + aralık parantezi | `--card`, `--muted` | "Geçmiş · 14–22 Ağu · bugünün verisi değil · Bugüne dön" |
| Gerçekleşmiş tespit | Dolu motor işareti ● | `--qp-origin-engine` + `--qp-valid` | "Kırılım · tuttu (3/3)" |
| Gerçekleşmemiş tespit | Boş işaret ○ | `--qp-origin-engine` + `--qp-invalid` | "Kırılım · tutmadı" |
| O tarihte açık olan | Yarım işaret ◐ (sonuç aralık dışında) | `--qp-origin-engine` | "Açık kaldı · sonuç 25 Ağu" |
| Aralık özeti | Şeritte Y33 kurallarıyla | `--fg` | "Bu aralıkta 6 tespit · 4 tuttu" (n < 20: yüzde yok) |
| Aralıkta tespit yok | `O-boş` şerit içinde | `--muted` | "Bu aralıkta tespit yok" |
| Yeniden oynatma imleci | `D-simüle`; imleçten sonrası gizli (S23) | `--note-bg`/`--note-fg` | "Yeniden oynatma · simüle · sonuçlar imleçten sonra gizli" |
| Yetki yok | `D-yetkiyok` | `--muted` | — |

**Köken:** Tespitler motor (noktalı çerçeve); o aralıktaki insan çizimleri o günkü sürümüyle (Y16 "çizim değişti" kuralı).

**Veri durumu:** Gezinti canlı sicilden okur; yeniden oynatma ayrı kip ve "simüle" (iki kip aynı ekranda birlikte açılmaz).

**Akış:** Zaman ekseninde sürükle ya da sicil kaydına dokun → aralık parantezi + şerit → işarete
gel → sonuç cümlesi → "Bugüne dön".

**Masaüstü / telefon:** Telefonda aralık iki dokunuşla, şerit üstte tek satır; özet alt sayfada.

**Uyarlanan:** OV-12, VV-C2, VV-C4. **Roller:** `--qp-origin-engine`, `--qp-valid`, `--qp-invalid`, `--note-bg`, `--note-fg`, `--card`, `--muted`.

### Y35 — Hazırlık dosyası, ortak analiz ve üyelik

**Ne:** Olay öncesi dosya (hazırlık görevi, K76: bilanço, temettü, aralık açıklaması); herkes
için aynı olan analiz bir kez üretilir, kullanıcılar üye olur, üye sayısı görünür; "Sinyal,
tavsiye değil" ibaresi. · **Kademe:** 2 · **Kararlar:** K76, K77; A61.

**Yerleşim:** İzleme görevleri listesinde ikinci görev türü ("Hazırlık" etiketi); dosya sağ
panelde (masaüstü) ya da tam sayfada (telefon). Dosya bölümleri: başlık (olay, tarih, "3 gün
kaldı" durum metni) · geçmiş tepkiler (motor) · taban oran ve n (Y33) · ilgili seviyeler (grafikte
noktalı) · ajan yorumu (kesik) · üyelik satırı · ibare.

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Hazırlanıyor | Bölüm iskeletleri | `--line` | "Dosya hazırlanıyor · bilanço 28 Kas" |
| Hazır | Tam dosya | `--fg` | "SYN20 bilanço hazırlığı · 28 Kas" |
| Üye değil | Üyelik satırında sayı + "Üye ol" | `--muted`, `--accent` | "Ortak analiz · Sistem · 1.240 üye · Üye ol" |
| Üye | Satırda durum; dosya güncellenince haber | `--fg` | "Üyesin · güncellenince haber verilir" |
| Güncellendi | Değişen bölüm vurgulu, değişiklik satırı | `--accent` (ince kenar) | "2 saat önce güncellendi: hacim beklentisi" |
| Olay geçti | Dosya kapanır, sonuç sicile | `--qp-closed` | "Bilanço açıklandı · tepki sicile yazıldı" |
| Yetki yok | Bölümler kilitli, varlık söylenir | `--muted` | "Dosya var · görmek için BIST seviye 1" |
| Sayı doğrulanamadı | Ortak analizde sayı yalnız motor cümlesinden (A61); değilse bölüm yayımlanmaz | `--qp-warn` | "Bu bölüm yayımlanmadı" |

**Köken:** Ortak analiz önce yalnız sistem yayımlar: "Sistem" + motor noktalı kenar; ajan yorumu
kesik; ileride kullanıcı yayınları canlı siciliyle ve kendi adıyla (K76). Pozisyon paylaşılmaz.

**Veri durumu:** Başlıkta `D-*`; geçmiş tepki satırları düzeltilmiş seriden (K88); ibare "Sinyal, tavsiye değil" her durumda altta.

**Akış:** Takvimde olay yaklaşır → listede "Hazırlık" satırı → dosya → "Üye ol" → üye sayısı artar →
olay günü bildirim → olay geçer → sonuç sicile, dosya kapanır.

**Masaüstü / telefon:** Telefonda bölümler katlı başlıklar; üyelik satırı yapışık altta.

**Uyarlanan:** VV-C1, VV-C4, AT-H2. **Roller:** `--qp-origin-engine`, `--qp-origin-agent`, `--fg`, `--muted`, `--accent`, `--qp-closed`, `--qp-warn`.
