# Araç yüzeyleri (R5B, M28)

`qp mcp serve --surface <ad>` ajana hangi araçların sunulacağını seçer. Her yüzey bu
klasörde **tek dosya** ve `mod.rs` içindeki `registry()` listesinde **tek satırdır**:

```rust
// surfaces/mod.rs
pub mod task;                                   // 1) modül
&[("thin", thin::new), ("task", task::new)]     // 2) registry() satırı
```

## Arayüz

```rust
pub trait ToolSurface {
    fn name(&self) -> &'static str;
    fn tools(&self) -> Vec<ToolDef>;            // sabit sıra (istemci önbelleği)
    fn call(&mut self, ctx: &mut Ctx<'_>, name: &str, args: Value)
        -> Result<Value, RpcError>;             // MCP CallToolResult
    fn instructions(&self, principal: &str) -> String { /* varsayılan */ }
}
pub type Factory = fn() -> Box<dyn ToolSurface>;
```

- `ToolDef { name, description, input_schema, annotations, output_schema }`.
  `annotations`: `Annotations::READ` (sorgu: `readOnlyHint: true`) ya da
  `Annotations::WRITE`; `destructiveHint` her zaman `false`. Ek açıklamalar yalnız ipucudur;
  yetkiyi otorite denetler.
- **Girdi şeması elle yazılmaz:** `input_schema::<T>()` (`T: schemars::JsonSchema`; bir
  `qp-schema` tipi ya da küçük bir argüman yapısı) ya da protokol şemasından türetilir
  (`crate::tools`).
- `Ctx` yüzeyin tek kapısıdır: `ctx.send(&msg) -> reply` (bir `qp/0` mesajı, uç noktanın
  kendi bağlantısında), `ctx.query(q)`, `ctx.command(ops)`, `ctx.request_id()`
  (`_meta["dev.qapsule/id"]` ya da `mcp-<n>`), `ctx.take_notifications()` (bekleyen
  `notify`'lar), `ctx.principal()`. Yüzey oturuma doğrudan yazmaz; okuma ve yazma
  otoritenin izin, lisans ve köken kurallarından geçer.
- Protokol hatası bir araç sonucudur (`isError: true`, `crate::tool_result`); `Err` yalnız
  bilinmeyen araç ve bozuk argüman içindir (`-32602`).

## Ortak araçlar

`qp mcp serve` her yüzeyin araçlarından sonra `render` (görünümü PNG olarak çizer, MCP
`image` içeriği) ve `notifications` (bekleyen bildirimler; `qapsule://notifications`
kaynağıyla aynı) araçlarını ekler (`crate::common`). Yüzey aynı adda bir araç tanımlarsa
onunki geçer.

## Bildirimler

Eski MCP sürümleri (`initialize` 2025-11-25/06-18) bildirimleri `notifications/message`
olarak alır (stdio'da yanıttan önce, HTTP'de `GET /mcp` akışında). 2026-07-28 istemcisi
`subscriptions/listen` ile `notifications/resources/updated` (`qapsule://notifications`)
sinyalini alır ve kaynağı okur. Akış açamayan istemci `notifications` aracını kullanır.

## Var olanlar

| Ad | Dosya | Tarif |
|----|-------|-------|
| `thin` | `thin.rs` | Protokolü birebir yansıtan, şemadan üretilmiş araçlar (RD1). Varsayılan. |
| `query` | `query.rs` | (M28 c) İki araç: `ask` (küçük sorgu dili: `find` isim, `where` düz süzgeçler/seçiciler, `select`, `expand`, `limit`, `format` = `card`/`json`/`layered`) ve `act` (`comment`, `watch`, `unwatch`, `propose`, `preview`; karar yok). |
