# RL1 — Alt Görevler

| # | Görev | Yazdığı yer | Bağımlı |
|---|-------|-------------|---------|
| T0 | Ana akış belge değişikliklerini kaydet (kökteki `Prompt.txt` bu projenin değil: commit etme, dokunma); alt ajan durumu rapora | kök | — |
| Ö1 | Kabuk WASM paketi küçültme ölçümü ve uygulaması; İ8 tavanı | `shells/web`, `scripts/` | T0 |
| Ö2 | İzleyici tipi (gizli yazan yöntemler); RF1 tutanağıyla karşılaştırma | `crates/qp-replica`, `qp-authority` (yalnız görünürlük), `qp-session` | T0 |
| Ö3 | Lint'ler; CI tam geçmiş | kök, `.github/` | T0 |
| S | Kavramlar, `docs/PROGRAMS.md`, `PROTOCOL.md` bölümü (koddan önce); iki bağımsız eleştiri (biri R7 ve RS1'in bu dile ne yükleyeceğine göre) | `docs/` | T0 |
| I1 | Ara gösterim tipleri | `crates/qp-schema` | S |
| I2 | Yeni çekirdekler (`highest lowest atr stdev percentrank`) | `crates/qp-compute` | S |
| I3 | Denetim: tip, saat, başvuru, parametre, geleceğe bakma, maliyet; hata modeli | `crates/qp-lang` | I1 |
| I4 | Çalıştırıcı: akışkan, geçici değer, çıktı imleci, köken; üst periyot hizası | `crates/qp-lang` | I1, I2 |
| I5 | Belirlenimcilik, önek kararlılığı, `add_study` eşdeğerliği sınamaları; üç mimari özeti | `crates/qp-lang/tests`, `scripts/` | I3, I4 |
| L | Metin dil: ayrıştırıcı, kanonik yazıcı, ret; gidiş-dönüş sınaması | `crates/qp-lang` (ayrı modül ya da `qp-lang-text`) | I1 |
| B | Maliyet ve bütçe: parametreler kayıt defterinde; işi sayan bütçe | `crates/qp-lang`, `crates/qp-schema` (params) | I3, I4 |
| P1 | `qp-program` alt sistemi: işlemler, politika, `Check` fazı, sorgular | `crates/qp-program`, `qp-session` (kayıt satırı) | I3, I4, B |
| P2 | Seri → `ValueSource`; gramer kaynağı ve lejant | `crates/qp-program`, `crates/qp-grammar` | P1 |
| P3 | Olay → `ConditionKind`; kesin bar, bildirim, kanıt | `crates/qp-program` | P1 |
| P4 | Sinyal ve durum tanımı: denetim, dene, yeniden oynatma | `crates/qp-lang`, `crates/qp-program` | P1 |
| M | MCP araçları, `qp program` komutları, `docs/AGENTS.md`, başvuru ajanı | `crates/qp-mcp`, `qp-host`, `qp-cli`, `agents/`, `docs/` | P1 |
| U1 | RX4 uygunluğu (görev çözümleri → beklenen çıktılar) | `rounds/RL1/experiments/rx4/` | I4, L |
| U2 | Ajan yazabilirlik teyidi (20 görev, iki model) | `rounds/RL1/experiments/agents/` | M |
| U3 | Kötü niyet denemeleri (ayrı denetçi) | `rounds/RL1/experiments/abuse/` | P1, B |
| N | İç içe gösterge adresi *(kesilebilir)* | `crates/qp-lang` | I4 |
| C | Performans (K17, K52, tik maliyeti), WASM, uygunluk dökümleri (yeni dökümler eklenir), CI | `rounds/RL1/experiments/`, `fixtures/protocol` | hepsi |
| D | Bağımsız denetim | `rounds/RL1/AUDIT.md` | C |
| X | `REPORT.md`, `STATUS.md`, `ROADMAP.md`, önerilen kararlar, kapanış temizliği | kök | D |
