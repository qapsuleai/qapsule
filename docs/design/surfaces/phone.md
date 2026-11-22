# Yüzeyler — telefon (Y26, Y27)

Ortak kurallar: [`../SURFACES.md`](../SURFACES.md). Profil `tavek-tel` (dokunma hedefi
`--qp-control-h`, `--qp-tab-h`); alt sekme çubuğu **Kule · Grafik · İzleme · Ajan** (AT-E1);
karar ve ayrıntı alt sayfada. Başvuru ekranları bu iki yüzey için kısmidir (ADOPTED "Galeri").

### Y26 — Mobil kontrol kulesi ana ekranı

**Ne:** Kapsül akışı, karar bekleyenler, izleme özeti: telefonun ana yüzeyi; kullanıcı burada
ajanlarını izler ve yönetir (NORTH_STAR "kontrol kulesi"). · **Kademe:** 1 · **Kararlar:** K72, K75, K76.

**Yerleşim (Ç3, yukarıdan aşağı):** (1) başlık "Kontrol kulesi" + tek satır veri durumu (Y25);
(2) üç sayı kutucuğu: "5 Etkin izleme · 1 Gecikmeli veri · 2 Bugün tetiklenen" (sözcüklü,
dokununca süzer); (3) **karar bekleyen kartları** (öneri kendi küçük grafiğinde: kesik mor seviye +
eksen etiketi; soru kartı seçenekleriyle); (4) "Kapsüller · Bugün" akışı: satır sol kenarı köken
çizgisi, durum eksen etiketi biçiminde ("112,40 │ Kırıldı 2/3"), altında sicil satırı (Y33).

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Boş (ilk gün) | Y30'un kısa hali: tek cümle + "Bir sembol ekle" + basamak seçimi | `--muted`, `--accent` | "Henüz izlediğin bir şey yok." |
| Sakin | Kutucuklar + "Bugün bir şey olmadı" satırı + sağlık | `--muted` | "Bugün bir şey olmadı · 5 izleme sağlıklı, son kontrol 14:35" |
| Karar bekleyen var | Kartlar akışın üstünde; sekme rozetinde sayı | `--qp-origin-agent` | "Ajan önerisi · 2 dk önce · SYN20 için 117,50'ye destek ekle" |
| Soru bekleyen var | Soru kartı, ≤ 3 tam cümle seçenek | `--qp-origin-agent` | "Soru · Kırılımı neye göre sayayım?" |
| Tetiklenen var | Akışta satır üstte, kutucukta sayı | `İ-*` rolü | "SYN20 14:35 · 112,40 direncini kırdı, tutunma 2/3 bar" |
| Çevrimdışı | `D-koptu` tam genişlik şerit; kararlar devre dışı + neden | `--alert-bg`/`--alert-fg` | "Çevrimdışı · son güncelleme 14:35 · kararlar bağlanınca" |
| Veri bayat | Kutucuklar ve satırlar `D-bayat` | `--qp-warn` | "Veri bayat · son veri 14:35" |
| Sağlık sorunu | Başlık altında tek şerit (Y31) | `--alert-bg`/`--alert-fg` | "İzlemeler çalışmıyor · 14:35'ten beri" |
| Simüle | Başlıkta "Sentetik veri" / "simüle" bandı | `--note-bg`/`--note-fg` | "Sentetik veri · oynatma" |

**Köken:** Akış satırlarının sol kenarı grafikteki çizgi dilini tekrarlar (insan düz, ajan kesik,
motor noktalı); karar kartı kesik kenarlı (ajan), kararın kendisi insan.

**Veri durumu:** Tek satır durum üstte; kutucuk "Gecikmeli veri" sayısı gecikmeli bağı sayar;
lisanslı rakam kilit ekranında yok (S16), uygulamada var.

**Akış:** Bildirim → kule açılır, ilgili kart üstte vurgulu → karar (tek dokunuş) → kart durum
satırına iner ("Eklendi · 14:35") → akış satırına dokun → Grafik sekmesi kanıtla açılır (Y27,
anlam zoom'u: cümle → kapsül → işaretli grafik).

**Masaüstü / telefon:** Yalnız telefon; masaüstündeki karşılığı Y07 + Y13 + ajan paneli.

**Uyarlanan:** AT-E1, AT-C6, VV-C1, OV-24. **Roller:** `--qp-origin-*`, `--qp-agent-dash`,
`İ-*` rolleri, `--card`, `--muted`, `--alert-bg`, `--alert-fg`, `--note-bg`, `--note-fg`,
`--qp-control-h`, `--qp-tab-h`.

### Y27 — Telefonda grafik

**Ne:** Görüntüleme, kanıt ve hızlı tek seviye çizimi; açılmış kapsül alt sayfada (Ç4).
· **Kademe:** 1 · **Kararlar:** K72, K73, K75.

**Yerleşim (Ç4):** üstte geri ("‹ Kule") + "SYN20, günlük" + periyot şeridi; grafik tam genişlik
(oluk `tavek-tel`); lejant yalnız C + değişim; grafik altında hapsız "Kapsül · İzleme" sekmeleri;
alt sayfa yarım yükseklik: iddia, kanıt satırları, sicil (Y33), "Sinyal, tavsiye değil", eylemler
"İzle" · "Grafikte göster".

**Durumlar**

| Durum | Gösterilen | Rol | Metin kalıbı |
|---|---|---|---|
| Y01 durumları | Y01 tablosu, telefon metinleriyle | Y01 | — |
| Kapsül açık | Kanıt bandı + zaman ekseninde parantez; seviye eksen etiketi + durum; alt sayfa | `--qp-origin-engine` | "Kanıt penceresi grafikte işaretli · 14–22 Ağu" |
| Yatay kip | Alt sayfa kalkar, imleç ve OHLC lejantı açılır | — | — |
| Çizim kapalı | "Seviye" düğmesi yok (salt okuma, yetki ya da ajan bağlamı) | `--muted` | "Bu grafikte çizim kapalı" |
| Hızlı seviye konuyor | Dokunulan yerde yatay çizgi + fiyat hapı; "Koy" / "Vazgeç"; sonra niyet sorusu (Y04 telefon) | `--qp-origin-human` | "112,40'a seviye · koy" |
| Dokunup tutma | İmleç + eksen hapları + lejant o bara | `--qp-axis` | — |
| Öneri bekliyor | Kesik seviye + kesik eksen etiketi; çipte iki eylem | `--qp-origin-agent` | "Reddet" · "Grafiğe ekle" |

**Köken:** Y01 ile aynı; yer darlığında çip eksen etiketinin içine girer ("112,40 │ ◐ 2/3"), köken
çerçeve biçiminde kalır (dolu / kesik / noktalı).

**Veri durumu:** Lejant satırının sonunda `D-*` sözcüğü; simülede başlıkta bant.

**Akış:** Kuleden kapsül → grafik kanıt penceresine açılır + alt sayfa → alt sayfayı yukarı çek →
tam kapsül; aşağı çek → yalnız grafik; "Seviye" → dokun → "Koy" → niyet sorusu.

**Masaüstü / telefon:** Yalnız telefon. Bilinen kusurlar (DIRECTION): RSI lejantı çizgiye binebilir;
son değer etiketi bir fiyat etiketine değebilir; gramer işi (P6) gerektirir.

**Uyarlanan:** OV-24, AT-E1, OV-5…OV-10. **Roller:** Y01 rolleri, `--qp-origin-*`,
`--qp-control-h`, `--card`, `--qp-radius-card`.
