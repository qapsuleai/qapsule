# RL1 / C — VERIFY 2: `scripts/refactor/compare.sh`, RF1 tutanağına karşı

Koşu: `rl1/close-c` (`f20d1aee`; `main` `64e30256` + C), tam (`--no-*` yok: wasm32, x86_64,
galeri dahil), 2026-11-26. Sonuç: çıkış 1 (beklenen: round eklemesi). RF1 tutanağının 270
kaleminden **245'i bayt eşit**, 25'i farklı, 12 kalem yeni (282). Silinen kalem yok.

Ek nitelik denetimi: round başındaki kod (`08bcb4d6`, RL1'in ilk kaydı, yalnız belge) ayrı bir
worktree'de derlendi; `mcp/serve-thin.jsonl`, `mcp/serve-mixed.jsonl`, `mcp/calls/thin.jsonl`,
`schema/Op`, `schema/Document` RF1 tutanağıyla bayt eşit çıktı (taban doğru). Sonra bu taban ile
yeni çıktı JSON düzeyinde karşılaştırıldı: her farkın yalnız **ekleme** olduğu (yeni anahtar,
dizinin sonuna eklenen öğe, yeni araç; silinen ya da değişen değer yok, `required` değişmedi)
betikle denetlendi. Metin kalemleri tutanağın `text/` kopyasıyla satır satır karşılaştırıldı.

## Farklı kalemler (25) ve neden ekleme oldukları

| Kalem | Fark | Neden ekleme |
|---|---|---|
| `schema/{ChangeOp,ClientMsg,Command,Document,InboundLine,LogEntry,Op,OutboundLine,Query,Series,ServerMsg,SessionInit,Transcript}.schema.json` (13) | `Op.oneOf` sonuna 3 kol (`add_program`, `set_program_params`, `remove_program`), `Query.oneOf` sonuna 3 (`check_program`, `try_program`, `explain_program`), `QueryResult.oneOf` sonuna 3, `SeriesSource` study koluna isteğe bağlı `program`, `ProtocolError.errors` (isteğe bağlı), yeni `$defs` (program tipleri) | PS §3: "şema dışa aktarımına yalnız yeni tipler eklenir"; denetim: 0 silinen/değişen yol |
| `mcp/serve-thin.jsonl` | 10 yeni araç, sonda (`op_add_program`, `op_set_program_params`, `op_remove_program`, `query_check_program`, `query_try_program`, `query_explain_program`, `program_check`, `program_try`, `program_explain`, `program_add`); `command`/`query`/`op_put_series` girdi şemalarında aynı şema eklemeleri (`oneOf` 38→41, 18→21, eski öğeler önek) | Araç tanımları yalnız eklemeyle farklı (VERIFY 2); var olan araçların sırası aynı |
| `mcp/serve-mixed.jsonl` | `query` aracının şemasında `Query.oneOf` 18→21 (önek korunur) | aynı |
| `mcp/calls/thin.jsonl` | `query_describe` cevabında `ops` sonuna 3, `queries` sonuna 3 ad | `i1/describe-diff.md` ile aynı fark |
| `conform/replies/03-errors-self-correction.jsonl` | bilinmeyen işlem hatasının `expected` listesinin sonuna `,"add_program","set_program_params","remove_program"` (+52 B) | `i1/describe-diff.md` (`03`), tek satır |
| `conform/replies/16-screen-describe-presenter.jsonl` | `describe` cevabında `ops` (+52 B) ve `queries` (+48 B) sonuna eklemeler | `i1/describe-diff.md` (`16`), tek satır |
| `conform/{inproc,serve,mcp}.txt` (3) | 70 satır aynı; 71–77 için 7 `PASS` satırı; özet `70 … passed` → `77 … passed` | yeni dökümler (bu görev), var olanların sonucu aynı |
| `cli/qp.txt` | alt komut listesine `program` satırı | yeni komut (`qp program`, PS §3.3) |
| `fixtures/committed.sha256` | 9 yeni satır: `fixtures/compute/digest-program.txt`, `digest-rl1.txt`, `fixtures/protocol/conformance/71…77`; silinen/değişen satır yok | yeni fikstürler yalnız yeni dosya (VERIFY 3) |
| `tests.count`, `tests.names` | 1194 → 1425; tutanağın 1194 adının hepsi duruyor (silinen 0) | yeni sınamalar (`qp-lang`, `qp-lang-text`, `qp-program`, oturum/MCP/CLI program sınamaları) |

## Yeni kalemler (12)

`schema/Program.schema.json`; `cli/qp.program.txt`, `cli/qp.program.check.txt`,
`cli/qp.program.run.txt`, `cli/qp.program.explain.txt`; `conform/replies/71-program-add.jsonl` …
`77-program-replay.jsonl` (7).

## Bayt eşit kalanlar (245)

Altınlar (`fixtures/golden`, görünümler PNG/`explain`/`scene`), hesap çıktıları ve dört hesap
özeti (aarch64, x86_64, wasm32), 01–70 dökümlerinin 03 ve 16 dışındaki `qp serve` cevap akışları,
yansı/görüntüleyici bantları, yeniden oynatma akışı, `task`/`query`/`mixed` yüzeylerinin çağrıları,
kabuksuz sürüş, görünüş kayıtları, tasarım denetimi ve sözleşmesi, galeri.
