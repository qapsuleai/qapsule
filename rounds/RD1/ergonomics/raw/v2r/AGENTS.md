# v2r — alt ajan kayıtları (yeniden koşu)

Beş taze alt ajan (sonnet, `general-purpose`); yönerge v1/v2 ile birebir aynı (klasör yolu
`ergo/v2r/`). `packet/` = v2 şartnamesi; `run.sh` `QP_ERGO_VARIANT=renamed` ile kimlikleri
yeniden adlandırılmış dokümanı (`harness/renamed/doc.json`) kullanır. İlk v2r koşusu
geçersiz sayıldı (`../v2r-void/VOID.md`).

| Ajan | Görevler | Toplam belirteç | Süre | Araç çağrısı |
|------|----------|----------------:|-----:|-------------:|
| A | G01 G06 G11 G16 | 73 725 | 34 s | 6 |
| B | G02 G07 G12 G17 | 77 818 | 43 s | 6 |
| C | G03 G08 G13 G18 | 77 352 | 52 s | 8 |
| D | G04 G09 G14 G19 | 75 943 | 43 s | 8 |
| E | G05 G10 G15 G20 | 76 127 | 45 s | 9 |

## Alt ajanlara verilen yönerge (birebir; denetim O3)

Her ajana aşağıdaki metin verildi; yalnızca `<TUR>` (`v1`, `v2`, `v2r`) ve `<AJAN>` (`A`–`E`)
değişti. Model `sonnet`, ajan türü `general-purpose`.

```text
You are an AI agent that must operate a financial chart document through its protocol. You know NOTHING about the implementation and must not look for it.

STRICT RULES:
- The ONLY files you may read are in the packet folder `/private/tmp/claude-501/-Users-arkns-Works-Mchart-qapsule/f8b0ed39-4f44-465c-a3ec-15923e37cbac/scratchpad/ergo/<TUR>/packet/`: `PROTOCOL.md` (the specification, in Turkish) and `schema/*.schema.json` (generated JSON Schemas of the messages). Do not read `run.sh`'s target or any other file on the machine; do not search the file system; do not use the web.
- The ONLY command you may run is `<packet>/run.sh <your-file.jsonl>`. It runs your message file against a FRESH copy of the live document (each run starts from the same state; earlier runs leave no trace) and prints the messages the authority sends to your connections.
- Message file format: one JSON object per line: `{"conn": "<a connection id you choose, not starting with zz->", "msg": <protocol message>}`. Start each connection with `hello`.

Your tasks are listed in `/private/tmp/claude-501/-Users-arkns-Works-Mchart-qapsule/f8b0ed39-4f44-465c-a3ec-15923e37cbac/scratchpad/ergo/<TUR>/agent<AJAN>/TASKS.txt`. For EACH task, in its work dir:
1. Read the spec/schema as needed, then write `attempt1.jsonl` with the complete message sequence you believe accomplishes the request, and run it. You may include queries, but remember you cannot see their answers before the run; each run is one shot over the whole file.
2. Look at the output. If the request is not accomplished (an error, or the result is not what was asked), write `attempt2.jsonl` (a complete new sequence, from the fresh state) and run it; at most `attempt3.jsonl`. Stop as soon as you believe it is done. Never edit an attempt file after running it.
3. If the request asks a question, write the answer you would give the user (one or two sentences, in Turkish) to `answer.txt`, based on what the authority returned in your last run.
4. Write `notes.txt`: one line per attempt — what you tried, what went wrong, and anything in the spec that was unclear or misleading.

At the end reply with a short table: task, number of attempts, whether you believe it succeeded, and the main difficulty.
```

**Sınır (denetim O3):** "yalnızca şartname ve şema" kısıtı **yönergeyle** konuldu, teknik olarak
zorlanmadı; `packet/run.sh` gizli düzeneğin (`harness/ergo.py`) mutlak yolunu içeriyor. Kayıtlar
(`notes.txt`, deneme dosyaları) ajanların başka dosya okuduğuna dair bir iz göstermiyor ve
denetçi de bulmadı; ama kayıtlar bunu **kanıtlamaz**. Sonraki ölçümler için öneri: paketi ayrı
bir kullanıcı/kap içinde, düzeneği ikili bir araç arkasında çalıştırmak.
