# qp-host

**Barındırma.** `qp` komutlarının arkasındaki barındırma kodu (RG1/M1b'de `qp-cli`'den
ayrıldı): canlı barındırıcı (bağlantı, merkez, onarım), Matriks kaydı ve incelemesi, işlem
karşılaştırması, hızlandırılmış benzetim (`qp sim`, `qp demo --sim`), demo HTTP sunucusu
(durağan dosyalar, galeri, kabuksuz sürüş) ve MCP sunucusunun bağlanması (`qp mcp serve`).
Komutların `clap` argüman yapıları da buradadır, çünkü barındırma işlevleri onları alır.

**Katman:** uyarlayıcılar (`docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-shellcore`, `qp-session`, `qp-mcp`, `qp-live`, `qp-matriks`,
`qp-source`, `qp-schema`, `qp-calendar`, `qp-capsule`; `clap`, `flate2`, `prost`, `sha2`,
`serde_json`.

**Dışa açık yüzey:**

- `demo` — `DemoArgs`, `demo` (demo sunucusu), `DriveArgs`, `drive` (kabuksuz sürüş).
- `sim` — `SimArgs`, `sim` (hızlandırılmış benzetim).
- `live` — `LiveCommand`, `run` (`qp live …`), `FeedArgs`, `replay_raw`.
- `mcp_serve` — `ServeArgs`, `serve` (`qp mcp serve`).

**Ne değildir:** komut satırı değildir (üst düzey `Cli`/`Command` ve `qp --version`
`qp-cli`'dedir); yetki (authority) değildir, belgeyi yalnız komut yoluyla değiştirir; kabuk
çekirdeği (`qp-shellcore`) ya da hesap çekirdeği değildir.
