# feed/

`LiveFeed` boru hattının parçaları (`feed.rs` kök modülü ayarları, ana tipi ve çekirdeği taşır):
`ingest.rs` saat, ileti dağıtımı ve fiyat kartı yolu; `trades.rs` işlem akışı (kural 9:
izleme, tampon, onarım); `output.rs` kayıt başına çıktı, bağlantı değişikliği ve sayaçlar.
Ağ yok; dışa açık yollar `qp_matriks::feed::…` olarak kalır.
