# RL1/U3 — Kötü niyet denetimi (GOAL §7, VERIFY 14)

Denetçi: ayrı alt ajan (uygulayıcı değil). Kaynak koda dokunulmadı. Sınanan yapı: dal `rl1/program`
(`7d12718f`, worktree `.claude/worktrees/program`), `cargo build -p qp-cli` (**debug** derleme;
süreler debug süreleridir), `CARGO_TARGET_DIR=target/wt/abuse` (iş sonunda silindi). Veri yalnız
sentetik (LCG rastgele yürüyüş, 1h, UTC); gerçek piyasa verisi kullanılmadı.

## Özet

| Hüküm | Sayı |
|---|---|
| OK | 109 |
| FAIL | 2 (aynı kusur, iki oturumda: P18A, P18B) — düşük önem |
| KOŞULLU | 3 (P03A, P04A, P05A) — ajan kuralı barındırıcının bağlantı işaretine bağlı |
| SINANAMADI | 1 (L20) — bu dalda program serisi değer hesabı yok |
| **Toplam** | **115 deneme** (CLI 53, `qp serve` 54, MCP 8) |

Sınıf başına: statik maliyet 33, çalışma bütçesi 5, geleceğe bakma 15, günlük şişirme 15,
politika 44, süreç güvenliği 3 (+ her denemede çökme/asılma denetimi: hiçbir girdide panik, yığın
taşması, sinyal ya da asılma yok; `qp serve` ve `qp mcp serve` her koşuda 0 ile çıktı).

**VERIFY 14 hükmü:** sınırı aşan program çalışmadan reddediliyor (S04–S10, S25, S31: `check`,
`run`, `explain` üçü de çıkış 1, çalıştırmadan); çalışma sırasında aşan duruyor (R01:
`stopped {reason: budget, bar 18914, work 50 002 050, limit 50 000 000}`, R03: durduktan sonra
kayıt doğmuyor); kişi başı toplam tutuyor (B03 örnek 32, B06/B07 canlı maliyet 2000, B09 günlük
baytı 4 MiB, B01/B02 komut başına 8 işlem; B04 undo, B10 undo döngüsü, B11–B13 öneri kabulü
yollarıyla da aşılamıyor). **Geçti**, aşağıdaki iki not ve bir sınanamayan özellikle.

## FAIL

### P18 — `try_program` / `check_program` iki girdiyi sessizce kabul ediyor (düşük)

PR §11.2: `E_NOT_BAR_SERIES` "`inputs` tam bir bar serisi değil (yok, **iki tane**, hesap serisi)".
`add_program` buna uyuyor (`invalid/E_NOT_BAR_SERIES`, "a program takes exactly one bar series,
got 2"); sorgular uymuyor: `qp-program/src/asked.rs:100` `a.inputs.first()` alıyor, fazlası yok
sayılıyor. Yetki atlatma **yok**: kullanılan ilk seri yetkiyle denetleniyor (`["bx","px"]` ile
yetkisiz `can` `no_entitlement` alıyor; `["px","bx"]` ile px'te koşuyor, bx'e hiç dokunmuyor).

Yeniden üretme (`qp serve --batch-end --init <init-a.json>`; `run_session.py`'nin `init()`'i):

```
{"conn":"a","msg":{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"headless"}}}
{"conn":"a","msg":{"type":"query","id":"q","query":{"kind":"try_program","inputs":["px","bx"],
  "program":{"version":"qp-ir/0","nodes":[{"id":"c","op":"input","field":"close"},
  {"id":"m","op":"sma","x":"c","period":10}],"outputs":[{"name":"y","kind":"series","node":"m"}]}}}}
→ {"type":"result",…,"data":{"kind":"program_try",…}}     beklenen: error invalid/E_NOT_BAR_SERIES
```

`check_program` aynı girdiyle `ok: true` döner.

## KOŞULLU (ajan politikası taşıma işaretine bağlı)

`qp serve` (stdin) init'te `agent_policy` yokken hiçbir bağlantıyı ajan bağlantısı olarak
işaretlemez; otoritenin ajan kuralı (R5B §25.3) ve onunla `OpHandler::agent` (PS §2.2) yalnız
işaretli bağlantıda (`qp mcp serve`) ya da `agent_policy: "r5b"` ile çalışır. Bu yüzden oturum A'da:

- P03A: `proposer` rollü ajan `remove_program` → `proposed` (öneri oldu; PS §2.2 "asla").
- P04A: `editor` rollü ajan (`scout`) `add_program` → **doğrudan uygulandı** (öneri değil).
- P05A: aynı ajan `remove_program` → **doğrudan uygulandı**.

Aynı denemeler oturum B'de (`agent_policy: "r5b"`, P01B–P05B) ve MCP'de (M02–M06) beklendiği
gibi: `forbidden/proposal_only`, `forbidden/agent_cannot_write`, `forbidden/human_only`. Program
alt sistemine özgü bir kusur değil, R5B'den gelen taşıma tasarımı; ama PS §3.3 "süreç içi, `qp
serve` ve MCP aynı mesajları taşır" der ve PS §2.2 ajan politikasını taşımadan bağımsız yazar.
Karar ana akışın: ya `qp serve`'de ajan ilkesi için kural varsayılan olur, ya da PS §2.2'ye "ajan
kuralı işaretli bağlantıda" notu düşülür.

## SINANAMADI

**L20 — dokümandaki olay serisinde geriye dönük tetik yokluğu (PS §1.2 `since_t`).** Senaryo:
`lv.e ge 1` niyeti; `set_program_params` her geçmiş barı olay yapar (dene: 2999 olay) → bildirim
yok (beklenen); ama `since_t` sonrasında kapanan barlarda da bildirim yok (olumlu denetim
başarısız; aynı oturumda düz `px ge 500` niyeti tetikleniyor). Neden: `rl1/program` dalı RL1/PA'yı
(`qp-plane` program serisi hesabı, main'de `356bb3dc`) içermiyor; `study` sorgusu "unknown study
`program`" diyor, `at` program serisini listelemiyor. Yani bu dalda dokümandaki program serisinin
değeri hiç hesaplanmıyor; "tetik yok" bir kanıt değil. **PA + program birleşince L20 yeniden
koşulmalı** (`run_session.py` içinde hazır).

## Gözlemler (kusur değil, bilgi)

- **Doküman geneli tavan yok** (B05): iki kişi 32+32 = 64 örnek açabiliyor. Şartname yalnız kişi
  başı sınır tanımlıyor (PR §8.2; çok doküman RS1). "Doküman toplamı" = kişi sayısı × kişi bütçesi.
- **Canlı maliyet tek örnekte kişi sınırını aşabilir mi:** B07'de `per_bar` 604 (≤ 666) ama 4
  çıktı × koni 601 = `live_per_bar` 2404 olan program `check`'ten geçiyor, `add_program`'da
  `E_BUDGET` alıyor. Doğru, ama `check_program` cevabında `ok: true` ile yan yana duruyor; ajan
  için `live_per_bar > program.live.doc.max_cost_per_bar` bir uyarı olarak söylenebilir (öneri).
- **Günlük şişirme hızı:** 8 KB × 4 çıktılık örnekte her `set_program_params` ≈ 34 KB; sınıra 123
  değişimde varılıyor (B09), komut başına ≈ 12,7 ms (debug), oturum belleği en çok ≈ 85 MB.
- **Öneri oluşurken bütçe** (B11a): bütçe doluyken ajanın yeni önerisi daha öneri olmadan
  `E_BUDGET` alıyor; dolmadan verilmiş öneri kabulde yeniden denetleniyor (B11) ve tek komutta
  birden çok `accept_proposal` otoritece zaten yasak (B12: "must be the only op of its command").
- **Komut satırı** `check`, saati bağsız denetler (PS §3.2: `inputs` yoksa saat bağdan bağımsız):
  `close@1h`/`close@30m` `check`'ten geçer, `run`'da `E_UNKNOWN_TF` (L04/L05). Tasarım gereği.
- **Sentetik besleme saati aşıyor:** `init.data.feeds[].synthetic` barları oturum saatinin
  ötesine uzanıyor (deneme düzeneği; program işi değil). `try_program` bağlı seride bu barları
  kapanmış sayıyor. Barındırıcı verisinin saatle tutarlılığı program alt sisteminin denetimi
  dışında; not olarak.

## Yöntem

- `qpabuse.py`: ortak parçalar (sentetik bar, `/usr/bin/time -l` ile CLI çağrısı ve en yüksek
  bellek, `qp serve --batch-end` istemcisi, yalnız kendi sürecini PID ile kapatma).
- `run_cli.py`: `qp program check|run|explain` (sınıf 1, 2, 3, 6). Önek kararlılığı (L09–L13):
  aynı program `n` ve `n+k` (k ∈ {1, 7, 24, 61}) barlık kapalı pencerede `run --full`; ilk `n`
  barın seri değerleri, olay barları, durum anahtarları bayt eşit; sinyalde kapanmış sonuçlar
  uzun koşunun alt kümesi. L14: `align(close@1d)` değeri bağımsız Python hesabıyla (kapanışı ≤ bar
  kapanışı olan son UTC günü) bar bar eşit.
- `run_session.py`: `qp serve` iki oturum (A: `agent_policy` yok; B: `"r5b"`), ilkeler: `arkin`
  (owner), `deniz` (editor, insan), `claude` (ajan, proposer, Arkın adına), `scout` (ajan,
  editor), `can` (viewer, veri yetkisi yok), `host`. Belge içi `px` (3000 × 1h) ve bağlı `bx`
  (sentetik besleme `bist/THYAO`).
- `run_mcp.py`: `qp mcp serve --stdio --surface thin --principal claude` (bağlantı ajan işaretli).
- Girdi dosyaları `inputs/` (küçük); büyük girdiler (50 MB ara gösterim, 200 000 derin JSON,
  60 000 parantez, 25 000 bar) koşuda `ABUSE_WORK`'e üretilir ve silinir.
- Yeniden koşma: `CARGO_TARGET_DIR=<…> ABUSE_WORK=<geçici> python3 run_cli.py && python3
  run_session.py && python3 run_mcp.py` → `results-{cli,session,mcp}.json`.

## Öneriler (kapsam dışı; yapılmadı)

1. P18: sorgularda `inputs.len() != 1` → `E_NOT_BAR_SERIES` (add_program ile aynı yol).
2. KOŞULLU: `qp serve` ajan ilkeleri için kural varsayılanı ya da PS §2.2'ye taşıma notu.
3. L20'yi PA + program birleşmesinden sonra yeniden koş; dökümde (75 `program-event`) zaten
   istenen olumlu denetim burada da hazır.
4. `check_program` cevabında `live_per_bar` kişi sınırını tek başına aşıyorsa uyarı (`W_`).
5. `study`/`at` sorgularının program serisini okuması PA ile gelmeli; gelmiyorsa ayrı iş.

## Denemeler

| # | Sınıf | Yüzey | Girdi | Beklenen | Gözlenen | Süre ms / bellek MB | Hüküm |
|---|---|---|---|---|---|---|---|
| S01 | 1-statik | CLI | 200 düğüm (sınır), abs zinciri | ok | exit 0, reasons [], cost 200 | 10 / 12.7 | **OK** |
| S02 | 1-statik | CLI | 201 düğüm | E_TOO_MANY_NODES | exit 1, reasons ['E_TOO_MANY_NODES'] | 8 / 11.0 | **OK** |
| S03 | 1-statik | CLI | stdev(close,665): per_bar tam 666 = sınır | ok | exit 0, reasons [], cost 666 | 7 / 12.8 | **OK** |
| S04 | 1-statik | CLI | stdev(close,666): per_bar 667 = sınır+1 | E_COST_LIMIT | exit 1, reasons ['E_COST_LIMIT'], cost 667 | 6 / 12.9 | **OK** |
| S05 | 1-statik | CLI | percentrank(close,666) | E_COST_LIMIT | exit 1, reasons ['E_COST_LIMIT'], cost 667 | 7 / 13.0 | **OK** |
| S06 | 1-statik | CLI | stdev(close,2000) (azami pencere) | E_COST_LIMIT | exit 1, reasons ['E_COST_LIMIT'], cost 2001 | 7 / 12.9 | **OK** |
| S07 | 1-statik | CLI | sma(close,2001) (pencere+1) | E_WINDOW_LIMIT | exit 1, reasons ['E_WINDOW_LIMIT'] | 7 / 12.8 | **OK** |
| S08 | 1-statik | CLI | üst periyot indirimi yok: 2×stdev(close@1d,333) | E_COST_LIMIT | exit 1, reasons ['E_COST_LIMIT'], cost 668 | 6 / 13.2 | **OK** |
| S09 | 1-statik | CLI | 5 çıktı (sınır 4) | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE'] | 6 / 12.5 | **OK** |
| S10 | 1-statik | CLI | kanonik ara gösterim > 8192 bayt (150 düğüm, 60 karakterlik kimlik) | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE'] | 7 / 10.8 | **OK** |
| S11 | 1-statik | CLI | 50 MB ara gösterim dosyası | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE'], rss 10.2 MB | 7 / 10.2 | **OK** |
| S12 | 1-statik | CLI | 200 000 derinlikte JSON `[` | E_PARSE | exit 1, reasons ['E_PARSE'] | 7 / 10.5 | **OK** |
| S13 | 1-statik | CLI | 60 000 iç içe parantez (metin) | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE'] | 7 / 10.8 | **OK** |
| S14 | 1-statik | CLI | 7000 terimli `close + close + …` (sol derin ağaç) | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE'] | 13 / 12.9 | **OK** |
| S15 | 1-statik | CLI | 60 000 tekli eksi `----…close` | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE'] | 14 / 18.3 | **OK** |
| S16 | 1-statik | CLI | 15 000 `not` zinciri | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE'] | 15 / 13.2 | **OK** |
| S17 | 1-statik | CLI | 20 000 `[1]` gecikme zinciri | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE'] | 19 / 19.0 | **OK** |
| S18 | 1-statik | CLI | 5000 karakterlik ad (metin) | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE', 'E_TOO_LARGE'] | 8 / 11.0 | **OK** |
| S19 | 1-statik | CLI | 65 karakterlik düğüm kimliği (ara gösterim; desen ≤ 64) | E_BAD_PARAM | exit 1, reasons ['E_BAD_PARAM'] | 7 / 12.1 | **OK** |
| S20 | 1-statik | CLI | Unicode/sıfır genişlik/sağdan-sola ad: `çizgi`, `a\u200bb`, `\u202eevil` | E_PARSE | exit 1, reasons ['E_PARSE', 'E_PARSE', 'E_PARSE'] | 7 / 11.1 | **OK** |
| S21 | 1-statik | CLI | çıktı adı emoji `📈` |  | exit 1, reasons ['E_BAD_PARAM'] | 7 / 12.1 | **OK** |
| S22 | 1-statik | CLI | 17 parametre (sınır 16) | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE'], cost 1 | 7 / 12.3 | **OK** |
| S23 | 1-statik | CLI | `param p int = 10 in 1..1000000000000000000`, sma(close,p) | E_PARAM_RANGE | exit 1, reasons ['E_PARAM_RANGE'] | 7 / 13.0 | **OK** |
| S24 | 1-statik | CLI | parametreyle gizli maliyet: `param p int = 10 in 1..2000`, stdev(close,p) | ok | exit 0, reasons [], cost 11 (statik maliyet bağlanan değerle; S2x ve oturum B-sınıfı yeniden denetler) | 7 / 13.0 | **OK** |
| S25 | 1-statik | CLI | S24 + bağlanan değer p=2000 (ara gösterim + --params yok → CLI varsayılanı) | E_COST_LIMIT | exit 1, reasons ['E_COST_LIMIT'], cost 2001 | 7 / 13.0 | **OK** |
| S26 | 1-statik | CLI | sonraki düğüme başvuru (döngü denemesi) | E_UNKNOWN_REF | exit 1, reasons ['E_UNKNOWN_REF'] | 6 / 12.0 | **OK** |
| S27 | 1-statik | CLI | kendine başvuru | E_UNKNOWN_REF | exit 1, reasons ['E_UNKNOWN_REF'] | 7 / 12.0 | **OK** |
| S28 | 1-statik | CLI | 199 bilinmeyen düğüm (hata seli) | E_UNKNOWN_OP | exit 1, reasons ['E_UNKNOWN_OP', 'E_UNKNOWN_OP', 'E_UNKNOWN_OP', 'E_UNKNOWN_OP'], truncated, 50 hata | 23 / 13.2 | **OK** |
| S29 | 1-statik | CLI | sayı taşması `1e400` sabit |  | exit 1, reasons ['E_PARSE'] | 7 / 10.2 | **OK** |
| S30 | 1-statik | CLI | metin 70 000 karakter (sınır 65 536) yorum satırları | E_TOO_LARGE | exit 1, reasons ['E_TOO_LARGE'] | 8 / 10.8 | **OK** |
| S31-explain | 1-statik | CLI | `qp program explain` sınır+1 programı (S04) | çalışmadan ret (çıkış 1) | exit 1, reasons ['E_COST_LIMIT'] | 7 / 12.9 | **OK** |
| S31-run | 1-statik | CLI | `qp program run` sınır+1 programı (S04) | çalışmadan ret (çıkış 1) | exit 1, reasons ['E_COST_LIMIT'] | 8 / 13.5 | **OK** |
| S32 | 1-statik | CLI | 185 düğümlü Fibonacci DAG (her düğüm önceki ikisini okur; saf koni yürüyüşü 2^n), 4 çıktı, 20 000 bar | doğrusal maliyet; hızlı biter | exit 0, cost 187/730, 530 ms | 530 / 32.6 | **OK** |
| R01 | 2-çalışma | CLI | 4 sinyal çıktısı her barda, max_bars 500, + stdev 600 maliyeti, 25 000 bar | çalışma bütçesiyle durur: stopped{reason: budget, bar, t, work, limit} | exit 0, stopped {'bar': 18914, 'limit': 50000000, 'reason': 'budget', 't': 1790157600000, 'work': 50002050}, work {'limit': 50000000, 'used': 49999420}, from_t 1722067200000 | 1121 / 48.4 | **OK** |
| R02 | 2-çalışma | CLI | 25 000 barlık dosya, okuma penceresi | son 20 000 bar: from_t = 1722067200000 | from_t 1722067200000 | 1121 / 48.4 | **OK** |
| R03 | 2-çalışma | CLI | R01 `--full`: durduktan sonra kayıt doğuyor mu | durma barından sonra kayıt yok | stopped.t 1790157600000, kayıt 75656, sonra doğan 0 | 5829 / 958.3 | **OK** |
| R04 | 2-çalışma | CLI | 4 boyutlu durum tanımı, 8'er kenar (≤ 9^4 anahtar) | biter ya da durur; çökme yok | exit 0, state [{'distinct': 575, 'name': 's', 'top': [{'count': 105, 'key': {'a': 0, 'b': 6, 'c': 6, 'd': 6}}, {'count': 102, 'key': {, stopped None | 151 / 32.1 | **OK** |
| R05 | 2-çalışma | CLI | her barda olay (const true), 20 000 bar | W_CONSECUTIVE_EVENTS; events 20000, runs 1 | events 20000, runs 1, work {'limit': 50000000, 'used': 340020} | 94 / 31.1 | **OK** |
| L01 | 3-gelecek | CLI | lag -1 (ara gösterim) | E_LOOKAHEAD | exit 1, reasons ['E_LOOKAHEAD'] | 9 / 12.1 | **OK** |
| L02 | 3-gelecek | CLI | `partial: true` üst periyot girdisi | E_LOOKAHEAD | exit 1, reasons ['E_LOOKAHEAD'] | 9 / 11.9 | **OK** |
| L03 | 3-gelecek | CLI | metinde `close[-1]` | E_LOOKAHEAD | exit 1, reasons ['E_LOOKAHEAD'] | 8 / 12.8 | **OK** |
| L04 | 3-gelecek | CLI | `close@1h` 1h tabanlı veriyle `run` (taban = üst periyot; `check` bağsız saati denetlemez, PS §3.2) | E_UNKNOWN_TF, çalışmaz | exit 1, reasons ['E_UNKNOWN_TF'] | 10 / 13.3 | **OK** |
| L05 | 3-gelecek | CLI | `close@30m` 1h tabanlı veriyle `run` (tabandan küçük; `check` bağsız saati denetlemez, PS §3.2) | E_UNKNOWN_TF, çalışmaz | exit 1, reasons ['E_UNKNOWN_TF'] | 9 / 13.3 | **OK** |
| L06 | 3-gelecek | CLI | `align(close)` (tabanın hizası) |  | exit 1, reasons ['E_CLOCK_MISMATCH'] | 8 / 12.7 | **OK** |
| L07 | 3-gelecek | CLI | `reject E_LOOKAHEAD` metni | ok | exit 0, reasons [], reject {'code': 'E_LOOKAHEAD', 'reason': 'next close unknown'} | 7 / 11.2 | **OK** |
| L08 | 3-gelecek | CLI | `qp program run` ret dosyası | çalışacak program yok (çıkış 1) | exit 1 | 9 / 12.0 | **OK** |
| L09 | 3-gelecek | CLI | üst periyot: align(sma(close@1d,3)), align(close@4h), olay | ilk n bar n+k koşusuyla bayt eşit (sinyal: kapanmış sonuçlar alt küme) | çıktılar ['sd', 'sh', 'x']; fark yok | 14 / 14.6 | **OK** |
| L10 | 3-gelecek | CLI | ema/rsi/percentrank/stdev/bollinger/macd | ilk n bar n+k koşusuyla bayt eşit (sinyal: kapanmış sonuçlar alt küme) | çıktılar ['b', 'e', 'p', 'r']; fark yok | 18 / 14.6 | **OK** |
| L11 | 3-gelecek | CLI | line: çapalar verinin ötesinde (gelecek t) | ilk n bar n+k koşusuyla bayt eşit (sinyal: kapanmış sonuçlar alt küme) | çıktılar ['above', 'sf', 'sl']; fark yok | 14 / 14.5 | **OK** |
| L12 | 3-gelecek | CLI | time/bar_index hileleri + bars_since/cooldown/rearm/streak | ilk n bar n+k koşusuyla bayt eşit (sinyal: kapanmış sonuçlar alt küme) | çıktılar ['e', 'o', 's', 't']; fark yok | 17 / 14.8 | **OK** |
| L13 | 3-gelecek | CLI | sinyal (max_bars 48) + durum tanımı | ilk n bar n+k koşusuyla bayt eşit (sinyal: kapanmış sonuçlar alt küme) | çıktılar ['sg', 'st']; fark yok | 19 / 15.8 | **OK** |
| L14 | 3-gelecek | CLI | align(close@1d): bağımsız hesapla, gün kapanmadan değer yok | her bar = kapanışı ≤ bar kapanışı olan son günün kapanışı | farklı bar 0 (ilk []) | 10 / 13.2 | **OK** |
| L20 | 3-gelecek | serve | geriye dönük tetik: niyet `lv.e ge 1`; set_program_params her geçmiş barı olay yapar (dene: olay sayısı); sonra since_t'den önce ve sonra kapanan barlar | değişimde ve since_t öncesi kapanan barda bildirim yok; since_t sonrası kapanan barda var | dene olayları 2999; değişimde 0; 1 bar sonra 0; 3 bar sonra 0 — olumlu denetim de tetiklemedi: bu dalda (rl1/program) qp-plane program serisi hesabı (RL1/PA, main'de 356bb3dc) yok; `study`/`at` sorguları program serisini görmüyor. Geriye dönük tetik yokluğu… | – / – | **SINANAMADI** |
| B01 | 4-günlük | serve | tek komutta 9 add_program (sınır 8) | limit/E_TOO_LARGE, hiçbiri uygulanmaz | error limit/E_TOO_LARGE at /ops/8 | 1.4 / 27.0 | **OK** |
| B02 | 4-günlük | serve | tek komutta 64 set_program_params | limit/E_TOO_LARGE (/ops/8) | error limit/E_TOO_LARGE at /ops/8 | 1.8 / – | **OK** |
| B03 | 4-günlük | serve | arkin 40 kez add_program (sınır 32 örnek) | 32 kabul, 33. limit/E_BUDGET | 32 kabul, sonra error limit/E_BUDGET at /ops/0 | 610 / 30.4 | **OK** |
| B04 | 4-günlük | serve | sınırdayken: remove_program, yeni add_program (32), sonra remove'un undo'su (33. örnek) | undo limit/E_BUDGET | remove ack applied  created 0; add ack applied  created 1; undo error limit/E_BUDGET at /ops/0 | 2.6 / – | **OK** |
| B05 | 4-günlük | serve | deniz (ikinci kişi) 32 örnek; doküman toplamı | kişi başı ayrı bütçe (32+32); doküman geneli tavan şartnamede yok | deniz 32 kabul; dokümanda 64 örnek | – / – | **OK** |
| B06 | 4-günlük | serve | deniz 4 × stdev(close,665) örneği (live_per_bar 666; kişi sınırı 2000) | 3 kabul (1998), 4. limit/E_BUDGET | ack applied  created 1; ack applied  created 1; ack applied  created 1; error limit/E_BUDGET at /ops/0 | – / – | **OK** |
| B07 | 4-günlük | serve | tek örnek: per_bar 604 (≤666) ama 4 çıktı × koni 601 = live_per_bar 2404 | limit/E_BUDGET (canlı maliyet kişi sınırını tek başına aşar) | error limit/E_BUDGET at /ops/0 | 1.8 / – | **OK** |
| B08 | 4-günlük | serve | deniz bütçe doluyken h0'a boş set_program_params | kabul (maliyet değişmez) ya da kodlu ret; çökme yok | ack applied  created 0 | 3.5 / – | **OK** |
| B09 | 4-günlük | serve | set_program_params döngüsü, ara gösterim 7948 B × 4 çıktı (günlük sınırı 4 MiB) | ≈ 120 değişimden sonra limit/E_BUDGET | 123 değişim kabul, sonra error limit/E_BUDGET at /ops/0; komut başına ort. 18.8 ms, en çok 105.1 ms | 3901 / 49.1 | **OK** |
| B10 | 4-günlük | serve | bütçe dolunca undo (eski değerleri geri yazar) | limit/E_BUDGET (undo'nun baytı da sayılır) | error limit/E_BUDGET at /ops/0 | 3.4 / – | **OK** |
| B11 | 4-günlük | serve | dolmadan önce verilmiş ajan önerisi (set_program_params); bütçe dolunca insan tek başına kabul eder | kabul limit/E_BUDGET, öneri bekler | öneriler ['prop-140', 'prop-141', 'prop-142']; kabul error limit/E_BUDGET at /ops/0 | 8.0 / – | **OK** |
| B11a | 4-günlük | serve | bütçe doluyken ajan yeni set_program_params önerir | öneri oluşurken limit/E_BUDGET (PS §2.3) | error limit/E_BUDGET at /ops/0 | – / – | **OK** |
| B12 | 4-günlük | serve | aynı, ama iki accept_proposal tek komutta (Check fazı yalnız tek başına kabulü denetler) | limit/E_BUDGET ya da kabul hiç olmaz | error invalid/None at /ops/0 — `accept_proposal` must be the only op of its command; (B12 sonrası) | 0.2 / – | **OK** |
| B13 | 4-günlük | serve | aynı, accept_proposal + zararsız bir işlem (add_comment) tek komutta | limit/E_BUDGET ya da kabul hiç olmaz | error schema/None at /ops/1 — missing field `cites`; bekleyen öneriler ['prop-140', 'prop-141', 'prop-142']; arkin'in günlükteki program baytı ≈ 4179027 (sınır 4194304) | 1.2 / – | **OK** |
| B14 | 4-günlük | serve | 1,1 MB'lık komut satırı | limit (ileti boyutu) | error limit/None at None | 0.5 / – | **OK** |
| M01 | 5-politika | MCP | MCP (thin yüzey) araçları | program araçları listede | ['op_add_program', 'op_set_program_params', 'op_remove_program', 'query_check_program', 'query_try_program', 'query_explain_program', 'program_check', 'program_try', 'program_explain', 'program_add'] | – / – | **OK** |
| M02 | 5-politika | MCP | MCP op_add_program mode:direct | forbidden/proposal_only | error forbidden/proposal_only | – / – | **OK** |
| M03 | 5-politika | MCP | MCP op_add_program (öneri) sonra op_remove_program | öneri; remove forbidden/agent_cannot_write | ack proposed prop-2; error forbidden/agent_cannot_write | – / – | **OK** |
| M04 | 5-politika | MCP | MCP program_add (metin) | öneri | ack proposed prop-3 | – / – | **OK** |
| M05 | 5-politika | MCP | MCP command accept_proposal (ajan karar veremez) | forbidden | error forbidden/human_only | – / – | **OK** |
| M06 | 5-politika | MCP | MCP command put_series program alanıyla (sahip `nobody`) | forbidden ya da E_PROGRAM_SERIES | error forbidden/agent_cannot_write | – / – | **OK** |
| M07 | 5-politika | MCP | MCP program_check 100 000 parantez | kodlu ret, süreç ayakta | error limit/None | – / – | **OK** |
| M08 | 5-politika | MCP | MCP sunucusu kapanışı | temiz çıkış (0) | çıkış 0 | – / – | **OK** |
| P01A | 5-politika | serve | ajan (proposer) add_program mode:direct | forbidden/proposal_only | error forbidden/None at /ops/0 | 0.4 / – | **OK** |
| P01B | 5-politika | serve | ajan (proposer) add_program mode:direct | forbidden/proposal_only | error forbidden/proposal_only at /ops/0 | 0.2 / – | **OK** |
| P02A | 5-politika | serve | ajan add_program → öneri; ajan kendi önerisini kabul; editör (deniz, decide yok) kabul; sahip kabul | öneri; ajan forbidden; deniz forbidden; arkin kabul, owner = arkin | ack proposed prop-267 created 2; ajan error forbidden/human_only at /ops/0; deniz error forbidden/None at /ops/0; arkin ack applied  created 2; owner arkin | – / – | **OK** |
| P02B | 5-politika | serve | ajan add_program → öneri; ajan kendi önerisini kabul; editör (deniz, decide yok) kabul; sahip kabul | öneri; ajan forbidden; deniz forbidden; arkin kabul, owner = arkin | ack proposed prop-2 created 2; ajan error forbidden/human_only at /ops/0; deniz error forbidden/None at /ops/0; arkin ack applied  created 2; owner arkin | – / – | **OK** |
| P03A | 5-politika | serve | ajan (proposer) remove_program; oturum A | forbidden/agent_cannot_write (öneri bile değil) | ack proposed prop-269 created 0 | 0.5 / – | **KOŞULLU** |
| P03B | 5-politika | serve | ajan (proposer) remove_program; oturum B | forbidden/agent_cannot_write (öneri bile değil) | error forbidden/agent_cannot_write at /ops/0 | 0.2 / – | **OK** |
| P04A | 5-politika | serve | yazma yetkili ajan (scout, editor) add_program; oturum A | öneri (PS §2.2: ajan yalnız önerir) | ack applied  created 1  [ajan kuralı bağlantı işaretine bağlı: `qp serve` işaretlemez, init'te agent_policy yok] | – / – | **KOŞULLU** |
| P04B | 5-politika | serve | yazma yetkili ajan (scout, editor) add_program; oturum B | öneri (PS §2.2: ajan yalnız önerir) | ack proposed prop-4 created 1 | – / – | **OK** |
| P05A | 5-politika | serve | yazma yetkili ajan remove_program; oturum A | forbidden/agent_cannot_write | ack applied  created 0 | 0.6 / – | **KOŞULLU** |
| P05B | 5-politika | serve | yazma yetkili ajan remove_program; oturum B | forbidden/agent_cannot_write | error forbidden/agent_cannot_write at /ops/0 | 0.2 / – | **OK** |
| P06A | 5-politika | serve | insan put_series: program serisini sahibi `nobody` yapılmış kopyayla yazar (bütçe atlatma) | invalid/E_PROGRAM_SERIES | error invalid/E_PROGRAM_SERIES at /ops/0/series | – / – | **OK** |
| P06B | 5-politika | serve | insan put_series: program serisini sahibi `nobody` yapılmış kopyayla yazar (bütçe atlatma) | invalid/E_PROGRAM_SERIES | error invalid/E_PROGRAM_SERIES at /ops/0/series | – / – | **OK** |
| P07A | 5-politika | serve | insan put_series: yeni kimlikle, program alanı taşıyan seri (denetimsiz ara gösterim) | invalid/E_PROGRAM_SERIES | error invalid/E_PROGRAM_SERIES at /ops/0/series | – / – | **OK** |
| P07B | 5-politika | serve | insan put_series: yeni kimlikle, program alanı taşıyan seri (denetimsiz ara gösterim) | invalid/E_PROGRAM_SERIES | error invalid/E_PROGRAM_SERIES at /ops/0/series | – / – | **OK** |
| P08A | 5-politika | serve | ajan put_series program alanıyla | E_PROGRAM_SERIES ya da forbidden | error invalid/E_PROGRAM_SERIES at /ops/0/series | – / – | **OK** |
| P08B | 5-politika | serve | ajan put_series program alanıyla | E_PROGRAM_SERIES ya da forbidden | error forbidden/agent_cannot_write at /ops/0 | – / – | **OK** |
| P09A | 5-politika | serve | set_params program serisine | invalid/E_PROGRAM_SERIES | error invalid/E_PROGRAM_SERIES at /ops/0/series | – / – | **OK** |
| P09B | 5-politika | serve | set_params program serisine | invalid/E_PROGRAM_SERIES | error invalid/E_PROGRAM_SERIES at /ops/0/series | – / – | **OK** |
| P10A | 5-politika | serve | remove tek çıktı | invalid/E_PROGRAM_SERIES | error invalid/E_PROGRAM_SERIES at /ops/0/id | – / – | **OK** |
| P10B | 5-politika | serve | remove tek çıktı | invalid/E_PROGRAM_SERIES | error invalid/E_PROGRAM_SERIES at /ops/0/id | – / – | **OK** |
| P11A | 5-politika | serve | append_bars / update_last_bar program serisine | ret (bar serisi değil) | error invalid/None at /ops/0/series; error invalid/None at /ops/0/series | – / – | **OK** |
| P11B | 5-politika | serve | append_bars / update_last_bar program serisine | ret (bar serisi değil) | error invalid/None at /ops/0/series; error invalid/None at /ops/0/series | – / – | **OK** |
| P12A | 5-politika | serve | olay serisi compare işleneni (add_intent, add_finding) | unsupported/E_EVENT_OPERAND | error unsupported/E_EVENT_OPERAND at /ops/0/trigger/left; error unsupported/E_EVENT_OPERAND at /ops/0/invalidation/right | – / – | **OK** |
| P12B | 5-politika | serve | olay serisi compare işleneni (add_intent, add_finding) | unsupported/E_EVENT_OPERAND | error unsupported/E_EVENT_OPERAND at /ops/0/trigger/left; error unsupported/E_EVENT_OPERAND at /ops/0/invalidation/right | – / – | **OK** |
| P13A | 5-politika | serve | add_program sinyal çıktılı | unsupported/E_TRY_ONLY | error unsupported/E_TRY_ONLY at /ops/0/program/outputs/0/kind | – / – | **OK** |
| P13B | 5-politika | serve | add_program sinyal çıktılı | unsupported/E_TRY_ONLY | error unsupported/E_TRY_ONLY at /ops/0/program/outputs/0/kind | – / – | **OK** |
| P14A | 5-politika | serve | add_program girdisi program serisi | invalid/E_NOT_BAR_SERIES | error invalid/E_NOT_BAR_SERIES at /ops/0/inputs/0 | – / – | **OK** |
| P14B | 5-politika | serve | add_program girdisi program serisi | invalid/E_NOT_BAR_SERIES | error invalid/E_NOT_BAR_SERIES at /ops/0/inputs/0 | – / – | **OK** |
| P15A | 5-politika | serve | add_program `owner` alanı (sahip sahteciliği) / metin `source` | schema (bilinmeyen alan) | error schema/None at /ops/0/owner; error schema/None at /ops/0/source | – / – | **OK** |
| P15B | 5-politika | serve | add_program `owner` alanı (sahip sahteciliği) / metin `source` | schema (bilinmeyen alan) | error schema/None at /ops/0/owner; error schema/None at /ops/0/source | – / – | **OK** |
| P16A | 5-politika | serve | yetkisiz okuyucu (can): try_program bx; try_program instance (bx'e bağlı); explain_program at; check_program inputs bx | try/explain no_entitlement; check bar verisi sızdırmaz | add ack applied  created 1; try no_entitlement/None; try-inst no_entitlement/None; explain-at no_entitlement/None; check bars=None | – / – | **OK** |
| P16B | 5-politika | serve | yetkisiz okuyucu (can): try_program bx; try_program instance (bx'e bağlı); explain_program at; check_program inputs bx | try/explain no_entitlement; check bar verisi sızdırmaz | add ack applied  created 1; try no_entitlement/None; try-inst no_entitlement/None; explain-at no_entitlement/None; check bars=None | – / – | **OK** |
| P17A | 5-politika | serve | try_program uç aralık (i64 min .. u64 max); program+source birlikte | kodlu ret, çökme yok; ikisi birden invalid | error schema/None at /query/range/to_t; error invalid/None at /query | – / – | **OK** |
| P17B | 5-politika | serve | try_program uç aralık (i64 min .. u64 max); program+source birlikte | kodlu ret, çökme yok; ikisi birden invalid | error schema/None at /query/range/to_t; error invalid/None at /query | – / – | **OK** |
| P18A | 5-politika | serve | try_program / check_program iki girdiyle (px, bx); yetkisiz can için de | E_NOT_BAR_SERIES (PR §11.2 "yok, iki tane"; add_program öyle yapıyor) | try(arkin) {"type": "result", "id": "q315", "seq": 273, "now": 17148708; try(can) {"type": "result", "id": "q317", "seq": 273, "now": 17148708; check ok=True — fazla girdi sessizce yok sayılıyor (ilki kullanılıyor; sıra bx,px ise can no_entitlement alıyor: … | – / – | **FAIL** |
| P18B | 5-politika | serve | try_program / check_program iki girdiyle (px, bx); yetkisiz can için de | E_NOT_BAR_SERIES (PR §11.2 "yok, iki tane"; add_program öyle yapıyor) | try(arkin) {"type": "result", "id": "q31", "seq": 6, "now": 17148708000; try(can) {"type": "result", "id": "q33", "seq": 6, "now": 17148708000; check ok=True — fazla girdi sessizce yok sayılıyor (ilki kullanılıyor; sıra bx,px ise can no_entitlement alıyor: … | – / – | **FAIL** |
| R06 | 6-süreç | CLI | veri dosyası: f64::MAX kapanış + yinelenen t | kodlu ret ya da sonlu cevap; çökme yok | exit 2, qp: bar t=1704081600000 is not after the last bar t=1704081600000 | 9 / 12.7 | **OK** |
| X01 | 6-süreç | serve | oturuma bozuk JSON, 120 derin ara gösterim, 100 000 derin JSON satırı, 50 000 parantez ve 15 000 terimli metin, 5000 düğümlü explain | her biri kodlu hata; süreç ayakta; sonraki sorgu cevaplanır | ['parse/None', 'schema/E_TYPE', 'parse/None', 'limit/None', 'limit/None', 'limit/E_TOO_LARGE']; ayakta True; sonraki {"type": "result", "id": "q325", "seq":  | 56 / 83.8 | **OK** |
| X02 | 6-süreç | serve | iki oturumun sonu | süreç temiz çıkar (0) | A çıkış 0 (en çok 83.8 MB), B çıkış 0 (en çok 31.5 MB) | 7450 / 83.8 | **OK** |

Süre: duvar saati (CLI: süreç; oturum: tek komut ya da döngünün toplamı). Bellek: CLI için süreç en yüksek RSS, oturum için o ana kadar `qp serve`in en yüksek RSS'i (`ps`). Debug derleme.

## Ek (RL1/C, 2026-11-26): L20 yeniden koşuldu — **OK**

PA + PB `main`'de birleştikten sonra (`64e30256` üstünde `rl1/close-c`), `run_session.py`'nin
`lookahead_session`'ı tek başına, aynı girdiyle (`init-a.json`, saatlik belge içi `px`, saat = son
bar + 2 saat) `qp serve` (release) üstünde koşuldu; öbür satırlar ve `results-session.json`
yeniden üretilmedi. Gözlenen: `dene` olayları 2999; `set_program_params` anında bildirim **0**;
1 bar eklenince **0** (kapanan barın `known_t` = son bar + 1 sa ≤ `since_t`); 2 bar daha eklenince
**1** (`known_t` = son bar + 3 sa > `since_t` olan bar kapandı). Beklenen tam bu: değişimde ve
`since_t` öncesi kapanan barda tetik yok, sonrasında var (olumlu denetim artık tetikliyor).
Dökümde de aynı kural: 75 `program-event` (yeni düzeyde geriye dönük tetik yok, `since_t`
sonrası kapanan barda `condition_met`). Özet tablo satırı (SINANAMADI 1) bu ekle kapanır.
