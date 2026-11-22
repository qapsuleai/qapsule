# Uygulama yönergesi — insan testi v4 (P15)

Amaç: `tavek`'in (RD2'de seçilen yön) "profesyonel ve güvenilir" görünüp görünmediğini insanla
ölçmek; R5'in `oltan`'ı (önce) ve bir başvuru ekranı (OpenChart, OpenTerminalUI) yanında.
Ölçüt `../refs/reports/VISUAL.md` §5 P14 (gösterim koşulu) ve P15 (insan yargısı).
**Ajan eleştirmeninin puanı seçim gerekçesi değildir**; bu testle ayrışırsa bu test geçerlidir.

## Özet

| | |
|---|---|
| Kim | **Arkin** ve en az **iki işlem yapan kullanıcı** (düzenli alım-satım yapan; proje ekibinden değil, galeriyi görmemiş) |
| Süre | Kişi başına **~10 dk**: giriş 1, masaüstü bloğu 3, telefon bloğu 3, açık sorular 3 |
| Kod | Arkin **P0**; öbürleri **P1, P2 …** geliş sırasıyla. Ad, yaş, iletişim yazılmaz |
| Malzeme | Masaüstü ekran (%100 yakınlaştırma), telefon (yürütücünün; ~390 pt genişlik, DPR 3), `FORM.md` (katılımcı başına), kronometre |
| Görseller | `SCREENS.md`: sentetik veri, nötr kodlu klasör; gerçek veri görseli commit edilmez |

## Kurulum

1. Görselleri `SCREENS.md`'ye göre üretin ve nötr kodlu klasöre (`~/qp-ht-v4/`) kopyalayın.
2. **Masaüstü:** tarayıcıda `~/qp-ht-v4/blind.html?o=<sıra>` tam ekran, yakınlaştırma %100.
   Aday solda, başvuru sağda, ikisi aynı genişlikte (P14).
3. **Telefon:** telefon kodlu görseller (`K5`–`K8`, `B3`, `B4`) telefonda Dosyalar uygulamasında;
   parlaklık normal, gece modu kapalı, yakınlaştırma yok. Başvuru masaüstünde, telefonun yanında
   (`blind.html?o=<sıra>&blok=tel`).
4. `compare.html` ve galeri **katılımcıya gösterilmez**: görünüş adlarını yazar.

## Ekranlar (yalnız yürütücü bilir)

| Kod | Çapa | Görünüş | Gösterim |
|---|---|---|---|
| K3 / K1 | Ç1 masaüstü çalışma alanı, koyu | `tavek` / `oltan` | masaüstü, yanında B1 |
| K2 / K4 | Ç2 grafik hücresi, koyu | `tavek` / `oltan` | masaüstü, yanında B2 |
| K6 / K7 | Ç3 telefon kontrol kulesi, koyu | `tavek-tel` / `oltan` | telefon, yanında B3 |
| K8 / K5 | Ç4 telefon grafik ve açılmış kapsül, koyu | `tavek-tel` / `oltan` | telefon, yanında B4 |
| B1–B4 | Ç1–Ç4 başvurusu (OpenChart Ç1/Ç2; OpenTerminalUI telefon) | — | tek başına |

B3/B4 kısmi başvurudur (küçültülmüş masaüstü, DPR 1): biçim için değil yoğunluk için.
Açık tema isteğe bağlı: aynı kodlar `a` ekiyle (`K3a` …), koyu bloklardan sonra, aynı sırayla.

## Sıralar (katılımcıya göre sabit; P0 → 1, P1 → 2, P2 → 3, P3 → 1 …)

| Sıra | Masaüstü bloğu | Telefon bloğu |
|---|---|---|
| 1 | K3, K4, B1, K1, K2, B2 | K7, K8, B3, K6, K5, B4 |
| 2 | K1, B2, K2, K3, B1, K4 | B4, K6, K5, B3, K7, K8 |
| 3 | B1, K2, K3, B2, K1, K4 | K5, B3, K6, K8, B4, K7 |

Her çapada `tavek` ile `oltan` üç sıranın en az birinde önce, birinde sonra gelir.

## Protokol (her ekran için)

1. "Hazır mısınız?" → ekran açılır, **5 saniye** bakış (kronometre; `blind.html` kendisi kapatır).
   Aday ekranda başvuru yanda durur; B ekranında başvuru tek başınadır.
2. Ekran kapanır. İki soru, aynen:
   - **"Bu profesyonel bir araç mı?"** — evet / hayır
   - **"Buna güvenir miydim?"** — 1 (hiç) … 5 (tamamen)
3. Cevap süresi (saniye) ve kendiliğinden söylenenler **aynen** forma.

İki blok bitince üç açık soru (aynen; cevap aynen yazılır):

1. "Gördüklerinizden profesyonel durmayan ne vardı?"
2. "Neyi görseydiniz daha çok güvenirdiniz?"
3. "Yanındaki ekrana göre eksik olan ne?"

Katılımcı bir ekranı işaret ederse yürütücü kodunu not eder (katılımcıya kod söylenmez).

## Kurallar

- **Hangisinin "bizim", "yeni" ya da "seçilen" olduğu söylenmez**; görünüş adları, yapay zekâ,
  R5 sözü edilmez. Başvuru da adıyla tanıtılmaz ("yanındaki ekran").
- Onay, baş sallama, yorum yok. Piyasa yorumu yok. Önceki katılımcıların cevapları söylenmez.
- Sözler **aynen** yazılır; kişiyi tanıtan her şey (ad, iş yeri, kendi yatırımları) çıkarılır.
- **P0 (Arkin) kör değildir**: görünüşleri biliyor. Cevabı ayrı raporlanır, öbürleriyle toplanmaz.
- Görseller yalnız sentetik veriyle. `--real` ile üretilen görsel yalnız yürütücünün kendi
  provası içindir; depo dışında kalır, katılımcı testine girmez, commit edilmez.
- Ajan eleştirmeni puanı, `design-check` sonucu seçim gerekçesi olarak kullanılmaz.

## Kayıt

1. Oturum biter bitmez `FORM.md` satırları denetlenir (her ekranın satırı var mı).
2. Form `form.csv` biçimine geçirilir; özet tablo (`FORM.md` "Özet") hesaplanır.
3. `RESULTS.md`: önce Arkin'in cevabı (round onunla kapanır), öbür kullanıcılar sonradan eklenir.
   `form.csv` kod-anonimdir; Arkin'in onayıyla commit edilir, yoksa depo dışında kalır.
4. Son kabul (K2) `DECISION.md`'ye Arkin'in kendi sözleriyle yazılır; bu paket yazmaz.
