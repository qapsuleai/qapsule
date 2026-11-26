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
| Program denetlemek, denemek, **öneri** olarak eklemek (RL1, aşağıda "Programlar") | Programı doğrudan eklemek ya da silmek |

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

## Programlar (RL1)

Ajan hesap dilinde (`qp-ir/0`, `docs/PROGRAMS.md`; metin dil §12) bir **program** yazabilir:
"20 barın en yükseğini kapanışla geçince olay" gibi bir kuralı, kendi ürettiği değil motorun
hesapladığı seriler olarak. Oturum tarafı `docs/PROGRAMS-SESSION.md`. Akış her zaman
**denetle → dene → öneri olarak ekle**'dir:

| Adım | `thin` aracı | Ne döner |
|---|---|---|
| 1. Denetle | `program_check {program, inputs?, params?, lang?}` | `program_check`: `ok`, **bütün hatalar birden** (`reason` alt kodu, `at` ara gösterim işaretçisi, metinde `span` satır:sütun, `did_you_mean`, `hint`), uyarılar, maliyet, çıktılar; metin verildiyse `ir`. Program hatası protokol hatası değildir, cevabın içindedir |
| 2. Dene | `program_try {program \| instance, inputs, params?, range?}` | `program_try` (yeniden oynatma kipi): çıktı başına sayılar ve ilk/son öğeler, sinyal kayıtları `mode: "replay"`, `label: "simulated"`; `work {used, limit}`, bütçe dolarsa `stopped`. Hiçbir şey yazmaz |
| 3. Ekle | `program_add {program, inputs, id?, params?, show?, reason?}` | Metin burada ara gösterime iner, tek bir `add_program` komutu gider. MCP bağlantısı ajan kökenlidir: **öneri olur** (`ack.status: "proposed"`); `mode: "direct"` → `forbidden` / `proposal_only` |
| Açıkla | `program_explain {program \| instance, at?}` | Kanonik metin, `program_id`, düğüm başına maliyet; `at {output, t}` ile bir değerin kökeni |

- `program` metin (dizgi) ya da ara gösterim (nesne) olabilir. Üretilen `query_check_program`,
  `query_try_program`, `query_explain_program`, `op_add_program` araçları da durur (metni
  `source` alanında alırlar; `op_add_program` yalnız ara gösterim alır).
- **Karar yok.** Öneriyi yalnız insan kabul eder ya da reddeder (§25.2); MCP ucunda karar
  aracı yoktur. Programı silmek (`remove_program`) ajana hiç açık değildir
  (`agent_cannot_write`); parametre değişikliği (`set_program_params`) yine yalnız öneridir.
- **Ret biçimi.** İstenen kural dilde yazılamıyorsa (yarının kapanışı gibi geleceğe bakan bir
  şey) program uydurma; kodlu ret yaz: metinde `reject E_LOOKAHEAD "neden"`, ara gösterimde
  `{"reject":{"code":"E_LOOKAHEAD","reason":"…"}}`. `program_check` onu geçerli ret diye
  tanır (`ok: true`, `reject {code}`); ret dokümana eklenmez.
- **Yalnız seri ve olay dokümana girer.** Sinyal ve durum çıktılı program yalnız denenir
  (`program_add` → `E_TRY_ONLY`).
- **Bütçe hataları** (`code: "limit"`): `E_COST_LIMIT` (bar başına maliyet), `E_WINDOW_LIMIT`
  (dönem), `E_TOO_LARGE` (bayt, çıktı, parametre, metin boyu), `E_TOO_MANY_NODES`, `E_BUDGET`
  (kişi başına canlı bütçe ya da günlük baytı; öneri kabulünde yeniden denetlenir, aşarsa
  öneri bekler). `expected` sınırı, `hint` ne yapılacağını söyler; denemede iş bütçesi
  dolarsa hata değil `stopped` gelir (öncesi geçerlidir).
- **Olay serisinin iki sınırı:** art arda olay barları tek bildirim verir; tek girdide birden
  çok bar kapanırsa aradaki tek barlık olay bildirimsiz geçer (seri ve kanıt yine doğru).
  `program_try` bunları `consecutive` ve `isolated` sayılarıyla gösterir (PS §1.2).

Komut satırında aynı çekirdek, oturumsuz (kayıt defteri varsayılan sınırlarıyla):

```sh
qp program check [--json] [--lang tr|en] prog.qp          # 0: geçerli ya da ret, 1: hata
qp program run [--json] [--full] --data seri.json prog.qp # kapalı pencere; --full bar başına
qp program explain [--json] [--lang tr|en] prog.qp
```

Dosya JSON nesnesiyse ara gösterim, değilse metin sayılır; veri `fixtures/bars` biçimi. Çıkış
kodları: 0 cevap basıldı, 1 programda hata (cevap basılır; `run`/`explain`'de ret de 1),
2 kullanım, dosya ya da veri hatası. Aynı girdide `qp program check --json` ile
`program_check`'in `data`'sı aynıdır (VERIFY 16, `crates/qp-cli/tests/program.rs`).

## Başvuru ajanları

`agents/` klasörü (ürün değil, örnek ve ölçüm düzeneği): **Yorumcu** (kapsül + kanıt + dış
bağlam → doğrulanmış yorum), **Nöbetçi** (niyetleri kurar, eşleşmeleri izler, yalnız anlamlı
olanı bildirir), **Soru cevaplayan** (kaynak göstererek cevap), **Programcı** (RL1: isteği
programa çevirir, denetler, dener, öneri olarak ekler; `agents/programci.md`). Çalıştırma:
`agents/README.md`.
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
