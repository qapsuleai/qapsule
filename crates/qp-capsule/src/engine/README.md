# engine/

Kapsül motorunun parçaları (`engine.rs` motor tipini, yapılandırmayı ve dışa açık okumaları
taşır): `run.rs` bir barlık yaşam döngüsü adımı ve hikâyenin karneye kapanışı, `index.rs`
karne dizini (noktasal zaman okumaları için anahtar başına sayımlar), `glyph.rs` hikâyenin
glif ölçüleri. Yeni kural ya da strateji burada değil, kendi modülünde (`theme`, `outcome`…).
