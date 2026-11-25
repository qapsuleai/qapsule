# authority/session/

`Session`'ın `impl` blokları konuya göre: kurma, giriş sınırı, yönlendirme, `command`,
iki dünya, kabul ve işleme, oturma noktası, olaylar, saat ve varlık (abonelik, bakış);
`phases.rs` alt sistem sözleşmesinin otorite tarafı (RF1 S3.0: `SessionView`, kayıt, fazların
toplama ve yazma adımları, önce kayda bakan dağıtıcılar); `push.rs` kayıtlı konulara abonelik
ve itilen konular (RF1 S3.3: görüntüleyici alt sistemi `qp-viewer` her girdiden sonra okuyucunun
dünyasında sorulur; `viewer_flush`, `viewer_data`, `visible` yönlendirmesi); `files.rs` barındırıcının dosya
yardımcıları (`confine`, `read_json`).
Tip ve alanları `session.rs`'tedir; başka alt sistemlerin `impl Session` blokları burada değildir.
