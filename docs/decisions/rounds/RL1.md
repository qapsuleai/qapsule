# Karar Günlüğü — Önerilen (RL1) — Arkin'in onayını bekliyor (2026-11-26)

<!-- dizin: DECISIONS.md; RL1 hesap dili round'unun önerdiği satırlar -->

RL1 bu satırları **önerir**; kesinleştirmez. Round içinde Arkin'in verdiği ve yürüten oturumun
kaydettiği kararlar `rounds/RL1/DECISION.md`'de: D1 (program çıktısı hesap serisi, üç şart),
D2 (`unreachable` lint'i ve iki öznitelik), D3 (`restricted_calls`), D4 (İ8 sıkılaştırma),
D5 (`describe` ekleri), D6 (çalıştırıcı kabuk paketinde, İ8 yeniden), D7 (varsayılan WASM modülü için yeni taban). Kanıt `rounds/RL1/REPORT.md`,
`AUDIT.md`, `experiments/`.

| # | Önerilen karar | Kanıt |
|---|----------------|-------|
| Ö127 | **Hesap dili `qp-ir/0`** (`docs/PROGRAMS.md`): tek tipli ara gösterim (43 düğüm, 4 çıktı türü, parametreler), günlüğe yalnız ara gösterim, program kimliği kanonik biçimin SHA-256'sı, kimlik kararlılık kuralları (PR §2). Yeni düğüm ya da alan yalnız ekleme ve var olan kimlikleri değiştirmeden. | PROGRAMS.md; I1, LA, LB denetimleri. |
| Ö128 | **Crate'ler:** `qp-lang` (denetim, maliyet, çalıştırıcı; alan katmanı), `qp-lang-text` (metin dil; yalnız `qp-schema`'ya bağlı), `qp-program` (oturum alt sistemi; bileşime bir satır). | `arch-config.json`; README'ler. |
| Ö129 | **Program çıktısı dokümanda hesap serisidir** (D1): `SeriesSource::Study.program` isteğe bağlı alanı; olay = kapanmış bar serisi + var olan `threshold` koşulu; K13 korunur (canlı bar `null`, kapanmış uzunluk ayrı). Sinyal ve durum tanımı dokümana girmez (yalnız denetle/dene/yeniden oynatma). | PROGRAMS-SESSION §1; PA denetimi; döküm 75. |
| Ö130 | **Hata modeli:** Qapsule'ün kapalı kodları + RX4 alt kodu (`reason`), bütün hatalar birden (`ProtocolError.errors`), konum (`at`, metinde `span`), en yakın ad; uyarılar `W_…`. | PROGRAMS §11. |
| Ö131 | **Bütçe işi sayar** ve sınırlar kayıt defterinde (21 `program.*` anahtarı): statik maliyet önce; çalışma bütçesi (bar × ağırlık + kayıt + açık kayıt + okuma); kişi başı doküman içi sınırlar `Check`'te, kabulde ve geri almada. | PARAMETERS.md; U3 (115 deneme). |
| Ö132 | **Ajan programı öneridir:** `add_program` / `set_program_params` ajanda yalnız öneri, `remove_program` ajana kapalı, MCP'de karar aracı yok; sahip, önerenin adına çalıştığı kişi. | PB denetimi; döküm 76. |
| Ö133 | **Üst periyot seansa hizalı** (`qp-calendar`), takvimsiz ya da 7/24'te UTC; üst periyot yalnız kapanmış bardan. | LB denetimi (XIST yarım gün, tatil). |
| Ö134 | **Kabuk paketinde `wasm-opt`** (`-O1` + işlev ayıklama/sıralama) ve İ8 = ölçülen + %2,5 (D4, D6: 7.332.886 / 2.128.236); varsayılan modül yeni taban 2.602.684 / 817.223, ±%2 (D7). | Ö1 RESULT; PA kapısı. |
| Ö135 | **İzleyici yöntemleri** `qp-replica` `Follower` tipinde; `arch-check` `calls.restricted` (D3). | Ö2 denetimi. |
| Ö136 | **Lint'ler** `unimplemented`, `unreachable` açık, mevcutlar öğe başına izinle, mandallı (D2); `config-diff`'te atıflı artış yayılma kuralını bozmaz. | Ö3 denetimi. |
| Ö137 | **Derleme klasörü iş başına** (`target/wt/<iş>`, K56'nın güncellemesi): worktree'ler arasında paylaşılan klasör aynı crate'in eserlerini karıştırıp yanlış derleme sonucu veriyor (bu round'da iki kez ölçüldü). | `rounds/RL1/AGENT-RULES.md`; I1 raporu. |

## Açık (Arkin'e)

| # | Soru | Öneri |
|---|------|-------|
| A74 | Eksik kancalar (D1 şart 3; `rounds/RL1/experiments/hooks/HOOKS.md`): `ValueSource`, bar başı olay değerlendirmesi, `param_for` + `Ctx` uzantısı, kamu dünyasında `Check`. `qp-authority` İ3 tavanında (12.744); her net satır karar ister. | R7'den önce: `ValueSource` sözleşmeden çıksın; bar başı olay `qp-program` `AfterData` yoluyla (0 çekirdek satırı); `param_for` + `Ctx` uzantısı (≈ +21, bayt eşit) RS1'den önce. |
| A75 | Program parametre taraması API'si (`qp_lang::sweep`, K17 içinde: yerel 12–24 ms, WASM 17–30 ms) henüz yüzeyde yok. | `qp compute --sweep`'e program kipi ve `sweep_program` sorgusu; R7 ya da RS1. |
| A76 | `qp serve`'de `agent_policy` verilmeden hiçbir bağlantı ajan sayılmıyor (R5B tasarımı); program kuralları MCP'de ve `agent_policy` ile tutuyor. | Ajan kuralı `qp serve`'de de varsayılan olsun ya da açıkça belgelensin. |
| A77 | Kesilen: iç içe gösterge adresi (K63) programa çevirisi. | R7'nin ilk işlerinden; dil zaten ifade ediyor, eksik yalnız adres ayrıştırıcısı. |
