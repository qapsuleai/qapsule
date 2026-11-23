# RF1 PLAN v1 — Eleştiri 2: sonraki dört round'un yüküyle

Eleştirmen: bağımsız alt ajan (yalnız okuma, derleme yok). Girdi: `GOAL.md` ("Sonraki round'ların
yükü"), `PLAN.md` §3–§7, `docs/ENGINEERING.md` §2, `docs/CONCEPTS.md`, `docs/design/SURFACES.md`,
K72–K79, K88, K89, `../refs/reports/COMPUTE-LANGUAGE.md` §8, `../refs/reports/SHARED-ANALYSIS.md`
§2, §6 ve `crates/qp-session` kodu (`05430e06`). Satır numaraları o koddandır.

Kabul testi (GOAL): her round kendi crate'inde doğar, otoriteye yalnız kayıt satırı ekler,
`Session` gövdesine alan ya da yöntem eklemez. Aşağıda "kanca var" = plan §4/§7'deki arayüzle
otorite crate'ine dokunmadan yapılabilir.

## Kodda bulunan, planın görmediği bağlantı noktaları (kanıt)

| Nokta | Yer | Neden önemli |
|---|---|---|
| İşlem politikası tabloları otoritede | `authority/session.rs:334` `proposable`, `:361` `allowed` (izin), `authority/agent.rs:170` `agent_op` (`_ => Deny`) | Her yeni `Op` bu üç tabloya kol ister |
| İşlemin iki dünyaya ve okuyucuya gidişi | `capsule/public.rs:263–281`, `authority/session/worlds.rs:98`, `dispatch.rs:53`, `capsule/project.rs:399–421` (izleme isteği yalnız sahibine) | Kişisel işlem (izleme, üyelik, program) gizliliği işleme göre kodlanmış |
| Niyet/izleme değerlendirmesi otoritede | `authority/session/events.rs:146` `eval_watchers`, `:254` `eval_intents`, `:210` `intent_lines`; alanlar `intent_state`, `intent_epoch`, `directives`, `focus`, `att_counter`; `create.rs:62` `max_watches` | Planın `qp-watch`'ı (≈0,5 bin) yalnız `capsule/watch.rs`; R7'nin büyüteceği kısım otoritede kalıyor |
| İki dünya takası alt sistem durumunu da takas ediyor | `capsule/public.rs:62–79` `World`, `:173–192` `swap_world`: `live_index`, `view_reads`, `fact_ix`, `cap_ix`, `sentences`, `projs`, `views`, `head_state`, `headlines`, `intent_state` | "İki dünya otoritede kalır" ile "durum alt sistemde" birlikte tutmaz |
| Abonelik başına alt sistem durumu otoritenin `Sub`'ında | `authority/session.rs:84–97` (`memo`, `watch`, `stories`); `presence.rs` 15 `Topic::` | Abonelik kancası yok |
| Kapalı enum dağılımı (`qp-session` içinde) | `ChangeOp` 23 dosya / 264 kullanım; `Condition` 10 dosya (otoritede `values.rs`, `presence.rs`, `events.rs`); `Event` 13; `QueryResult` 14; `Topic` 11; protokol `Op` 14 dosya, 6'sı otoritede | Şemaya gelen her kol bugün otoriteye de dokunur |
| Barındırıcıya açık ayarlayıcılar `impl Session`'da | `story.rs:77,84`, `dispute.rs:178`, `reader.rs:636`, `clock.rs:22` | Yeni round'lar aynı kalıpla `Session`'a yöntem ekler |
| Yansı iç alanlara uzanıyor | `replica/follow.rs:130–248` (`s.plane.provider`, `s.doc`, `s.log`, `s.claims.take()`), `judge.rs:83–165` | S2b "dışa açık API kullanırlar, risk düşük" doğru değil |

## RL1 — program nesnesi

| İhtiyaç | Planda kanca? | Boşluk / öneri |
|---|---|---|
| `add_program` açılımı (Op kolu) | Var (`OpExpander`) | Açılım yeterli değil: izin (`allowed`), önerilebilirlik (`proposable`), ajan sınıfı (`agent_op`; bugün `_ => Deny`) otoritede. → `OpExpander` bir `OpPolicy` taşısın (bulgu 1) |
| `check_program` sorgusu | Var (`query`) | `Value` yerine `QueryResult` dönsün (bulgu 17); şema kolu RL1'in işi |
| Maliyet bütçesi denetimi (K78 hak, kullanıcı/plan kapsamı) | Kısmen (`check`) | `Host`'ta parametre çözümü yok (K-4 sırası nesne > enstrüman > kullanıcı > plan > sistem). → `host.param(key, scope)`; `welcome.limits`'e katkı için `limits(&mut Limits)` (bugün `create.rs:62` izleme sınırını otorite yazıyor) |
| Canlı değerlendirme | Plan: `qp-program::on_data` | Yanlış yer. Artımlı canlı yol (yapı kuşağı, `LiveIndex`, `Sub.memo`, değer önbelleği) `qp-livepath` + otoritenin `cache`'inde; paralel bir `on_data` K79'un "canlı = yeniden oynatma kodu"nu ve RP2 artımlılığını bozar. → `qp-livepath`'te **koşul türü kaydı** (`ConditionKind`), `Condition::Program` değerlendiricisi `qp-program`'dan kaydolsun |
| `Condition`'a `program` türü (§8.1/3) | Yok | `Condition` otoritede 3 dosyada eşleniyor (`values.rs`, `presence.rs`, `events.rs`). → koşul eşlemesi otoriteden çıkmalı (bulgu 4) |
| `ClaimRef`'e `signal` türü (sonuç kuralıyla) | Yok | `qp-claims`'e kol; kabul testi "kendi crate'inde"yi zorlar. → `ClaimKind` kaydı ya da `qp-claims`'e değişikliğin açıkça izinli olduğu yazılsın |
| Program serileri (gösterim, `peek`) | Yok | Değer önbelleği `qp-ops::values` + otoritenin `cache` alanı. → seri türü kaydı ya da program serisinin `study` gibi kayda girmesi |
| Kapanmış bar / seans hizası | — | `qp-lang` alan katmanında, `qp-calendar`'a bağlı; sorun yok |
| Yeni `ChangeOp`/doküman alanı (`programs`) | Yok | `apply`, ters çevirme, `canon`, `summary`, `describe` `qp-ops`'ta kapalı `match`. → `ChangeKind` kaydı (bulgu 4) |
| İki dünya (lisanslı akışta program) | Yok | Program durumu dünya başına mı? → `PerWorld<T>` (bulgu 3) |
| Yansı programı kendi verisinde yargılar | Yok | `replica/judge.rs` koşul türünü tanımalı → aynı koşul kaydını kullansın |
| Hata modeli (kapalı `ErrorCode` + alt kod) | — | Şema işi, RL1'de; plan engel değil |

## R7 — oyun alanı

| İhtiyaç | Planda kanca? | Boşluk / öneri |
|---|---|---|
| Çizim nesneleri (S1: 9 tür) | Plan: `qp-drawing` oturum alt sistemi | Gramer (alan) çizimi çizecek, "geometri yalnız çizimde" (K73), K88 katsayı taşıması geometriye uygulanacak → geometri, isabet, taşıma **alan katmanında** `qp-drawing` olmalı (gramerin altında). Oturum tarafı ayrı bir alt sistem gerektirmeyebilir: çizim doküman içeriğidir (`ChangeOp`), durumlu olan izlemedir (bulgu 11) |
| Çizim → koşul → olay | Kısmen | Koşul değerlendirmesi otoritede (`events.rs:146`); "yalnız kesin barda", "kendi periyodunda bar kapanışı" (S4). → `ConditionKind` (RL1 ile aynı kanca) |
| İzleme görevleri ve yaşam döngüsü (önerildi … süresi doldu, `İ-*` 17 durum, sağlık) | Plan: `qp-watch` büyür | Niyet değerlendirmesi, `intent_state`, `intent_epoch`, `att_counter` otoritede (`events.rs:202–303`). → RF1'de bunlar `qp-watch`'a taşınmalı (≈ +1 bin); yoksa R7 otoriteyi değiştirir (bulgu 2) |
| Sağlık "son kontrol 25 dk önce", süre dolumu | `on_clock` | Yeterli; ama barındırıcı saati ne zaman vereceğini bilmiyor. → `next_wake()` (RA1 hazırlık görevi de ister) |
| Niyet ve netleştirme (soru kartı, ≤3 seçenek, "Soru 1 / 2", cevapsız soru kurmaz) | Yok | Bağlantı/ajan başına açık soru durumu. → `on_connect` / `on_close` + bağlantı başına alt sistem durumu (bulgu 5) |
| Kur ve göster (K72; deterministik güven sayacı; dikkat yönergesi) | Belirsiz ("`qp-read`'de ya da `qp-drawing`'de") | Dikkat yönergeleri (`directives`, `focus`) ve öneri kabulü (`accept.rs`) otoritede; ajan sınıfı `agent_op`'ta. → `OpPolicy` + dikkat yönergesinin `HostMut::attend(...)` olarak verilmesi; sahibini kararlaştırın (`qp-watch` öneririm, `qp-read` sorgu/okumadır) |
| Yeni işlemler (çizim ekle/değiştir/sil, izlemeyi onayla/duraklat, soruyu cevapla) | Açılım var | İzin, önerilebilirlik, ajan sınıfı, iki dünya, okuyucuya gizlilik (izleme yalnız sahibine, `project.rs:399`) otoritede (bulgu 1) |
| "İzlemeler" aboneliği / yeni konu | Yok | `Topic` otoritede eşleniyor (`presence.rs`). → `topics()`, `on_subscribe`, `on_unsubscribe` (bulgu 5) |
| `İ-değerlendirilemiyor` (bayat, yetki yok), S22 gecikmeli | Kısmen | `host.binding_state(series)`, `host.entitled(principal, symbol)` (bugün `clock.rs:35` otoritede `pub(crate)`) |
| Grafik işaretleri: seviye etiketi, kırılım çentiği, kanıt bandı, niyet etiketi | `qp-grammar` alfabesi, ≈3,6 bin | Dört işaret için yeterli (bugünkü alfabe 926 satır). Ama SURFACES ayrıca: 9 çizim türünün çizimi, tutamaç ve çizim hareketi (`present/gesture` 111 satır), sol uç çipleri (S21), hayalet geometri (`İ-çizimdeğişti`), ◌/◷ rozetleri, 1/2/4 hücre (S8). Tahmin: işaretler ≈1,2 bin + çizimler ≈1,5–2 bin + hareket/isabet ≈0,6–1 bin = 3,3–4,2 bin → **sıkı** (bulgu 12) |
| Görünüş değerleri | `qp-look` (9 bin pay) | Yeterli |
| Durum cümleleri ("Kırıldı · tutunma 2/3 bar · günlük") | Yok | Metin çekirdekten (SURFACES). Gerçekleştirici `qp-voice`'ta; izleme cümleleri nereye? → `qp-voice`'a izleme dili ya da `qp-watch`'ta şablon; planda yazılsın |
| K88 "beklemede" | Bkz. RS1 | — |

## RS1 — sicil

| İhtiyaç | Planda kanca? | Boşluk / öneri |
|---|---|---|
| Tek sicil (iddia + sonuç; kapsül, çizim izlemesi, sinyal) | Plan: `qp-record`, `after_commit` + `on_clock` | Karne bugün `qp-capsule-life`'ta (`queries`, `settled`) ve dokümanın `ledger` alanında. RS1 bunu genişletecek → ya karneyi RF1'de ayrı bir tohum alt sisteme alın ya da RS1'in `qp-capsule-life`'a dokunacağını kabul edin. ENGINEERING §2 "sicil"i **alan** katmanına koyuyor: saf kurallar (tutma yüzdesi, taban oran, n<20 gizleme, sonuç kuralı) alan crate'i, oturum tarafı ince alt sistem (bulgu 11) |
| Ad | `qp-record` | K75 ve CONCEPTS: `Ledger` / `ledger entry`. "record/kayıt" planda `Registry` için de kullanılıyor → `qp-ledger` (bulgu 10) |
| Yeniden oynatma kaydı (simüle; canlıyla hiçbir toplamda karışmaz) | Yok | Alt sistem koşunun canlı mı oynatma mı olduğunu bilmiyor. → `host.run_kind()` (canlı / oynatma / benzetim) ya da kurulumda yapılandırma (bulgu 9) |
| Tutma yüzdesi, taban oran, zaman gezintisi (S23: oynatmada imleçten sonrası gizli) | `query` | Sorgu yeterli; gizleme okuyucu izdüşümüne değil oynatma imlecine bağlı → `host.replay_cursor()` |
| Günlükten yeniden kurulum | §7.3 "değişimden çalışır" | Sicil istatistiği türetilmiş durum; `apply::replay` dokümanı kurar, alt sistem önbelleğini kurmaz. → `rebuild(&dyn Host)` kancası ya da durumun yalnız dokümandan türediğinin kuralı |
| K88 kurumsal işlem (beklet → yenile → göster → sor) | Plan: `qp-record::on_data` | Tek alt sistemin işi değil: çizim geometrisi, izleme koşulları ("beklemede"), açık iddiaların düzeyleri, hesapların yeniden kurulması, **tek mesaj**. `on_data` içinde öteki alt sistemleri `&mut` değiştirmek ödünç kuralına takılır. → `DataEvent::Adjustment{symbol, kind, factor, at}` her alt sisteme sırayla; her biri `adjust() -> {changes, held, report}` döner; eşgüdüm `qp-subsystem`'de, tek motor girdisi + tek bildirim (bulgu 7) |
| Kurumsal işlemin girişi | Yok | Barındırıcıdan gelir (veri düzlemi ya da yeni giriş). `Session::set_corporate_actions` eklenmemeli → genel `host_input` / `sub_mut::<T>()` (bulgu 8) |
| Sicil aboneliği | Yok | Bulgu 5 |
| Yansı / oynatma barındırıcısı | `qp-replica`, `qp-replayhost` | İç alanlara uzanıyor (`follow.rs`, `judge.rs`); taşıma "düşük risk" değil (bulgu 15) |

## RA1 — hazırlık, ortak analiz, aralık okuması

| İhtiyaç | Planda kanca? | Boşluk / öneri |
|---|---|---|
| Hazırlık görevi (olay öncesi dosya) | `qp-prep`, `on_clock` | Olay takvimi verisi nereden? Barındırıcı girdisi kancası yok (bulgu 8). Zamanlanmış uyanma → `next_wake()`. İçerik modelle üretilir: otoritede LLM yok kuralı gereği alt sistem yalnız görevi ve durumunu tutar, üretim barındırıcı/ajan tarafında — planda yazılsın |
| Ortak analiz (bir kez üretilir, çok kişi üye olur, sayaç kimliksiz ve rızalı) | Plan: `qp-shared` oturum alt sistemi | Otorite "belge başına tektir" (CONCEPTS). Ortak analiz belgeler/oturumlar arasıdır (SHARED-ANALYSIS §2.2 tekilleştirme, §2.3 sayaç). Üretim deposu ve tekilleştirme barındırma katmanına (`qp-host` ya da yeni uyarlayıcı crate) aittir; oturum alt sistemi yalnız üyelik, kişisel katman (`PersonalOverlay`) ve izdüşüm tutar (bulgu 14) |
| Üyelik işlemleri (katıl/ayrıl; kişisel, önerilemez) | Açılım var | `AddWatch` gibi kişisel: izin, önerilebilirlik, ajan sınıfı, okuyucuya gizlilik otoritede (bulgu 1) |
| Bölüm bazında lisans izdüşümü (§2.5) | `project` | `project(host, reader, out)` belirsiz. Bugün izdüşüm dokümanı **ve** günlük girdisini yeniden yazıyor (`project_entry`, `withheld_for`). → `project_doc(&mut Document)` ve `project_entry(&mut LogEntry)` ayrı kancalar |
| Zaman aralığı okuması + sayı/atıf doğrulayıcısı | `qp-read` + `query` | Doğrulayıcı bugün yorum için `qp-claims`'te (`comments`); sıra `qp-claims < qp-read` uygun. Yeterli |
| Ad | `qp-shared` | Belirsiz ("ortak" ≈ common). → `qp-analysis` ya da `qp-membership`; kavram CONCEPTS'e (bulgu 10) |
| Rızalar / hak (etkin üyelik sınırı satılabilir) | Yok | `host.param` + profil rızası okuması (bulgu 9) |

## Genel bulgular

1. **(yüksek) İşlem politikası otoritede kalıyor.** `OpExpander` yalnız açılımı taşıyor; izin
   (`allowed`), önerilebilirlik (`proposable`), ajan sınıfı (`agent_op`), iki dünya eşlemesi ve
   okuyucuya gizlilik otoritede kapalı `match`'lerle. RL1, R7 ve RA1'in her yeni işlemi bunlara
   dokunur. Öneri: `OpExpander::policy() -> OpPolicy { perm: Write|Read|Decide|Propose,
   proposable: bool, agent: Proposal|Direct|Deny, personal: bool, public: Mirror|Skip|Own }`;
   otoritedeki tablolar `_` kolunda kayda sorar. S4 ile birlikte yapılır.
2. **(yüksek) Niyet/izleme değerlendirmesi otoritede kalıyor.** `eval_watchers`, `eval_intents`,
   `intent_lines`, `intent_state`, `intent_epoch`, `directives`, `focus`, `att_counter`,
   `max_watches` ataması. Planın `qp-watch`'ı yalnız `watch.rs`. R7'nin ana işi (yaşam döngüsü,
   sağlık, kur ve göster) doğrudan otoriteye yazılır. Öneri: S3.3'te bunlar `qp-watch`'a
   (CONCEPTS adı "niyet": `qp-intent` daha doğru olabilir); `qp-authority` ≈ 5,6 → ≈ 4,6 bin.
3. **(yüksek) İki dünya ile alt sistem durumu çelişiyor.** `World` takası canlı yol, kapsül
   önbellekleri ve niyet durumunu da takas ediyor. Durum alt sisteme geçince ya otorite alt
   sistem alanlarını tanır (kabul testi bozulur) ya da takas yarım kalır (davranış değişir).
   Öneri: `qp-subsystem`'de `PerWorld<T>` (iki kopya + etkin dünya) ya da
   `Subsystem::swap_world(&mut self)`; her alt sistem `world_policy()` bildirir
   (`Both` / `AuthorityOnly`). S3.0'da tasarlanmalı, S3.4'e bırakılmamalı.
4. **(yüksek) Kapalı enum dağılımı ölçülmemiş.** Yeni `Op`, `ChangeOp`, `Condition`, `Topic`,
   `Event`, `QueryResult` kolu bugün 10–23 dosyaya dokunuyor; bir kısmı otoritede. Şema round'un
   işi olsa da kolun **nereye** ineceği RF1'in işi. Öneri: (a) her kapalı enum için tek dağıtıcı
   ve `_` kolunda kayıt (`ChangeKind`: apply/invert/canon/summary/describe/izdüşüm;
   `ConditionKind`: değerlendirme/engelleme/kanıt; konu ve olay için abonelik kancası);
   (b) ölçüt: "yeni kol = şemada kol + kendi crate'inde uygulama + `qp-ops`/kayıtta en çok bir
   dağıtıcı satırı; otoritede 0". (c) `experiments/fanout.md`'de enum başına önce/sonra dosya sayısı.
5. **(yüksek) Abonelik ve bağlantı kancası yok.** `Sub.memo/watch/stories` otoritenin yapısında;
   `presence.rs` konuları eşliyor. R7 (izlemeler, açık sorular), RS1 (sicil), RA1 (ortak analiz)
   yeni konu ve bağlantı başına durum ister. Öneri: `topics()`, `on_subscribe(conn, sub, topic)
   -> initial`, `on_unsubscribe`, `on_connect`, `on_close`; abonelik başına durum alt sistemde
   `(conn, sub_id)` anahtarıyla.
6. **(orta) `HostMut` dyn-uyumlu değil, kayıt ödünç sorunu.** `fn in_world<R>(…)` genel yöntem
   `&mut dyn HostMut`'ı imkânsız kılar → `fn in_world(&mut self, public: bool, f: &mut dyn
   FnMut(&mut dyn HostMut))`. Kayıt otoritenin alanıysa kanca çağrısında `&mut self` ile
   `&mut host` aynı anda alınamaz → çağrı sırasında kaydı dışarı al (`mem::take`) ve öbür alt
   sistemlere yalnız okuma (`host.sub::<T>()`) ver; değiştirme iç olayla (bulgu 7).
7. **(orta) Alt sistemler arası işbirliği (K88) tanımsız.** `Registry::get::<T>()` okuma için
   yeter, eşgüdümlü değişiklik için yetmez. Öneri: `Adjustment` olayı + her alt sistemin
   `adjust()` planı + `qp-subsystem`'de birleştirici (tek motor girdisi, tek bildirim,
   "taşınamadı → beklemede + soru").
8. **(orta) Barındırıcı API'si için kanca yok.** Bugün `set_coin_flip_rule`,
   `set_settle_check`, `set_headline_calendar`, `comment_audit` `impl Session`. Bölünmeden sonra
   bunlar ve RS1/RA1'in yenileri (kurumsal işlem, olay takvimi, üyelik sayacı) ya bileşimde
   `impl Session`'a yöntem olur (kabul testi bozulur) ya da genel erişim gerekir: `sub::<T>()`,
   `sub_mut::<T>()`, `drive::<T>(|s, host| …) -> Vec<OutboundLine>` (çıktı ve günlük için).
9. **(orta) Kurulum, sınır, parametre, koşu türü.** `runner` `init.capsules`'tan, güven
   anahtarları kurucudan, `max_watches` `welcome.limits`'e otoriteden. Öneri: `build(&Init,
   &HostConfig)`, `limits(&mut Limits)`; `Host`'a `param(key, scope)` (K78, K-4),
   `entitled(principal, symbol)`, `binding_state`, `run_kind()`, `replay_cursor()`, `next_wake()`.
10. **(orta) Adlar ve kavramlar.** `Host` trait'i CONCEPTS'teki "Barındırma | host (`qp-host`)"
    ve `Role::Host` ile çakışıyor → `AuthorityView`/`Ctx` gibi bir ad. `qp-record` → `qp-ledger`
    (K75). `qp-shared` → işini söyleyen ad. "Kayıt" hem `Registry` hem sicil kaydı için
    kullanılıyor. Sözlükte yok, kod yazılmadan eklenmeli: alt sistem, bileşim, işlem açılımı kaydı,
    tasarım kaydı (`Designs`), kapsül yaşamı, canlı yol (yalnız ROADMAP'te geçiyor). `qp-capsule-life`
    kabul edilebilir; `qp-livepath`, `qp-read`, `qp-viewer`, `qp-ops` işini söylüyor.
11. **(orta) Katman yerleşimi ENGINEERING §2 ile uyuşmuyor.** §2 "hesap dili, sicil"i alan
    katmanına koyuyor; plan `qp-record`'u tümüyle oturuma koyuyor. Çizim geometrisi gramerin
    altında olmalı (gramer onu çizer, K88 taşır); plan `qp-drawing`'i oturuma koyuyor, gramer onu
    göremez. Öneri: §5 tablosunda alan/oturum ikilisi: `qp-lang`+`qp-program`,
    `qp-drawing` (alan) + `qp-watch` (oturum), `qp-ledger` (alan kuralları) + oturum tarafı.
12. **(orta) R7 gramer payı sıkı.** Dört işaret 3,6 bine rahat sığar; SURFACES'in çizim türleri,
    tutamaç/hareket, çipler ve hayalet de çekirdeğe girerse 3,3–4,2 bin. Öneri: çizim geometrisi
    ve isabeti `qp-drawing` (alan) crate'ine; `present/depth/card.rs` (451) gibi yüz tasarımına
    yakın parçalar `qp-designs`'a; hedef ≥ 4,5 bin pay.
13. **(orta) RL1'in canlı değerlendirmesi `qp-program`'da paralel yol kurmamalı.** Koşul türü
    kaydı `qp-livepath`'te; program serisi değer önbelleğine kayıtla. Aksi halde RL1 ya
    `qp-livepath`'i değiştirir ya da artımlılığı kopyalar.
14. **(orta) Ortak analiz oturumlar arasıdır.** `qp-shared`'in yalnız belge içi tarafı oturum
    alt sistemi olabilir; üretim, tekilleştirme ve kimliksiz sayaç barındırma katmanında. §5'e yazılsın.
15. **(orta) Yansı ve oynatma barındırıcısı iç alanlara uzanıyor.** `follow.rs` `plane.provider`,
    `doc`, `log`, `claims.take()`; `judge.rs` `plane`, `doc`. S2b "taşıma, düşük risk" değil:
    ya açık API eklenir (S3'ten sonra) ya da bu iki crate bileşimin yanında kalır. RS1 (oynatma
    kaydı) ve RL1 (yansının program iddiasını yargılaması) buna bağlı.
16. **(orta) Olay sırası "kayıt sırası" ile kurulamaz.** Bugünkü §15 sırası alt sistemleri iç
    içe geçiriyor (`settle.rs` ve `events.rs`: geçersizleme → koşul aboneleri → niyetler → kamu
    dünyası). Tek `on_data`/`after_commit` × kayıt sırası bunu ancak bir alt sistem birden çok
    kez çağrılırsa verir. Öneri: otoritede adlı evreler (`Phase::{Invalidate, Conditions,
    Intents, Capsules, Views}`); alt sistem evre başına kaydolur; yeni alt sistem evresini seçer.
17. **(düşük) Küçük arayüz notları.** `query` `QueryResult` dönmeli (`Value` gidiş-dönüşü tip
    kaybı). `CompileOptions` `Clone, PartialEq` türetiyor ve 45 dosyada; `&dyn Designs`'ı içine
    koymak türetmeleri bozar → ayrı parametre ya da `&'static dyn Designs`. İşlem türü adı
    `op_name` (serde) ile `qp-ops`'ta kalsın; `qp-schema`'ya yöntem eklemek ölçüt 2'ye takılır.
18. **(düşük) Bu round'da şema/protokol değişikliği zorlanmıyor.** Yukarıdaki kancaların hepsi
    Rust içi. Risk yalnız: `Op::kind()`'ın şemaya konması, `Limits`'e alan eklenmesi (ikisi de
    yapılmamalı). Sınırlar ve yeni konular sonraki round'ların şema işi.
19. **(düşük) Ek noktası sınaması en kolay yolu sınıyor.** `Tally` var olan bir işlemi sayar;
    politika, gizlilik, abonelik, iki dünya sınanmaz. Öneri ek sınama: `add_watch`/`remove_watch`
    bütünüyle (açılım + politika + gizlilik + sınır + konu) `qp-watch` kaydından gelsin;
    bileşimden `qp-watch` satırı çıkarılınca `add_watch` "bilinmeyen işlem" hatası verir, otorite
    `src/` farkı 0. Bu, kabul testinin gerçek kodda kanıtıdır.

## Önerilen arayüz eki (özet)

```text
trait Subsystem {
    fn build(init: &Init, cfg: &HostConfig) -> Self where Self: Sized;
    fn expanders(&self) -> &[&dyn OpExpander];          // + OpExpander::policy() -> OpPolicy
    fn kinds(&self) -> Kinds { … }                      // ChangeKind, ConditionKind, ClaimKind
    fn phases(&self) -> &[Phase];                       // olay evreleri
    fn topics(&self) -> &[TopicKind];  fn on_subscribe(…); fn on_unsubscribe(…);
    fn on_connect(…); fn on_close(…);
    fn limits(&self, l: &mut Limits);  fn next_wake(&self) -> Option<i64>;
    fn adjust(&self, h: &dyn Ctx, a: &Adjustment) -> AdjustPlan;
    fn project_doc(…, &mut Document); fn project_entry(…, &mut LogEntry);
    fn world_policy(&self) -> WorldPolicy;  fn swap_world(&mut self);
    fn rebuild(&mut self, h: &dyn Ctx);
}
Ctx (bugünkü Host): + param, entitled, binding_state, run_kind, replay_cursor, sub::<T>()
CtxMut: in_world(public, &mut dyn FnMut(&mut dyn CtxMut)), attend(...)
Session (bileşim): sub::<T>(), sub_mut::<T>(), drive::<T>(…)
```

## Hüküm

Yön doğru (otorite + kayıt + alan/oturum ayrımı, risk sırası, geri dönüş). Ama planın §4
arayüzüyle **dört round'un hiçbiri kabul testini geçmez**: RL1 ve RA1 işlem politikası
tablolarına (bulgu 1), R7 otoritede kalan niyet değerlendirmesine ve aboneliğe (2, 5), RS1
iki dünya takasına, alt sistemler arası K88 eşgüdümüne ve barındırıcı girdisine (3, 7, 8)
takılır. Kod yazmadan önce yapılması gerekenler: bulgu 1–5 arayüze girsin; 10–11'deki ad ve
katman düzeltmeleri, kavramlar CONCEPTS'e işlendikten sonra §3.3 ve §5'e yazılsın; ek noktası
sınaması bulgu 19'daki gerçek alt sistemle genişlesin. Bunlarla plan onaylanabilir; bunlar
olmadan RF1 "bölünmüş ama genişletilemez" bir oturum bırakır.
