# Dil Modelleri ve Anahtarlar

Bu dosya, projenin hangi aşamada hangi dil modeline ve hangi erişim anahtarına ihtiyaç
duyduğunu kaydeder. Bir round model anahtarı gerektiriyorsa bu, round'un `PROMPT.md`
dosyasının başında "Başlatmadan önce" altında yazılır ve burada güncellenir.

## İlke

- **Motorun içinde dil modeli yoktur** (K3, K21). Hesap, olgu, kapsül, cümle, işaret ve
  çizim deterministiktir; anahtarsız ve ağsız çalışır.
- Dil modeli yalnızca kenarlardadır ve değiştirilebilir (ilke 10): sağlayıcıya, modele
  ya da modelin nerede çalıştığına (bulut, cihaz) bağımlılık kurulmaz.
- Hangi iş için hangi modelin kullanılacağı varsayımla değil ölçümle seçilir (K39).

## Bugünkü durum: anahtar gerekmiyor

| Nerede | Model | Anahtar |
|--------|-------|---------|
| Geliştirme (round'lar) | Claude Code'un kendi oturumu ve alt ajanları | Yok; Claude Code aboneliği yeterli |
| Ölçümlerde okuyucu/kullanıcı rolü (ajan yazabilirlik, işaret okuma, cümle anlama) | Claude Code alt ajanları ya da `claude -p` | Yok |
| Motor, protokol, çizim | — | Yok |

Sınır: alt ajanla alınan gecikme ve maliyet ölçümleri üst sınırdır (RD1/T6e). Kesin
gecikme ve maliyet için doğrudan API ölçümü gerekir; o zaman anahtar gerekir.

## Anahtar gerektirecek aşamalar (öngörü)

| Aşama | İş | Model sınıfı | Not |
|-------|----|--------------|-----|
| R5 (ajan yüzeyi) | Doğal dil → protokol mesajı; gerçek gecikme ve maliyet ölçümü | Hızlı ve ucuz bir model + karşılaştırma için güçlü bir model | En az iki sağlayıcıyla ölçülür; MCP üzerinden bağlanır |
| R9 (niyet, anlatım) | Kullanıcının serbest metin niyetini koşula çevirme; kapsülü kişiye göre anlatma; ses | Hızlı model; ses için ayrıca konuşma modeli | Motorun doğruladığı yuvalara yazar; sayı üretmez |
| Yorumlayan ajanlar (Qapsule dışı, A5) | Olguları haber ve temel veriyle birleştirip bulgu önermek | Güçlü model | Ayrı sistem; Qapsule'ı MCP ile kullanır. Mevcut Matriks AI altyapısıyla ilişkisi A5'te |
| R4/R8 (cihaz) | Cihaz üstü küçük model denemesi | Küçük açık model | Anahtar gerekmez; cihazda çalışır |

Sağlayıcı ve model adları burada bilerek yazılmadı: seçim, ilgili round'da güncel
seçenekler ölçülerek yapılır ve bu tabloya o zaman işlenir.

## Anahtarların saklanması

- Anahtarlar depoya girmez. Depo kökünde `.env.local` dosyasında tutulur; bu dosya
  `.gitignore` ile dışlanmıştır.
- Değişken adları: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` (ve ileride eklenecek başka
  sağlayıcılar için aynı düzende).
- Ajan `.env.local` dosyasını açmaz, içeriğini yazdırmaz, günlüğe ya da komut satırına
  koymaz. Betikler değişkeni ortamdan okur.
- Anahtarı Arkin girer; ajan yalnızca hangi değişkenin gerektiğini söyler.
- Ölçümlerde harcama sınırı round tasarımında yazılır; sınır aşılacaksa ajan durur ve sorar.

## Matriks MCP (R5B, K67)

Ajanlar bağlam için Matriks'in MCP sunucusunu kullanır (temel veriler, haberler, kurum
raporları). Kimlik `.env.local`'da, Arkin girer:

| Değişken | Ne |
|---|---|
| `MATRIKS_MCP_URL` | MCP sunucu adresi |
| `MATRIKS_MCP_CLIENT_ID` | `X-Client-ID` başlığı |
| `MATRIKS_MCP_API_KEY` | `X-API-Key` başlığı |

MCP yapılandırma dosyası gizli değer taşımaz; ortam değişkenine başvurur. Sorgular
Arkin'in lisansıyla koşar: ölçülü kullanılır, sonuçlar depoya yazılmaz.

