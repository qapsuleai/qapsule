# authority/session/

`Session`'ın `impl` blokları konuya göre: kurma, giriş sınırı, yönlendirme, `command`,
iki dünya, kabul ve işleme, oturma noktası, olaylar, saat ve varlık (abonelik, bakış);
`phases.rs` alt sistem sözleşmesinin otorite tarafı (RF1 S3.0: `SessionView`, kayıt, fazların
toplama ve yazma adımları, önce kayda bakan dağıtıcılar); `files.rs` barındırıcının dosya
yardımcıları (`confine`, `read_json`).
Tip ve alanları `session.rs`'tedir; başka alt sistemlerin `impl Session` blokları burada değildir.
