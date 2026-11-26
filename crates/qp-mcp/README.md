# qp-mcp

**MCP uyarlayıcısı** `qp/0` başvuru otoritesinin üstünde (`rounds/RD1/EMBEDDING.md` §1).
Bir MCP uç noktası bir kişi (principal), bir cihaz yüzeyi ve bir araç yüzeyine bağlıdır;
JSON-RPC 2.0 cevaplar (`initialize`, `tools/list`, `tools/call`, `resources/*`,
`subscriptions/listen`…). İnce yüzey protokole durum eklemez: her araç çağrısı tam bir
`qp/0` iletisi olur ve `qp_session::Session::handle_raw`'a gider.

**Katman:** uyarlayıcılar (`docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-schema`, `qp-render` (`render` aracı için), `qp-session`,
`qp-lang-text` (RL1 `program_add`: metin → ara gösterim),
`schemars`, `serde`, `serde_json`. Geliştirme: `qp-schema` (`validate`).

**Dışa açık yüzey (`lib.rs`):**

- `Hub`, `SharedHub`, `Queued` — birçok uyarlayıcının paylaştığı tek otorite ve bağlantı
  başına bekleyen `notify` kuyruğu.
- `McpServer`, `Config`, `Options` — uç nokta ve kimliği.
- `Inbox`, `Delivery`, `NotifyMode`, `NOTIFY_URI` — bildirim teslimi.
- `RpcError`, `tool_result`, `notification`, sürüm ve `_meta` sabitleri.
- `ToolSurface`, `ToolDef`, `Ctx`; modüller `surfaces`, `tools` (protokol şemasından
  üretilen araç listesi), `server_tools` (`render`, `notifications`), `calllog`, `conform`.

**Düzen (`src/`):** `surfaces/` — araç yüzeyleri, her biri bir dosya (`thin`, `task`,
`query`, `mixed`; büyüyenlerin parçaları yanlarındaki klasörde), kayıt `registry.rs`
(ayrıntı `src/surfaces/README.md`). Geri kalanı kökte konu başına bir dosya.

**Ne değildir:** otorite değildir; yetki, lisans ve köken kurallarını oturum uygular,
yüzey oturuma doğrudan yazmaz. Süreç tarafı (stdio/HTTP sunucusu, `qp mcp serve`) bu
crate'te değil, barındırmadadır (`qp-cli`; RG1'de `qp-host`'a taşınıyor).
