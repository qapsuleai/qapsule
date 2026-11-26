# exec/

Çalıştırıcı (PR §3.2, §4–§7, §8.3, §9, §10; PS §1). `exec.rs` dışa açık tipleri tutar
(`ExecOptions`, `WorkBudget`, `Stopped`, `ExecError`); bu klasör adımlardır:

- `plan.rs` — çözülmüş programdan plan: düğüm çıktısı başına düz değer yuvası, saate göre
  gruplar (saatsiz bir kez, her üst periyot kendi barı kapanınca, taban her barda), istenen
  çıktıların konisi.
- `node.rs` — bir düğümün bir barı: `eval` + `commit` (canlı bar yalnız `eval`), köken `lo`.
- `kernel.rs` — `qp-compute` çekirdekleri tek arayüzde (`peek1`/`commit1`; `Chain` kuralı
  çağıranda).
- `tf.rs` — hiza kaynağı (`Align`: seans ya da UTC), üst barın kuruluşu ve kapanışı.
- `engine.rs` — akışkan çekirdek (`Executor`: `push`, `update_last`, `push_closed`,
  `close_live`), bütçe.
- `record.rs` — kapanmış barda yayımlananlar: seri, olay (öbek sayıları), sinyal (sonuç
  kuralı), durum anahtarı.
- `cursor.rs` — çıktı imleci (`value_at`, `events_at`, `signals_at`, `state_at`, `lineage`,
  `provenance`), okumalar bütçede; `canonical` (bayt eşitlik ve özet metni).
- `summary.rs` — `try_program` özeti (saf işlev) ve PR §6 biçiminde sinyal kaydı.
- `digest.rs` — üç mimari belirlenimcilik özeti (`fixtures/compute/digest-program.txt`).

Özyineleme yok: düğümler topolojik sırayla döngüde koşar.
