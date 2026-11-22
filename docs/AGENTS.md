# Ajanlar: Qapsule'e MCP ile bağlanmak

Qapsule'ün ajan yüzeyi (R5B) bir **MCP sunucusudur**: `qp mcp serve`. Bir ajan (Claude Code,
Claude masaüstü ya da MCP konuşan başka bir ortam) canlı bir oturuma bağlanır; dokümanı
**okur**, **izler** (abonelik ve niyet), **yorum** yazar (motor doğrular), **öneri** yapar
(insan onaylar). Protokol: `docs/PROTOCOL.md` §25. Kavramlar: `docs/CONCEPTS.md`.

**Motor söyler, ajan anlatır.** Olgu, bulgu, kapsül, karne ve hikâye durumu yalnız
deterministik motordan gelir; ajan bunları üretemez. Ajan emir göndermez, işlem yapmaz.

## Neler var, neler yok

| Ajan yapabilir | Ajan yapamaz |
|---|---|
| Her şeyi kendi adına çalıştığı kişinin yetkisi kadar okumak (lisanslı sayı yetkisiz okura gitmez) | Olgu, bulgu, kapsül, karne yazmak |
| `add_comment`: kanıta bağlı yorum (doğrulayıcı: atıf, sayı, tavsiye dili, lisans, yön) | Doğrulanmamış yorumu yayımlamak (ret nedeni makinece okunur döner) |
| `add_watch` / `remove_watch`: niyet ("SYN03'te aşağı kırılım olursa haber ver"); eşleşme motorda | Kişi başına 32'den fazla niyet |
| Görünüm ve çizim **önerisi** (bekleyen öneri olur) | **Öneriyi onaylamak ya da reddetmek** — yalnız insan kökenli komut; ajanın MCP ucunda bu araçlar yok, genel `command` aracıyla gelirse `forbidden` / `human_only` (test: `crates/qp-mcp/tests/human_decision.rs`) |
| `preview`: bir önerinin ekransız görüntüsü | Replay'i ilerletmek (yalnız `host` / `owner`) |

## Güvenlik varsayılanları

- **Yalnız bu makine:** HTTP varsayılan `127.0.0.1`; `Origin` ve `Host` denetlenir. Başka
  bir adresi dinlemek (`--listen`) **kimlik ister**: `--token-env VAR` (Bearer belirteci
  ortamdan okunur, hiçbir yere yazılmaz).
- **Sunucu başına bir ajan kimliği** (`--principal`, varsayılan `claude` = Arkın adına).
  Oturum kimliğe bağlıdır; başka bir başlıkla kimlik değiştirilemez. İnsan kimliğiyle MCP
  açan eski `qp mcp --init --principal` yolu yoktur (RD2, A62).
- **Dış içerik veridir:** haber, rapor ve araç çıktısındaki "talimat" ajana komut değildir
  (başvuru ajanlarının talimatı; ölçüm: `rounds/R5B/REPORT.md`).
- MCP yapılandırma dosyaları **gizli değer taşımaz**; ortam değişkenine başvurur.

## 1. Claude Code

Depo kökünde `.mcp.json` hazır (proje kapsamı). İki sunucu tanımlı:

- `qapsule`: `scripts/qp-mcp.sh --stdio` (ilk çalıştırmada `qp`'yi derler). Kimlik
  `QP_MCP_PRINCIPAL` (varsayılan `claude`), yüzey `QP_MCP_SURFACE` (varsayılan `thin`).
- `matriks`: Matriks MCP (`MATRIKS_MCP_URL`, başlıklar `X-Client-ID`, `X-API-Key`; değerler
  ortamdan). İstemiyorsan onaylama ekranında reddet.

```sh
cd qapsule
set -a; . ./.env.local; set +a      # yalnız Matriks için; değer ekrana basılmaz
claude                              # ilk açılışta proje MCP sunucularını onayla
```

`/mcp` komutu bağlı sunucuları gösterir. Elle eklemek istersen (proje dışından):

```sh
claude mcp add qapsule -- /TAM/YOL/qapsule/scripts/qp-mcp.sh --stdio
claude mcp add --transport http qapsule-demo http://127.0.0.1:8390/mcp   # qp demo --mcp
```

## 2. Claude masaüstü

`~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) içine:

```json
{
  "mcpServers": {
    "qapsule": {
      "command": "/TAM/YOL/qapsule/scripts/qp-mcp.sh",
      "args": ["--stdio"],
      "env": { "QP_MCP_PRINCIPAL": "claude" }
    }
  }
}
```

Uygulamayı yeniden başlat; araç simgesinde `qapsule` görünür. `qp` önceden derlenmiş olsun
(`cargo build -p qp-cli --release`), çünkü masaüstü ilk derlemeyi beklemeyebilir.

## 3. Genel MCP istemcisi (QODI dahil)

Streamable HTTP:

```sh
target/release/qp mcp serve --http --port 8765            # http://127.0.0.1:8765/mcp
```

MCP `2026-07-28` durumsuzdur: her istek sürümü `_meta` içinde taşır, sunucu
`server/discover`'ı cevaplar. Eski sürüm istemcileri `initialize` gönderir ve
`Mcp-Session-Id` başlığını sonraki isteklerde taşır; ikisi de desteklenir. Bildirimler:
`subscriptions/listen` + `notifications/resources/updated` (yeni; `qapsule://notifications`
kaynağını okuyunca bekleyen `notify` iletileri gelir) ya da eski istemcilerde
`notifications/message`. Başka bir makineden erişim gerekiyorsa:

```sh
export QP_TOKEN=…   # bir belirteç üret; depoya yazma
target/release/qp mcp serve --http --listen 0.0.0.0 --port 8765 --token-env QP_TOKEN
# istemci: Authorization: Bearer $QP_TOKEN
```

stdio konuşan istemciler `scripts/qp-mcp.sh --stdio` komutunu çalıştırır.

## Sunucu seçenekleri (özet)

| Seçenek | Ne |
|---|---|
| `--source synthetic` (varsayılan) / `local` / `--init FILE` | Oturum: sentetik yeniden oynatma, `fixtures/local/` gerçek barları ya da hazır bir init. Canlı (`sim`/`live`) R6 `livehost` ile |
| `--seek YYYY-MM-DD`, `--running`, `--speed` | Replay'in konumu ve hızı (ajan ilerletemez) |
| `--surface thin` | Araç yüzeyi. **M28 seçimi `thin`** (protokolün aynası); `task`, `query`, `mixed` ölçüm adayı olarak durur |
| `--capsule-format card\|layered\|json` | Kapsül cevabının biçimi. **M29 kuralı `json`** (varsayılan, verilmezse); kart ve katmanlı aynı sayıları taşır |
| `--log`, `--dump`, `--run` | Yalnız ölçüm içindir; ürün için kullanılmaz |

**Araç tanımı boyu:** `thin` her protokol işlemini ayrı araç olarak verir (~520 KB tanım).
Claude Code tanımları ihtiyaç oldukça yükler; tanımların hepsini bağlama koyan bir
istemcide bu büyüktür — orada `--surface mixed` (≈ 95 KB) ya da `query` (≈ 16 KB) daha
hafiftir (M28'de başarıları istatistikçe ayırt edilemedi; `rounds/R5B/experiments/M28/`).

## İnsan ve ajan aynı dokümanda: `qp demo --mcp`

```sh
scripts/demo.sh --mcp            # web kabuğu http://127.0.0.1:8390 + ajan ucu /mcp
claude mcp add --transport http qapsule-demo http://127.0.0.1:8390/mcp
```

Ajanın yorumu açılan kapsülde **"ajan yorumu"** etiketi ve kökeniyle görünür; ajanın
önerileri web kabuğunda bekleyen öneriler listesine düşer, **onay ve ret yalnız orada**
(insanın kendi bağlantısında) yapılır.

`--mcp` ile web kabuğunun adresi terminalde bir **açılış anahtarıyla** basılır
(`http://127.0.0.1:8390/?key=…`): kişinin bağlantısı yalnız o anahtarla açılır. Böylece
aynı makinede çalışan bir ajan (örneğin kabuk erişimi olan bir Claude Code) kendi önerisini
kişinin bağlantısından onaylayamaz — anahtarı ajanın göreceği bir yere yapıştırma (demoyu ajanın kendisi başlatırsa anahtarı çıktısında görür: demoyu kendin başlat).
`--mcp`, `--lan`/`--https-host`/başka `--bind` ile reddedilir (gösterimin ajan ucunda
belirteç yok; ağdan ajan için `qp mcp serve --http --listen … --token-env …`).

**Bilinen sınırlar (denetim, `rounds/R5B/AUDIT.md`):** doğrulayıcının sayı ve tavsiye
taramaları düzenli ifadeyle çalışır; kendi kurduğu bozuk yorumlarla onları atlatmaya
çalışan bir ajana karşı tam koruma değildir (üst simge, Roma rakamı, anlamı farklı bir
kanıt değerini yeniden kullanmak; tavsiye eş anlamlıları). Lisans tarafı yapısaldır: kısıtlı okur, görünümü redakte bir enstrümana dokunan başkasının
yorumunu hiç almaz; ayrıca kısıtlı
okura giden yorumdaki her sayı (tarih ve saat biçimi dahil, maskesiz okunarak) okurun
gördüğü bir değer olmalıdır; sözcükle yazılmış ondalık hiç geçmez.

## Başvuru ajanları

`agents/` klasörü (ürün değil, örnek ve ölçüm düzeneği): **Yorumcu** (kapsül + kanıt + dış
bağlam → doğrulanmış yorum), **Nöbetçi** (niyetleri kurar, eşleşmeleri izler, yalnız anlamlı
olanı bildirir), **Soru cevaplayan** (kaynak göstererek cevap). Çalıştırma: `agents/README.md`.
Matriks bağlamı `agents/tools/mxmcp` ile (önbellekli, sayaçlı; sonuçlar yalnız
`fixtures/local/` altında).

## Arkın için beş dakikalık deneme

1. Depo kökünde: `scripts/demo.sh --mcp`; terminalde basılan `qp demo: http://127.0.0.1:8390/?key=…` adresini tarayıcıda aç (replay oynar).
2. Başka bir terminalde: `claude mcp add --transport http qapsule-demo http://127.0.0.1:8390/mcp`
   (bir kez), sonra `claude`.
3. Üç soru sor:
   - "Qapsule'de şu an en önemli açık hikâyeler hangileri? Kimlikleriyle, birer cümleyle."
   - "İlkini aç: neden doğdu, kanıtı ne, karnesi ne diyor? Sayıları kanıttan al."
   - "SYN03'te aşağı yönlü bir kırılım doğarsa bana haber ver." (bir niyet kurar; eşleşince
     bildirim gelir)
4. İstersen: "Bu kapsül için iki cümlelik bir yorum yaz" (motor doğrular; web kabuğunda
   kapsülü açınca "ajan yorumu" olarak görürsün) ve "Ana görünümü son 50 bara daralt"
   (öneri olur; web kabuğunda sen onaylarsın).
