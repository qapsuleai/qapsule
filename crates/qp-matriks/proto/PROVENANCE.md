# Matriks proto köken kaydı (K62)

Bu klasördeki `.proto` dosyaları Matriks'in mesaj tanımlarıdır. **Elle değiştirilmez.**
Örnek bağlayıcıdaki klasörün tamamı bayt bayt (`cp`) kopyalandı; hiçbir alan eklenmedi,
çıkarılmadı, biçim düzeltilmedi. Rust tipleri derleme sırasında bu dosyalardan üretilir
(`crates/qp-matriks/build.rs`); paket adı ya da çakışma gibi sorunlar build.rs içinde
çözülür, protoya dokunulmaz.

| Bilgi | Değer |
|---|---|
| Kaynak depo | `quantex` (yerel, salt okunur) |
| Kaynak yol (depo kökünden) | `services/quantex-stream/matriks-adapter/src/proto` |
| `quantex` HEAD | `f3d95aeaa5b8fc4858b3160acf882055fbf47f02` |
| Klasörü son değiştiren commit | `b4de978137caa53bd10306c8658f2034f7dbdf16` (2026-03-23) |
| Kopyalama tarihi | 2026-11-19 |
| Yöntem | cp (byte-for-byte, no edits) |
| Dosya sayısı | 22 |

Makinece okunur kayıt: `provenance.json`. `tests/provenance.rs` her dosyanın SHA-256
özetini ve bayt boyunu bu kayıtla, klasördeki `.proto` kümesini kayıttaki kümeyle
karşılaştırır. Yerelde kaynak depo varsa (`QUANTEX_ROOT` ya da `~/LocalGit/quantex`)
kaynak dosyalarla da karşılaştırır; yoksa testi atladığını yazar.

`.gitattributes` bu dosyaları metin dönüşümünden korur (`-text`), özetler her
platformda aynı kalır.

Güncelleme: Matriks protoları değişirse klasör yeniden **tamamen** kopyalanır,
`provenance.json` ve bu dosya yeni commit ve özetlerle yeniden yazılır; kopya elle düzeltilmez.

| Dosya | Bayt | SHA-256 |
|---|---|---|
| `ComputedValues.proto` | 700 | `399e4ae165dd20980a544dce56e5ff035dd210337c28b932e5864fbec313902b` |
| `DepthRow.proto` | 482 | `531b621d799859bc0f55ae3bfdd18ce2efcd8b25904eaead6a8e0c40d161214e` |
| `DepthStats.proto` | 316 | `5723c030b56b7ef5bef866ab09eb61a1bca45697d8fa5beddace0a04be5d625d` |
| `DepthTable.proto` | 824 | `a088e77afab15eef7bfb718746c3590c62d6c2fdac26c61770f5a712828aee43` |
| `Derivative.proto` | 4648 | `f2664cd3531392283666cbfa036fa6da50d0a6cb92a8377f6e074a8211cc248d` |
| `Event.proto` | 311 | `dd6c74a4b9b57b94498154f83d9cff98cfaa1e8d08a6f9c785d6cc7eebe8f4d6` |
| `Fundratio.proto` | 294 | `a5fe48db5d173307d5c085e81220f23ee9c85959a0bfdc154d4119e6b8bd2733` |
| `Messenger.proto` | 484 | `4a8bb0631c48a122f12bded8cfb922312e87ccd8cedc5efaba07117f9f310599` |
| `Meta.proto` | 3285 | `d13c1a64347c8ea59ab9c49a1d6a98687935bb25dcf0b8e39416943456f2c162` |
| `News.proto` | 1002 | `f51212b0ad6f4185d5a8b1f5265b1a752daf3a8eaedb6799b4d31a1c20ada8fd` |
| `Pgc.proto` | 310 | `7b9b28ff5d0ff2d9ac4bf9297ec8c2c10e65a73ee9653959c48ab91845e46092` |
| `QStatus.proto` | 348 | `88717a22377e3b8622239c95d9545796cd325cc69b42c44f0c399e746d8c3a4a` |
| `RankedSymbols.proto` | 317 | `d8cc8f7447a19f51dbf3f2c9711dfb43356dfc14a9e22c3c52817ee6b0be45fc` |
| `Ranker.proto` | 508 | `7c134e299706acf07aa3653d3c2698768e84f32fab9ac0d97d66a22fe7bb9820` |
| `RatingScore.proto` | 460 | `d8f5ec93c3801d6fa1a8463beed87ab505d91d38defa5709ff8d5ee78ed26ff4` |
| `Session.proto` | 565 | `79c7ec1af0a46382cb1f9a731732b3bf010b728e53205ee09785f0986e4c9480` |
| `Symbol.proto` | 6201 | `77d8bf6c9bba1bed00e5336e7b412e1293913e8c951ab6b2f24636158467de9f` |
| `Timestamp.proto` | 391 | `5987073dd9e6579e465b203f6afc3c1708705777796b3066e468ef526b55ca4b` |
| `TopOfTheBook.proto` | 661 | `768aee78ffddb8063fc938cd39bf9d3620aad5be4716d2f3ded6bcb94d21f767` |
| `Trade.proto` | 502 | `b89799c4f8d3257c49e57dc51985d17ea8d5e0995f3ab9afb3a275e8047a20d9` |
| `Tradedate.proto` | 279 | `cf9c5df6994aa6b45f3e6226f8d781fe28b427bbe96ea4b9233b461061e05005` |
| `ViopAgentList.proto` | 286 | `cf451f1f88b7b164fce4f35daac96c05d095195f1019ff2545d734f2e33aab8b` |
