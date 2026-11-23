# compile/layer/prepare

`prepare`'in adlandırılmış adımları (RF1/G3). `prepare.rs` sırayı tutar: veri kaynağı,
kodlama denetimi (`check_encoding`), işaretin satırları (`mark_rows`), `finish`.
`source.rs` katmanın veri kaynağı ve satırları (bulgu satırları, kapsül metinleri);
`series.rs` `candle`, `line`, `area`/`band`/`bar`; `rule.rs` `rule` (parça, yatay, dikey);
`panel.rs` panele göre konumlu `text` ve `glyph`; `point.rs` `point` ve çubuğa oturan `text`.
Ne değildir: sabit çözümü (`../resolve.rs`), seri ve çift satır toplama (`../rows.rs`),
piksel işi (`draw/`).
