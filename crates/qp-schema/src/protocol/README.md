# protocol/

`qp/0` alan protokolünün mesaj tipleri, mesaj ailesine göre (zarf, işlem, değişiklik, sorgu,
cevap, belge özeti, bildirim, girdi, tekrar oynatma, izleyici, kapsül, seçici, tel biçimi,
tutanak, hikâye). Ajan yorumları ve öneriler işlemlerle birlikte `ops.rs`'tedir.
Dışa açık yollar `qp_schema::protocol::X` olarak `protocol.rs`'teki `pub use` ile korunur.
Yalnız tip tanımı ve şema; mesajları işleyen otorite mantığı burada değildir.
