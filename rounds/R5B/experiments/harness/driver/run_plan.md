# R5B ölçüm koşu planı (H)

Bu plan alternatifler yazılmadan önce yazıldı (K39). Sözleşme `rounds/R5B/PLAN.md` §5–§6;
düzenek `../SPEC.md`. Komutlardaki `qp mcp serve` bayrakları PLAN §5'tekilerdir; M alt
görevi farklı adlandırırsa yalnız bu dosyadaki komut satırları güncellenir, puanlama değişmez.

## 1. Değişkenler

```sh
H=rounds/R5B/experiments/harness
QP=$CARGO_TARGET_DIR/release/qp                       # cargo build -p qp-cli --release
SP=<oturumun scratchpad'i>                            # /private/tmp/claude-501/…/scratchpad
OUT=$SP/agents                                        # ham koşular; depoya girmez
```

## 2. Sunucu (koşu grubu başına taze)

Her ajan grubu **kendi taze sunucusunu** alır (durum yazan görevler birbirini görmesin;
başlangıç her seferinde aynı, `../SPEC.md` §2):

```sh
PORT=<boş port>
$QP mcp serve --http --port $PORT \
    --init $H/session/init.r5b.json --root $H/session --source synthetic \
    --surface <thin|task|query|mixed> [--capsule-format <json|card|layered>] \
    --log $OUT/<koşu>/server.log.jsonl \
    --dump $OUT/<koşu>/state/document.json
```

- Kimlik HTTP'de istek başına (`X-Qp-Principal: claude | claude-guest`); iki ajan kimliği
  `init.r5b.json`'da (`on_behalf` arkin / guest).
- `--dump` (düzeneğin M'den isteği, ürün değil): sunucu kapanırken (SIGTERM ya da stdin
  EOF) otoritenin dokümanını (`document` sorgusunun `host` gözüyle cevabı) yazar. Yoksa
  bkz. SPEC §9 yedek yolu.
- Oturum duraklatılmış başlar (`running: false`); ölçüm boyunca zaman ilerlemez. M31
  dışında hiçbir koşu adım atmaz.
- Araç tanımlarının boyu: `QMCP_URL=… python3 $H/driver/qmcp list > $OUT/<koşu>/tools.jsonl`;
  `wc -c` → `meta.json` `tooldefs_bytes`.

## 3. Ajan grupları

Görevler (`../tasks.json`, 45) dört gruba bölünür; her grup bir alt ajandır (11–12 görev).
Bölüm sabittir ve her kategoriden karışıktır (öğrenme etkisi gruplar arasında dengeli):

| Grup | Görevler |
|------|----------|
| g1 | F01 F05 E01 E05 V01 V05 C01 C05 W01 W05 P01 K01 |
| g2 | F02 F06 E02 E06 V02 V06 C02 W02 P02 K02 G01 G05 |
| g3 | F03 F07 E03 V03 C03 W03 P03 K03 G02 G06 |
| g4 | F04 F08 E04 V04 C04 W04 P04 K04 K05 G03 G04 |

Koşu = (yüzey × model × grup). Koşu adı `<yüzey>-<biçim|->-<model>-<grup>` (ör.
`task---haiku-g1`). Sıra: önce bir **pilot** (yüzey `thin`, üç model, `g1`): maliyet ve
düzenek doğrulaması; sonra M28 tam matris, sonra M29, sonra gömülü talimatlar ve M31.

| Ölçüm | Matris | Ajan sayısı |
|-------|--------|------------:|
| M28 | 4 yüzey × 3 model × 4 grup (45 görev) | 48 |
| M29 | seçilen yüzey × 3 biçim × 3 model × 2 okuma grubu (r1: F01–F08 E01–E06 G01 G02; r2: V01–V06 C01–C05 G03–G06; 31 görev) | 18 |
| Gömülü talimat | seçilen yüzey × 3 model × 2 grup (I01–I07, I08–I14) | 6 |
| M30 yeniden gönderme | M28'in yorum görevlerinden (K01–K05, `comment_rejected` sonrası) | 0 ek |
| M31 | 3 süzgeç × 1 model (sonnet) | 3 |

Alt ajan Claude Code `Agent` aracıyla başlatılır: `model: haiku | sonnet | opus`; istem
`prompt_template.md` (yer tutucular dolu); ajanın yazma klasörü `$OUT/<koşu>/`; ajan yalnız
`qmcp` komutunu çalıştırır (istem kuralı; Claude Code izinleri `Bash(python3 $H/driver/qmcp:*)`
ve kendi klasörüne yazma ile sınırlanır). Ajanlar birbirinin klasörünü görmez.

## 4. Koşu klasörü (puanlayıcının girdisi)

```
$OUT/<koşu>/
  meta.json            {"run","surface","model","format"?,"group","tasks":[…],
                        "tokens","tool_uses","duration_ms","tooldefs_bytes"}
  answers/<ID>.answer.json
  client.log.jsonl     qmcp (QMCP_LOG) satırları
  server.log.jsonl     qp mcp serve --log satırları
  state/document.json  --dump
  tools.jsonl          qmcp list çıktısı
```

`tokens`, `tool_uses`, `duration_ms`: `Agent` aracının sonucundaki kullanım bilgisi
(sürücü elle ya da betikle yazar). `QMCP_RUN=<koşu>/<görev>` görev başına değişir; böylece
günlük satırı göreve bağlanır.

Puanlama:

```sh
python3 $H/score.py tasks $OUT/<yüzey>-* --json $OUT/score-<yüzey>.json
python3 $H/score.py tasks $OUT/{thin,task,query,mixed}-* --select --json $OUT/m28.json
python3 $H/score.py comments $OUT/m30/<sıkılık>.jsonl      # doğrulayıcının kararları
python3 $H/score.py injections $OUT/inj-*
python3 $H/score.py m31 $OUT/m31-<süzgeç>/notifications.jsonl
```

M30 sonuç dosyası: `comments/comments.json`'daki her taslak, her sıkılıkta doğrulayıcıdan
geçirilir (Rust testi ya da `qp` alt komutu; dil modeli yok) ve `{id, strictness,
decision, checks}` satırı yazılır. Taslağın okuru (`principal`) ve `about`'u otoriteye
verilir; kanıt (`Evidence`) otoritenin bu oturumdaki toplamasıdır.

M31: sunucu aynı init ile, bu kez barındırıcı `replay step` ile `m31_steps` (10) gün adım
atar (`--running` değil; adım başına bir gün, `qp mcp serve`'ün barındırıcı denetimi ya da
`qp demo --mcp`). Nöbetçi niyetleri `m31/watches.json`'dakilerdir; çıktısı
`notifications.jsonl` (`{day, capsule, stage, text}`).

## 5. Depoya ne girer

Ham koşular (`$OUT`) depoya girmez. Depoya yalnız özet tablolar (`score.py` çıktısı:
markdown ve küçük JSON) `rounds/R5B/experiments/<ölçüm>/` altına girer (K68: dosya
≤ 1 MB). Ölçüm bitince `$OUT` silinir; raporda koşu sayısı ve toplam belirteç yazılır.

## 6. Bütçe ve durma kuralı

Tahmin `../SPEC.md` §11. Pilotun (3 ajan) gerçek belirteçleriyle tahmin yeniden
hesaplanır; toplam 60 $'ı aşacaksa ölçüm durur ve Arkin'e sorulur (TASKS "Model boyları").
Her koşudan sonra birikimli tahmin `$OUT/cost.tsv`'ye yazılır.
