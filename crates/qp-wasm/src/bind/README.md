# bind/

`wasm_bindgen` bağlamaları, konu başına bir dosya: hesap (`compute`), R0 görünümü (`render`),
çizim yığını denemeleri (`spike`, yalnız `spike-*` özellikleriyle), kabuk (`shell`), galeri
(`gallery`) ve yansı (`replica`; son üçü `replica` özelliğiyle). Mantık yoktur: her bağlama
çekirdeği (`qp-compute`, `qp-render`, `qp-shellcore`, `qp-session`) çağırır. `shell` ve
`replica` dosyaları `lib.rs`'teki satır içi modülü olduğu gibi taşır (girinti değişmesin diye).
Dışa açık JS adları ve `qp_wasm::…` yolları `lib.rs`'teki yeniden dışa aktarmalarla aynıdır.
