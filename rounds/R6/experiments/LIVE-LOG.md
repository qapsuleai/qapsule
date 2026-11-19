# R6 — Canlı deneme günlüğü

Her canlı denemenin başı, sonu, kipi, uç noktası (adı, adresi değil), sembol sayısı ve
sonucu. Veri değil, olay kaydı. Canlı bağlantıyı yalnız orkestratör açar, sırayla;
eşzamanlı deneme yok (kilit dosyası da reddeder).

| # | Başlangıç (TRT) | Bitiş (TRT) | Kip | Uç | Sembol | Ne | Sonuç | Çoklu bağlantı bildirimi |
|---|---|---|---|---|---|---|---|---|
| L1 | 2026-11-19 14:24:59 | 14:27:00 | delayed | market | 3 | İlk bağlantı + ham kayıt (THYAO, GARAN, ASELS + `mx/timestamp`), 120 sn | Bağlandı (oturum 1), kopma yok; 732 ileti; `mx/timestamp` SUBACK 0x80 (ret) | Yok |
| L2a | 2026-11-19 14:27:53 | 14:32:54 | delayed | market | 30 | Evren 30, 5 dk, `mx/messenger` denemesi | Bağlandı, kopma yok; 7.555 ileti (~25/sn); 29/30 sembol kabul, 1 sembol SUBACK 0x80; `mx/messenger` ve `mx/timestamp` 0x80 | Yok |
| L2b | 2026-11-19 14:35:29 | 14:40:29 | delayed | market | 100 | Evren BIST100, 5 dk, `/usr/bin/time -l` | Bağlandı, kopma yok; 100/100 kabul, abonelik ~4 sn; 15.074 ileti (~50/sn); işlemci 0,50 sn, en çok RSS 16,5 MB | Yok |
| L2c | 2026-11-19 14:40:37 | 14:45:37 | delayed | market | 580 | Evren BISTTUM (bütün pay piyasası), 5 dk | Bağlandı, kopma yok; 580/580 kabul; 52.316 ileti (~175/sn); işlemci 0,77 sn, en çok RSS 17,0 MB; abonelik sınırına ulaşılmadı | Yok |
| L2d | 2026-11-19 14:45:52 | 15:17:12 | delayed | market | 580 | Uzun kayıt (M22 verisi), canlı gösterime geçmek için SIGTERM ile durduruldu | Kopma yok (izleyici 30 dk boyunca olay vermedi) | Yok |
| L3 | 2026-11-19 15:17:40 | (sürüyor) | delayed | market | 30 | Canlı gösterim `qp demo --live` (BIST30, 5m, 120 bar REST geçmişi; r6/t7 7a8ba7a) | Bağlandı; ilk saniyede 1 kapsül canlı durum değiştirdi (invalidated) | Yok |
| L3-M23a | 2026-11-19 15:18:44 | 15:19:26 | delayed | market | 30 | Gösterimde bilerek kesme `POST /live/drop?secs=20` | Kapandı → bayat → 20 sn sonra oturum #2; REST onarımı 30 besleme 20,4 sn; bayat toplam 40,6 sn; patch 0, yeni bar 0; bayatken kapsül 0 | Yok |
| L3-M23b | 2026-11-19 15:19:40 | 15:25:33 | delayed | market | 30 | Gösterimde bilerek kesme `POST /live/drop?secs=330` (5 dk barı aşan) | Bayat 350,6 sn; yeniden bağlandı; REST onarımı 20,4 sn: 30 patch + 30 yeni bar; bayatken kapsül 0, bayatken bırakılan olay 2 | Yok |
| L3 | 2026-11-19 15:17:40 | 15:39:20 | delayed | market | 30 | (devam) gösterim; /live/quit ile kapatıldı | 33.129 ileti, 8.078 veri olayı, 120 kesin bar, 44 canlı kapsül olayı, bayatken 0; **15:38:45 planlanmamış kopma** (sunucu kapattı), 2 sn sonra yeniden bağlandı, onarım 52 patch | Yok |
| L4 | 2026-11-19 15:39:50 | 15:58:58 | delayed | market | 100 | Gösterim BIST100 (5m), ölçek M24 | Kopma yok; ~48 ileti/sn; işlemci kararlı ~%82 bir çekirdek (profil: headparts takvim kurulumu), tik→çerçeve p50 84 / p95 151 ms; RSS 173 MB | Yok |
| L4b | 2026-11-19 15:58:55 | 16:05:39 | delayed | market | 100 | Gösterim BIST100, takvim önbellekli ikili (r6/perf 9e90010) | Kopma yok; işlemci kararlı ~%74 (önce %82); tik→çerçeve p50 67 / p95 110 ms | Yok |
| L4c | 2026-11-19 16:05:40 | 16:10:09 | delayed | market | 100 | Gösterim BIST100, görüntüleyici yeniden hesabı ≤4/sn (r6/perf 84a01df) | Kopma yok; ~76 ileti/sn; işlemci kararlı ~%17; tik→çerçeve p50 0,26 / p95 51 ms; RSS 142 MB | Yok |
| L5 | 2026-11-19 16:10:40 | 16:21:57 | delayed | market | 580 | Gösterim BISTTUM (5m), REST geçmişi 6,3 dk | Kopma yok; ~146 ileti/sn; işlemci ~%69; RSS 864 MB; tik→çerçeve p50 159 / p95 840 ms; 1 sembolün geçmişi yok | Yok |
| L6 | 2026-11-19 16:21:50 | 18:35:38 | delayed | market | 100 | Gösterim BIST100 seans kapanışına kadar (ölçüt 3, 8) | Kopma yok (2 sa 13 dk); borsa saati 18:00'da piyasa durumu kapalı, evreler 5→3→7→27→1→26 geldi; ekran "Seans kapalı" | Yok |
| L7 | 2026-11-19 18:41:17 | 18:41:43 | delayed | market | 3 | Seans sonrası eksiz konu denemesi `mx/symbol/<SYM>` (RI1 S6); ilk deneme `--out` göreli yol kusuruyla bağlanmadan çıktı | Bağlandı; 3/3 eksiz konu SUBACK 0x80 (ret), 0 ileti — gecikmeli uçta `@lvl2` gerekli | Yok |
