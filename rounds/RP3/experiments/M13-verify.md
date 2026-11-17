# RP3/M13 — bağımsız denetim

Dal `rp3/m13-open` (`4902b8e`), worktree `agent-a6314f87d0793ae11`. Denetçi işi yapan ajan değil.
Dala dokunulmadı.

**Sonuç: PASS** (VERIFY satır 11: sıkıştırılmış paket var, ölçüm önce/sonra tablosuyla uyumlu)

| # | Denetim | Kanıt |
|---|---|---|
| 1 | Kapsam ve hijyen | `git diff 6877bda..rp3/m13-open --stat`: 9 dosya. Yalnız `crates/qp-cli` (`demo.rs` içinde modül belgesi ve `GET /<dosya>` kolu; `demo_static.rs`, `main.rs` içinde `mod`, testler), `scripts/demo.sh`, `shells/web/index.html` (ön yükleme satırı), `shells/web/tests/{compress,startup}.mjs`, not. Hesap kodu, şema, altın görsel, transkript değişmiyor. Gizli bilgi yok. Host/Origin `Gate` kodu değişmemiş. `node shells/web/arch-check.mjs`: 0 ihlal, öz-sınama 14/14. |
| 2 | fmt, clippy, test | `cargo fmt --all -- --check` temiz. `cargo clippy -p qp-cli --all-targets -- -D warnings` uyarısız. `cargo test -p qp-cli`: bütün ikililer `ok` (birim 1, cli 11, demo_security 1, demo_static 1, explain 4, mcp 2, protocol 5, replay 1, views 5). |
| 3a | Paket ve compress.mjs | `QP_DEMO_PACK_ONLY=1 scripts/demo.sh`: `.gz` ve `.br` üretildi, `sizes.json` içinde `wasm_br` var. `compress.mjs` bağlantı noktası 8505'e çevrilmiş bir kopyayla koşturuldu (yalnız port ve import yolları farklı): `chromium: wasm br`, `reload … 0 B body`, `webkit: wasm gzip`, `compress: PASS`. |
| 3b | curl (`qp demo --synthetic --port 8506`) | `Accept-Encoding: gzip` ile `200`, `Content-Type: application/wasm`, `Content-Encoding: gzip`, `Vary`, `ETag`, `Cache-Control: no-cache`. Başlıksız istekte kodlama yok, ham uzunluk. gzip, br ve ham gövdeler açılınca ham WASM ile bayt bayt aynı (`cmp`). Eşleşen `If-None-Match` `304` döndürüyor; başka kodlamanın ETag'i `200`. Yanlış `Host` `403`. Not: HEAD `405` döndürüyor, bu R4'ten beri böyle (yalnız GET). |
| 4 | Tablodan örnek satır | Aynı adsız WASM (4.377.168 B, `main` paketi), `startup.mjs`, 4×, 40/20, DPR 3, 3 koşu medyanı. Chromium soğuk toplam: önce **4.472.351 B** (not 4.472.496), sonra br **935.599 B** (not 935.707). İlk kapsül soğuk: 1.857 → 1.212 ms (not 1.907 → 1.230). Sıcak durumda 2.396 B, WASM 404 B (304). Hepsi %3 içinde. |
| 4b | WebKit'te çift indirme | Önce (`6877bda` kabuğu): **8.850.528 B, 9 istek** (WASM ×2). Sonra: **1.334.442 B, 8 istek** (gzip). Not 8.850.801 → 1.334.452 B diyor. Doğrulandı. |

Gözlemler (engel değil):
- `compress.mjs` 8476 bağlantı noktasını sabit kullanıyor. `startup.mjs`'teki gibi `QP_PORT` yok.
- `demo.sh` paketi güncel saydığında yeniden derlemiyor. Worktree'deki `pkg` `verify.sh` derlemesinden kalan **adlı** paketti (5,6 MB; br 1.066.361 B). Gösterim bu durumda adlı paketi sıkıştırıp sunar. Davranış R4'ten geliyor, M13 kaynaklı değil.
- Chromium, 304 yanıtını Playwright'ta `200` (gövde 0 B) olarak raporluyor. Test bunu kabul ediyor, doğru.
