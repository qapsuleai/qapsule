# livebar/

Canlı çubuk kurucusunun parçaları (`livebar.rs` kök modülü tipleri, ayarları ve `LiveBars`
çekirdeğini taşır): `push.rs` fiyat kartı ve işlem tıklarının yolu (`push`, `push_trade`),
`compare.rs` canlı çubukların REST çubuklarıyla karşılaştırılması (`compare_bars`).
Kaynaktan bağımsızdır; dışa açık yollar `qp_source::…` olarak kalır.
