# qp-shellcore

**Kabuk çekirdeği (`ShellCore`).** Gösterim kabuğunun saf Rust çekirdeği (RG1/M1a'da
`qp-wasm`'dan ayrıldı): oturum yansısı (`qp_session::Replica`) üstünde kullanıcının
hareketlerini protokol iletilerine çevirir, kapsül yüzlerinin ve açık kapsülün sahnelerini
derler, boya listelerini (Canvas 2D, GPU) üretir, metinleri ve ölçümleri hesaplar. Tarayıcı
kabuğu yalnız bayt ve piksel taşır; karar burada verilir.

**Katman:** uyarlayıcılar (oturum yansısına bağlıdır; `docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-schema`, `qp-session`, `qp-source`, `qp-calendar`, `qp-capsule`,
`qp-grammar`, `qp-render`, `fontdue`, `serde`, `serde_json`; yalnız `wasm32` hedefinde
`wasm-bindgen` (`=0.2.130`, `performance.now()` saati için).

**Dışa açık yüzey:**

- `shell` — `ShellCore` (kabuk çekirdeği) ve alt modülleri (`agent`, `bench`, `freq`, `live`,
  `motion`, `paint`, `paint_canvas`, `paint_gpu`), saf sahne işlevleri, sabitler.
- `viewer` — cihazın okur olarak çekirdeği (`Viewer`, `Stories`; `docs/PROTOCOL.md` §22).
- `gallery` — karşılaştırma galerisinin çekirdeği (`GalleryCore`) ve yerel boyama.
- `drive` — kabuksuz sürüş ve eşlik denetimi (yalnız yerel; `wasm32`'de derlenmez).

`qp-wasm` bunları eski yollarıyla (`qp_wasm::shell`, `qp_wasm::gallery`, `qp_wasm::viewer`,
`qp_wasm::drive`) yeniden dışa aktarır.

**Ne değildir:** `wasm_bindgen` bağlaması değildir (o `qp-wasm`'dadır); yetki (authority)
değildir, belgeyi yalnız komut yoluyla değiştirir; hesap çekirdeği değildir.
