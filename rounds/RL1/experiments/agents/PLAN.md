# RL1 ajan teyidi — koşu planı (U2)

GOAL §7 "Ajan yazabilirlik teyidi", kabul ölçütü 16, VERIFY 18. Seçim değil, gerçek
uygulamanın sınaması; **bir kez** koşulur. Bu plan ve eşikler (§6) ölçümden önce yazıldı.
Düzenek: `SELECTION.md` (20 görev), `PREAMBLE.md` (ölçülen ajana ön not), `LANG-CARD.md`
(ajanın tek belgesi), `qmcpx` (ajanın komutu), `driver.py`, `score.py`, `selftest.py`.

## 1. Bağlantı biçimi (R5B başvuru ajanlarından)

`agents/run.py` ile aynı: koşu başına taze `qp mcp serve --http --listen 127.0.0.1` (yüzey
`thin`, ilke `claude`: ajan, öneren, Arkın adına), `--log` ve `--dump`. Alt ajan sunucuya
**MCP araçlarıyla değil, Bash'ten** istemdeki komutla bağlanır:
`env QMCP_URL=… QMCP_PRINCIPAL=claude QMCP_RUN=<koşu> QMCP_STATE=<koşu>/mcp.sid QMCPX_DIR=<koşu> python3 …/qmcpx`.
`qmcpx`, R5B `rounds/R5B/experiments/harness/driver/qmcp` istemcisini yükler (aynı Streamable
HTTP, `X-Qp-Principal`/`X-Qp-Run`, oturum kimliği dosyası) ve üç şey ekler: her çağrının
**tam** argümanı ve cevabı `transcript.jsonl`'a; `--source-file` (program metnini JSON'a gömme
derdi olmasın: yerli MCP istemcisinde zaten yoktur); `submit` (yerel sayaç, en çok 3; teslimi
`check_program` ile denetleyip kaydeder). Neden yerel `submit`: PS §3.3'te sunucu aracı olarak
teslim yok ve `add_program` sinyal/durum programını `E_TRY_ONLY` ile reddeder (G7, G8). Seri/olay
görevlerinde ajan son tesliminden sonra `add_program` ile öneri de verir (PREAMBLE); karar aracı
yok, kararı insan verir.

## 2. Ön koşullar (koşudan önce, sırayla; biri tutmazsa dur ve REPORT'a yaz)

1. MCP araçları ve komut satırı var: `qp mcp serve` `thin` yüzeyinde `check_program`,
   `try_program`, `explain_program`, `add_program`; `qp program check|run` (PS §3.3).
2. RX4 uygunluğu yeşil: `python3 -I rounds/RL1/experiments/rx4/conform.py --run "<şablon>"`
   26/26 + 4 ret eşit (motor çıktısı biçimi `conform.adapt_outputs`'a uyuyor demektir; puanlayıcı
   aynı adaptörü kullanır).
3. Arayüz teyidi (bir deneme koşusu, ölçüme sayılmaz, sonra silinir):
   ```sh
   python3 -I rounds/RL1/experiments/agents/driver.py start probe G1-01 --tag probe
   # istem dosyasındaki komutla:
   <komut> list                     # dört program aracı görünmeli
   <komut> schema check_program     # alan adları: source | program, inputs, params?, lang?
   ```
   `inputs` biçimi `["rx4"]` değilse (ör. dizgi ya da `{series}`) yalnız `QMCPX_INPUTS`
   (driver `qmcpx_line`), `PREAMBLE.md` ve `LANG-CARD.md` §7 düzeltilir; düzeltme REPORT'a.
4. **Kart doğrulaması:** `LANG-CARD.md` §6 ve §8'deki her örnek `check_program`'dan `ok: true`
   (ret `reject` ile) dönmeli; dönmüyorsa kart ya da uygulama yanlıştır → önce o düzeltilir
   (şartname geçerli). `add_program` ret metniyle ne döndürüyor ve sinyalde `E_TRY_ONLY` mi,
   bakılır (puanlama ikisini de cevaptan okur).
5. Oturum yükleniyor: `driver.py start` `server exited` vermemeli (init: belge + ilkeler,
   `--init` ile sunucu ajan ilkesi eklemez; init kendisi ekler). Deneme koşusu:
   `driver.py stop probe-G1-01-probe --no-engine` ve klasörü sil.
6. `python3 -I rounds/RL1/experiments/agents/selftest.py` → `selftest: OK`.

## 3. Ortam

```sh
cd /Users/arkns/Works/Mchart/qapsule
export CARGO_TARGET_DIR=/Users/arkns/Works/Mchart/qapsule/target
cargo build -p qp-cli --release                 # QP_BIN = $CARGO_TARGET_DIR/release/qp
export QP_RL1_AGENTS=<oturum scratchpad'i>/rl1-agents    # ham koşular: depoya girmez
A=rounds/RL1/experiments/agents
```

## 4. Koşu (2 model × 20 görev = 40 alt ajan oturumu; tek görev = tek oturum, RX4 gibi)

Modeller: küçük **`haiku`**, büyük **`opus`** (RX4 de opus'la ölçtü; karşılaştırma aynı boyla).
Model kimliği dökümden `meta.model_ids`'e yazılır (RX4: `claude-haiku-5-5`, `claude-opus-5-5`).
Aynı anda en çok 5 oturum (her biri kendi sunucusu ve portu). Her görev için:

```sh
python3 -I $A/driver.py start <haiku|opus> <görev>
#  -> {"run","workdir","prompt_file","url","pid"}
```

Orkestratör bir Claude Code alt ajanı başlatır: `Agent(subagent_type="general-purpose",
model=<haiku|opus>, description="RL1 ajan teyidi <görev>", prompt="<prompt_file içeriği>")`
(istem dosyasının içeriği aynen; başka bağlam verilmez). Alt ajan `DONE` deyince ya da hata
verince:

```sh
python3 -I $A/driver.py stop <model>-<görev> --tokens <bildirim total_tokens> \
        --tool-uses <bildirim tool_uses> --ms <bildirim duration_ms>
```

`stop` sunucuyu kimliğiyle kapatır (K92), alt ajan dökümünü bulur
(`~/.claude/projects/*/*/subagents/agent-*.jsonl`, istem yolunu anan en yenisi; bulunamazsa
`--transcript <yol>` ile verilir) ve **belirteci RX4 tanımıyla** dökümden hesaplar (istek
başına girdi + önbellek okuma + önbellek yazma + çıktı, kümülatif, ileti kimliği başına bir
kez; RX4 `COMPUTE-LANGUAGE.md` §7). Bildirimdeki sayı `tokens_notification` olarak ayrıca kalır
(RX4 REPORT §8: o alan kümülatif değildir, karşılaştırmaya girmez). Yalıtım taraması
(`violations`: kart, istem ve koşu klasörü dışı okuma; Bash'te depo/`refs`/`docs/`/`cargo`/`git`)
elle okunur; ihlalli oturum REPORT'ta adıyla işaretlenir, yeniden koşulmaz. Sonra `engine`
adımı: her teslim `qp program check --json` ve iki veri kümesinde `qp program run --json --full
--data` ile yeniden koşulur (`engine/`).

Teslimsiz biten ya da çöken oturum yeniden başlatılmaz (bir kez kuralı); yalnız altyapı
hatası (sunucu açılmadı, eşzamanlılık sınırı) olursa aynı istemle bir kez daha ve REPORT'a.

## 5. Puanlama ve RX4 karşılaştırması

```sh
python3 -I $A/score.py $QP_RL1_AGENTS --models haiku,opus      # -> $A/RESULT.md, RESULT.json
```

`RESULT.md` RX4 ile yan yana tabloyu, hükmü (§6), görev tablosunu ve yanlış teslimlerin
ayrıntısını verir; REPORT.md'ye VERIFY 18 için o tablo girer. Ham koşular depoya girmez; kanıt
olarak yalnız `RESULT.md/json` ve gerekirse küçültülmüş transkript örnekleri
`rounds/RL1/experiments/agents/` altına (dosya ≤ 1 MB, klasör ≤ 20 MB; K68). Koşu bitince
`$QP_RL1_AGENTS` silinir.

**RX4 taban çizgisi (kol A = metin dil, Qapsule'ün seçtiği yüzey).** Kaynak
`../refs/docs/rounds/RX4/REPORT.md` §1, `../refs/reports/COMPUTE-LANGUAGE.md` §3.1, §3.2, §7;
alt küme sütunları bu 20 görev için `../refs/experiments/rx4/runs/sessions.json`'dan (`total`,
`tool_calls`; anahtarlar `runs/a-haiku/<görev>`, `runs/a-opus/<görev>`):

| Ölçüt | A-haiku (30) | A-haiku (bu 20) | A-opus (30) | A-opus (bu 20) |
|---|---:|---:|---:|---:|
| İlk teslim doğru (M1) | 30/30 | 20/20 | 30/30 | 20/20 |
| Sonunda doğru (M2) | 30/30 | 20/20 | 30/30 | 20/20 |
| Düzelme (M3) | — (ilki yanlış yok) | — | — | — |
| Ret gereken yerde ret (G4-04 + G5) | 4/4 (M6 2/2, M7 3/3) | 4/4 | 4/4 | 4/4 |
| Yanlış ret | 0 | 0 | 0 | 0 |
| Sessiz yanlış (M4) / M5 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| Teslim öncesi başarısız `check` (oturum) | 0 | 0 | 0 | 0 |
| Uydurma işlev adı | ölçülmedi; dolaylı 0 (başarısız `check` 0) | 0 | aynı | 0 |
| Belirteç, görev başı medyan (toplam) | 270 704 | 270 696 | 208 070 | 207 990 |
| Belirteç, hücre toplamı | 8 646 697 | 5 695 376 | 5 889 370 | 3 815 421 |
| Araç çağrısı, ortalama | 7,53 | 7,45 | 5,17 | 5,15 |
| Çıktı belirteci, medyan | 1 712 | 1 607 | 688 | 678 |

Bilgi için kol B (JSON/şema): B-haiku tek başarısız `check` oturumu G9-02 (`E_UNKNOWN_REF`),
medyan 321 201 / 273 803 belirteç. Taban (alt ajan sistem istemi + araç tanımları) RX4'te ≈ 37,4
bin; burada da Claude Code alt ajanı, aynı düzen.

**Karşılaştırmayı bozan bilinen farklar** (sonuç okunurken akılda; REPORT'a yazılır):
araçlar CLI değil MCP (Bash köprüsüyle); belge RX4 `SPEC.md` + `DOC.md` yerine tek `LANG-CARD.md`;
ajan veri dosyasını okumaz (`try_program` özeti var); seri/olay görevlerinde fazladan bir
`add_program` çağrısı; çıktı `fingerprint` → `state`, Qapsule hata kodları ve sınırları.
RX4'te ölçüm tavandaydı (180/180 ilk teslim doğru): burada 20/20'nin altı tek görevde bile
nedeniyle açıklanır.

## 6. "Belirgin altında" eşikleri (önceden yazılı; `score.verdict`)

Model başına, aşağıdakilerden **biri** tutarsa "RX4'ün belirgin altında" denir ve neden aranır:

1. Sonunda doğru ≤ 18/20 (RX4 20/20'den en az 2 görev eksik), ya da ilk denemede doğru ≤ 17/20.
2. Sessiz yanlış teslim > 0.
3. Ret gereken yerde son teslim < 4/4, ya da program görevinde yanlış ret > 0.
4. Uydurma işlev adı (`E_UNKNOWN_OP`) ≥ 2 görevde.

Belirteç medyanı > 1,5 × RX4 A (aynı model, bu 20 görev) doğruluk hükmüne girmez, maliyet
notu olarak yazılır.

**Neden arama yöntemi (yüzey yeniden seçilmez, GOAL §7):** her yanlış görev için transkript
(`transcript.jsonl`) ve motor cevapları okunur ve tek bir nedene bağlanır: (a) kart eksik ya
da yanlış (şartnamede var, kartta yok/çelişik), (b) hata iletisi yetersiz (`did_you_mean`,
`hint`, `span` yol göstermedi), (c) uygulama şartnameden sapıyor (RX4 uygunluk tablosuyla
çapraz bakılır), (d) MCP/araç sürtünmesi (şema, `inputs`, köprü), (e) görev cümlesi ile
Qapsule anlamı arasındaki bilinçli sapma (PR §15), (f) model hatası. (a)–(d) düzeltme önerisi
olarak REPORT "Öneriler"e; ölçüm yeniden koşulmaz (bir kez).

## 7. Belirteç ve kullanım kaydı

- Oturum başına `meta.json`: `tokens` (dökümden, RX4 tanımı), `tokens_detail`
  (`in/cache_r/cache_w/out`), `requests`, `tool_uses`, `duration_ms`, `model_ids`,
  `tokens_notification`, `preamble_sha`, `card_sha` (bütün koşularda aynı olmalı).
- Tahmini bütçe: RX4 A medyanlarıyla 20 × 270 bin (haiku) + 20 × 208 bin (opus) ≈ **9,6 M**
  belirteç (büyük kısmı önbellek okuması); kart RX4 belgelerinden kısa, MCP köprüsü ve öneri
  çağrısı uzatır. Orkestratörün ve denetçinin belirteci ayrı satır.
- REPORT "Belirteç ve model kaydı" tablosuna: model başına oturum, toplam, medyan, ortalama
  araç çağrısı.

## 8. Temizlik (K56, K68)

Her `stop` sunucuyu kapatır; koşu bitince `ps` ile `qp mcp serve` kalmadığına sürücünün
yazdığı `pid`'lerle bakılır (adla toplu kapatma yok, K92). `$QP_RL1_AGENTS` (ham koşular,
`heldout.bars.json` kopyaları) silinir; kalan yalnız `RESULT.md/json`.
