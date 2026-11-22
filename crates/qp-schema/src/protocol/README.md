# protocol/

`qp/0` alan protokolünün mesaj tipleri, mesaj ailesine göre (zarf, işlem, değişiklik, sorgu,
cevap, bildirim, girdi, tekrar oynatma, izleyici, kapsül, ajan, seçici, tutanak, hikâye).
Dışa açık yollar `qp_schema::protocol::X` olarak `protocol.rs`'teki `pub use` ile korunur.
Yalnız tip tanımı ve şema; mesajları işleyen otorite mantığı burada değildir.
