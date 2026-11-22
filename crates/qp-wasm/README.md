# qp-wasm

**Web bağlaması.** İnce bir `wasm_bindgen` sarmalayıcısı: her işlev yerel CLI'nin ve
testlerin kullandığı aynı çekirdek kodu çağırır. R0 görünümü hesap çekirdeği → gramer
derleyicisi → başvuru çizicisi zinciriyle kurulur; `replica` özelliğiyle oturum yansısı
(`Replica`), kabuk (`Shell`) ve galeri (`Gallery`) bağlamaları eklenir. Tarayıcı kabuğu
(`shells/web/`) bu paketi yükler.

**Katman:** kabuklar (`docs/ENGINEERING.md` §2; kabuk bütçesi 4 bin satır).

**Bağımlılıklar:** `qp-schema`, `qp-compute`, `qp-grammar`, `qp-render`, `qp-source`,
`serde_json`, `wasm-bindgen` (`=0.2.130`); `replica` özelliğiyle `qp-session`,
`qp-shellcore`, `qp-capsule`; `spike-vello-gpu` ile `wasm-bindgen-futures`.
Özellikler: `replica`, `simd`, `spike-vello-cpu`, `spike-vello-gpu`.

**Dışa açık yüzey (`lib.rs`):** `compute`, `compute_digest`, `sweep_bench`,
`sweep_digest`; `render_png`, `render_rgba`, `render_stress_rgba`, `render_synthetic_rgba`;
`Replica`, `Shell`, `Gallery` (`replica`); `qp-shellcore`'dan eski yollarıyla yeniden dışa
aktarılan `shell`, `viewer`, `gallery`, `drive` (yalnız yerel).

**Düzen (`src/`):** `bind/` — bağlamalar konu başına bir dosya (`compute`, `render`,
`spike`, `shell`, `gallery`, `replica`); `lib.rs` yalnız bildirir ve dışa açar.
`tests/` kabuk çekirdeği, yansı ve tasarım testleri (çoğu round adıyla); `bench/`,
`examples/` ölçüm sürücüleri.

**Ne değildir:** çizim, yerleşim, hesap ya da kabuk kararı içermez; kabuk çekirdeği
(`ShellCore`) RG1/M1a'dan beri `qp-shellcore`'dadır. Otorite değildir.

**Bilinen borç:** `tests/` klasöründe 20 dosya (klasör bütçesi 15; `dir.files`).
