# R3 — Ajan yazabilirlik görevleri (kapsül)

R2 düzeneğiyle (`rounds/R2/ergonomics/`) ölçülür: taze ajan, yalnız şartname + gramer +
şema + `run.sh`; kimlikler ajan başına rastgele; doğruluk gizli son durumdan puanlanır.
Gizli kurulum: iki enstrüman (`THYAO`, `ASELS`), her birinde 300 günlük bar; motorun
ürettiği olgular ve kapsül hikâyeleri (en az bir açık, bir `realized`, bir `invalidated`,
bir `expired`, bir lisanslı akış kapsülü); ajan `editor`, `headless`; ayrıca yetkisiz bir
`viewer` (`flow/*` yok).

| # | İstek (ajana verilen) | Doğru sayılan |
|---|------------------------|---------------|
| C01 | Bugün THYAO'da ne var? | `capsules` sorgusu (enstrüman THYAO, açık); cevapta açık kapsüllerin tamamı |
| C02 | THYAO'daki kırılım kapsülünün kanıtını aç. | `capsule` sorgusu, `evidence: true`; kanıt değerleri döndü |
| C03 | ASELS'teki geçersiz olan kapsül neden geçersiz oldu? | `capsule` sorgusu; cevapta `invalidated` geçişi ve neden kodu |
| C04 | Bu kırılım kapsülünü reddet. | `reject` (`set_capsule_state` / R3 kapsül işlemi) doğru kapsülde; günlükte |
| C05 | En önemli üç kapsülü sırala. | `capsules` sorgusu, önem sırası; ilk üç doğru |
| C06 | Kırılım türünün karnesi ne? | `ledger` sorgusu (tür `breakout`); n ve gerçekleşen doğru |
| C07 | Hassasiyetimi "yalnız önemliler" yap. | `set_sensitivity` `important`, kendi kitlesinde |
| C08 | ASELS'i izleme listeme ekle. | kitle izleme listesine `ASELS` |
| C09 | THYAO'daki aşırılık kapsülünü kapsül derinliğinde göster. | `present` derinlik `capsule`, doğru bulgu (seçiciyle) |
| C10 | Aynı kapsülü grafik derinliğinde göster; kırılan seviye çiziliyor mu? | `present` `chart`; görünümde seviye katmanı, kökeni olgu |
| C11 | Akış kapsülünün kanıtını aç (yetkisiz kullanıcı olarak). | `capsule` sorgusu; kanıt değeri yok, gereken yetki adı var |
| C12 | THYAO kapsüllerine abone ol; yeni geçişleri bildir. | `subscribe` `capsules` konusu, enstrüman THYAO |
| C13 | Bu kapsülün geçmişini (bütün geçişlerini) listele. | `capsule` sorgusu; `history` tam |
| C14 | Bu olgunun kanıtını bugünkü veriyle doğrula. | `verify_evidence` doğru olguda |
| C15 | Yansıdaki veri bu kesin iddiayı tutmuyor; itiraz et. | `dispute` doğru iddia/kapsülde |
| C16 | Sıkışma kapsülü hâlâ geçerli mi? | `capsule` sorgusu; aşama ve cümle; cevap doğru |
| C17 | Son on barda doğan kapsülleri, türüne göre say. | `capsules` sorgusu, bar süzgeci; sayılar doğru |
| C18 | THYAO'da pozisyonum var (100 adet); bunu kaydet. | kitle `positions.THYAO = 100` |
| C19 | Gerçekleşen kapsülleri listele. | `capsules` sorgusu `stage: realized` |
| C20 | Bu kapsülü işaretli grafik olarak PNG'ye çiz. | `present` `chart` + ekransız çizim (`render`/`screen`); dosya üretildi |
