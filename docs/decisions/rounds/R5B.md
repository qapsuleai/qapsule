# Karar Günlüğü — Önerilen (R5B) — Arkin'in onayını bekliyor (2026-11-22)

<!-- dizin: DECISIONS.md; bu dosyadaki satırlar RG1 bölünmesinde aynen taşındı -->

## Önerilen (R5B) — Arkin'in onayını bekliyor (2026-11-22)

R5B (ajan yüzeyi) bu satırları **önerir**; ölçümler `rounds/R5B/REPORT.md`,
`rounds/R5B/experiments/{M28,M29,M30,M31,agents,injections,trace50}/`. Görev kümesi, doğru
cevap üreticisi ve seçim kuralları alternatiflerden önce commit edildi
(`rounds/R5B/experiments/harness/SPEC.md`).

| # | Öneri | Gerekçe |
|---|-------|---------|
| Ö94 | **Öneriyi onaylamak ve reddetmek (ve bulgu onay/ret) yalnız insan kökenli komutla:** ilkenin `author` alanı `human` **ve** bağlantı barındırıcının ajan bağlantısı diye işaretlemediği bir bağlantı (`Session::mark_agent_connection`). Aksi `forbidden`, `reason: human_only`. Ajanın MCP ucunda karar araçları hiç yok (çağrı: bilinmeyen araç); genel `command` aracıyla gelen karar her kipte otoritede reddedilir. | Arkin'in koşulu. Test: `crates/qp-mcp/tests/human_decision.rs` (3 test: araç listesinde yok; ajanın kendi önerisini adıyla, `command`'la dört kipte, başka ajanla ve işaretli bağlantıdaki insan kimliğiyle onaylayamaması; öneri bekler, doküman değişmez; insan kendi bağlantısında onaylar → `author: scout`, `approved_by: arkin`); döküm 65. Ölçümlerde (M28/M29, başvuru ajanları, gömülü talimat 42 örnek) ajan karar denemesi 0. |
| Ö95 | **Ajanın yazabildikleri izin listesiyle:** `add_comment`, `add_watch`/`remove_watch`, görünüm ve çizim **önerisi**; olgu, bulgu, kapsül, karne, hikâye durumu yazılamaz (`agent_cannot_write` / `proposal_only`); ajan kökenli bağlantı replay'i denetleyemez (yalnız durum; denetim B6 sonrası). `on_behalf` geçişsiz: bir insanı adlandırır, o insan başka birinin vekili olamaz. | PROTOCOL §25.3; dökümler 66; `agent_policy.rs`. |
| Ö96 | **Araç yüzeyi `thin` (protokolün aynası, şemadan üretilmiş).** | M28: 4 yüzey × 3 model × 45 görev; birincil (ilk denemede doğru, model ortalaması) thin 0,822, query/task 0,763, mixed 0,756; havuz aralıkları örtüştü → "ayırt edilemedi" → küçük model: thin haiku 0,756 vs 0,667–0,689. Sızıntı 0. Bedel: araç tanımı ~520 KB (istemci tembel yüklemiyorsa ağır; `docs/AGENTS.md`). |
| Ö97 | **Kapsül biçimi: varsayılan `json` (protokol cevabı); `--capsule-format card|layered` seçenek olarak kalır** (thin de uygular). | M29 (geçerli koşu): card 0,935, layered 0,925, json 0,903; aralıklar örtüştü; sayı aktarım hatası hepsinde 0/21 → belirteç: json 5952 < layered 6141 < card 6174. Geçersiz ilk koşu (format etkisizdi) A/A gürültüsünü ~4 puan gösterdi. Küçük modelde kart 27/31 vs json 25/31: daha büyük n ile yeniden ölçmeye değer (A58). |
| Ö98 | **Kısıtlı okur, görünümü redakte/yeniden yazılmış bir enstrümana dokunan başkasının yorumunu almaz** (yapısal; metin taramaları ek savunma). **Yorum doğrulayıcı sıkılığı `cites_numbers` (varsayılan):** atıf + her sayı atıf yapılan kanıtta ya da adı verilen dış alıntıda (yazılan basamağa yuvarlanmış), tavsiye dili, lisans, yön. Kısıtlı yazara ret nedeni körleştirilmiş, gerçek neden barındırıcının denetim kaydında. Metindeki kimlikler (`cap.…`) ve "para çıkışı" gibi akış adları taramadan çıkarılır. | M30 (149 bozuk + 113 doğru yorum): cites_numbers 149/149 yakaladı, 16/113 yanlış ret (hepsi bar sayısı ailesi); cites 85/149, 0/113; per_sentence 149/149, 33/113. Lisans sızıntısı ve olmayan olgu her sıkılıkta %100. Başvuru ajanlarında doğrulayıcıdan geçen yorumlarda uydurma sayı 0. |
| Ö99 | **Niyet (izleme):** yapılandırılmış seçici (enstrüman, anlam, yön, aşama, `negative`), eşleşme motorda, `intent_matched` olayı `intents` konusunda; sunucu kimlik verir (`watch-*`), kişi başına en çok 32, yalnız sahibi ve vekilleri görür. | `agent_watch.rs` (eşleşme bayt bayt deterministik); döküm 69. |
| Ö100 | **Nöbetçi süzgeci `tier_repeat_hold`** (önem + tekrar bastırma + kişinin elindekiler). | M31: her süzgeçte kaçırılan zorunlu olay 0/7, uydurma 0; seçilenin gereksiz bildirimi 0/8, günde 0,8 bildirim. Başvuru Nöbetçisi üç modelde de kaçırma 0, gereksiz 0. Pencere küçük, süzgeçler zor ayrışıyor (A57). |
| Ö101 | **`qp mcp serve` güvenlik varsayılanları:** stdio ya da yalnız `127.0.0.1` HTTP; `Origin`/`Host` denetimi; yerel dışı dinleme `--token-env` (Bearer, ortamdan) ister; sunucu başına bir ajan kimliği, oturum kimliğe bağlı (GET/DELETE de); `--principals` yalnız yerelde (ölçüm). MCP yapılandırması ortam değişkenine başvurur, gizli değer taşımaz. | `mcp_serve.rs` testleri; güvenlik incelemesi bulguları kapatıldı (REPORT). |
| Ö102 | **Gösterimde en az iz:** açılan kapsülde doğrulanmış ajan yorumu "ajan yorumu" etiketi ve kökeniyle ("aa, arkin adına"); bekleyen öneriler her işlem **bütün argümanlarıyla** (dizge değerleri tırnaklı, kesilmeden) ve ajan gerekçesi ayrı etiketli satırda; kabul/ret yalnız kabuğun kendi (insan) bağlantısından. | `crates/qp-wasm/tests/agent_trace_r5b.rs` (8 test); verify tarayıcı adımı yeşil, altınlar değişmedi. |

### Açık (R5B)

| # | Soru | Kim | Not |
|---|------|-----|-----|
| A57 | Yazı-tura karnesindeki (coin-flip) kademe-1 doğumlar nöbetçide bildirilmiyor; elindeki hissede bile mi bildirilmeli? | Arkin | M31 `RESULTS.md`. |
| A58 | Kapsül kartı küçük modelde json'dan iyi görünüyor ama A/A gürültüsü boyunda. Ürün ajanı için kart mı (daha büyük n ile yeniden ölçüm), json mı? | Sonraki round | M29. |
| A59 | Doğrulayıcı kullanılabilirliği: kapsülün bar alanları (`born_bar`), liste numaraları "(1)", yalın yıl "2018", dış alıntıda işaret yazımı ("%1,41 düşüş" ↔ −1,41) reddediliyor. Motor bunları kanıt mı saysın, ajan talimatı mı değişsin? | Arkin | Başvuru ajanları RESULTS; uydurma değil. |
| A61 | Doğrulayıcı kendi yorumunu bilerek gizleyen ajana karşı garanti değil (aralık içi uydurma tarih/saat, üst simge, Roma, sahte dış alıntı; tavsiye eş anlamlıları, Kiril harf). Lisans tarafı yapısal kuralla kapalı; uydurma sayı ve tavsiye için NFKC + biçim karakterlerini atma + daha geniş sözlük mü, yoksa yorumun yalnız motorun cümlelerinden kurulması (şablon) mı? | Arkin | `AUDIT.md` B4, B5. |
| A62 | Eski `qp mcp --init … --principal <insan>` bağdaştırıcısı (RD1) insan kimliğiyle karar araçlarını sunuyor (ayrı oturum). Kaldırılsın mı, ajan kimliğine mi kısıtlansın? | Arkin | `AUDIT.md` B8. |
| A60 | `thin` araç tanımı ~520 KB: tanımları bağlama bütün koyan istemciler (QODI?) için varsayılan yüzey `mixed`/`query` mı olmalı? | Arkin | M28: başarı ayırt edilemedi. |
