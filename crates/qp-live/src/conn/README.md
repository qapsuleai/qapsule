# conn/

`LiveConnection` işçi iş parçacığının parçaları (`conn.rs` kök modülü dışa açık tipleri ve
bağlantı tutamağını taşır): `worker.rs` çalışma döngüsü, geri çekilme, durdurma ve abonelik
komutları; `session.rs` tek bir WebSocket + MQTT oturumu ve tel giriş/çıkışı.
Dışa açık yollar `qp_live::conn::…` olarak kalır.
