# Mimari

Durum: taslak v0. R0'daki denemelerin sonucuna göre güncellenir.

## Sınır

Qapsule bir **kütüphane ve sözleşme** projesidir. Ajanların çalıştığı ortam, abonelik,
persona ve dağıtım ayrı sistemlerdir ve Qapsule'ı tüketir.

```
            Matriks yayınları / NATS / mevcut indicator-engine, anomaly-engine
                                   │  (seri ve olgu akışları)
                                   ▼
┌──────────────────────────── QAPSULE ─────────────────────────────┐
│  Seri kaynağı arayüzü   ── kaynağı soyutlar                      │
│  Hesap çekirdeği        ── deterministik, akışkan, istek anında  │
│  Olgu dedektörleri      ── seriden tipli olgu                    │
│  Doküman + komut yolu   ── tek şema, tek yazma yolu              │
│  Görsel gramer          ── görünüm tarifi                        │
│  Çizim çekirdeği        ── headless + etkileşimli                │
│  Ajan yüzeyi            ── CLI, MCP, anlamsal okuma              │
└───────────────────────────────────────────────────────────────────┘
        ▲                           ▲                        ▲
   Yorumlayan ajanlar        Kabuklar (web, iOS,       Gömme (asistanlar,
   (ayrı sistem)             Android, sunucu)          WhatsApp, widget)
```

## Kalıcı çekirdek

| Parça | Sorumluluk |
|-------|------------|
| Şema | Seri, Olgu, Bulgu, Kapsül, Görünüm, Doküman, Komut tipleri. Rust tipi tek kaynaktır; JSON Schema, TS tipleri ve ajan araç tanımları ondan üretilir. |
| Hesap çekirdeği | `f64`, akışkan (her yeni bar O(1)), çok girdili/çok çıktılı, parametre taraması için toplu çalıştırma. LLM yok. |
| Görsel gramer | İşaret (mark), ölçek, katman, kodlama. Mum grafiği ve anlam işaretleri bu gramerle ifade edilir. |
| Komut yolu | Dokümanı değiştiren tek yol. Geri alma, yeniden oynatma ve köken kaydı buradan gelir. |

## Değiştirilebilir kenarlar

| Kenar | Bugün | Yarın olabilir |
|-------|-------|----------------|
| Veri | Dosya, Matriks REST/MQTT | NATS, başka sağlayıcılar |
| Yorumlayan model | Bulut LLM | Cihaz üstü küçük model |
| Ekran | Web, sunucu PNG | Mobil, saat, gözlük, ses |
| Gömme | Kendi görüntüleyicimiz | MCP Apps, A2UI, widget SDK |
| Hesap donanımı | CPU (SIMD) | GPU hesap |
| Uzantı | Derlenmiş hesaplar | Sandbox'ta ajan yazımı hesap/görünüm |
| Tahmin | Yok | Takılabilir tahmin sağlayıcı |

## Hesap nerede yapılır

- **Ortak ve yaygın olan** sunucuda bir kez hesaplanır ve akış olarak gelir.
- **Geri kalan her şey** çekirdekte, istek anında hesaplanır (cihazda ya da sunucuda;
  aynı kod).
- Bir değer hem akıştan hem yerel hesaptan gelebiliyorsa, tanım tektir ve sonuçlar
  test ile karşılaştırılır.

## Kod düzeni (hedef)

```
qapsule/
  crates/
    qp-schema/     tipler + JSON Schema üretimi
    qp-compute/    hesap çekirdeği
    qp-facts/      olgu dedektörleri
    qp-grammar/    görsel gramer
    qp-render/     çizim (headless + GPU)
    qp-doc/        doküman + komut yolu
    qp-cli/        ajanların ve testlerin kullandığı komut satırı
    qp-wasm/       web bağlaması
  shells/          web, ios, android (ince kabuklar)
  fixtures/        örnek veri, örnek doküman, altın görseller
  docs/  rounds/
```

## Doğrulama omurgası

Ajanların yaptığı işin doğruluğu üç mekanizmayla ölçülür:

1. **Altın görsel testi:** sabit girdi → sabit görsel. Referans çizici deterministiktir.
2. **Özellik testleri:** hesap ve şema için rastgele girdiyle değişmezler.
3. **Uçtan uca CLI senaryoları:** her round'un çıktısı `qp` komutuyla ekransız sürülebilir.

## Açık teknik kararlar

`DECISIONS.md` içindeki "Açık" bölümüne bakın. R0 denemesi çizim yığınını belirler.
