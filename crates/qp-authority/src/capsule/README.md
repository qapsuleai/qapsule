# capsule/

Kapsül yaşamı (`capsule.rs` motor bağlama ve koşucu): iddialar, itiraz, yorumlar, izleme
eşleşmesi, hikâye, dışa açık görünüm (`public`), `capsule.rs`'in `impl Session` parçaları
(okur, koşu, `capsule` sorgusunun ayrıntısı `detail`, izdüşüm, oturma; `capsules` ve `ledger`
sorguları S3.2'de `qp-read`'e çıktı) ve oturumun başlık kaynağı (`headsource`). Saf kapsül
dizinleri (`capidx`, `proj`, `sentences`, `headparts`) `qp-plane`'dedir; `*_tests.rs` onları
oturum üstünden sınar.
Kapsül motorunun kendisi değildir; o `qp-capsule` crate'indedir.
