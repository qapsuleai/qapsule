# RF1 / S3.3 — bağımsız denetim

**Dal:** `rf1/s33-replica` @ `87a3664e` (taban `main` @ `a2a3f641`).
**Denetçi:** bağımsız alt ajan; kendi worktree'si (`.claude/worktrees/audit-s33`, `--detach`),
kendi derleme klasörü (`target/wt/audit-s33`). Dal değiştirilmedi, birleştirilmedi.
**Kapsam:** (c) `qp-replayhost`, (b) `qp-viewer`, (a) `qp-replica`; İ3 tavanı 12.744; İ8 (kabuk
WASM paketi, Arkin'in kararı, `DECISION.md` "Uygulama notları").

## Hüküm: **GEÇTİ**

Engelleyici bulgu yok. Üç yumuşak bulgu (Y1–Y3) ve iki not var; hiçbiri birleştirmeyi
engellemiyor. Y1 sonraki round'un "Öneriler"ine yazılmalı.

## 1. Doğrulama

| Adım | Sonuç |
|---|---|
| `scripts/verify.sh` (21 adım) | **OK**, 1002 sn; her adım PASS |
| 1 fmt / 2 clippy `-D warnings` | PASS / PASS |
| 3 `cargo test --workspace` | PASS; toplam 1243 geçti, 0 kaldı |
| S1 karakterizasyonu (`qp-session` `tests/charac`) | **25/25** |
| G1 karakterizasyonu (gramer `tests/charac`) | **11/11** |
| Ek noktası (`tests/extension`) | **4/4** |
| Liste sınaması | `compose::lists::the_follower_list_is_the_standard_list_without_reading_and_viewers` ok |
| Tutanaklar (`crates/qp-session/tests/charac/expected`) | `main`'e göre değişmedi (dalda bu klasörde fark yok), bayt eşit |
| `scripts/refactor/compare.sh` | **270/270 bayt eşit** |
| 15 replica | lag 0: 186 adım, 0 sapma, 0 ihlal; lag 25: 0 ihlal, son iddialar 9/9 sonda kesin; görüntüleyici lag 0/25: 0 ihlal, öbür enstrümana abonelik 0; wasm32 (Node) = yerel 186/186 ve 211/211 (doküman, görünüm, gösterim); lag 0'da wasm = otorite 186/186 |
| 19 arch / 20 config | PASS / PASS |
| `moved.py main HEAD` | 580 değişmeden, 20 yalnız yol, 154 `CHANGED`, 27 `FLAG`, 37 yeniden dışa aktarım; `rewritten.md` S3.3 ile aynı |

## 2. Gizli yöntemler

### 2.1 Liste ve eşdeğerlik

`#[doc(hidden)] pub`; çağıranlar `grep` ile tarandı: her biri yalnız amaçlanan crate'ten çağrılıyor.

| Yöntem | Yer | Çağıran | Eski kodla karşılaştırma |
|---|---|---|---|
| `build_follower` | `authority/session/follow.rs` | `qp-replica` | `build_with(init, ".", Provider::None, Null, subs)`: eski `follower()` ile aynı |
| `follow_start` | aynı | `qp-replica` | eski `replica.rs:229-241`'in 12 alan yazımı, aynı sırada, satır satır aynı |
| `take_provider` | aynı | `qp-replica` (`load`) | eski `mem::replace(&mut s.plane.provider, None)` |
| `mirror_apply` | aynı | `qp-replica` (`mirror`) | eski `Restore{initial,log}` + `apply_in_place`; hata yolu aynı (`false` → yeniden yükleme) |
| `mirror_commit` | aynı | `qp-replica` (`mirror`) | eski `committed(e)`, `pending_views.clear()`, `claims.take()`; `self.seq` ve kanıt öğrenme yine çağıranda, aynı sırada (kanıt, uygulamadan sonraki `document()` üstünden: eski `s.doc` ile aynı alan) |
| `follow_data` | aynı | `qp-replica` (`data_event`) | eski `apply_event` → boşsa erken dönüş → `data_changed` → `pending_views.clear()`; kimlik listesi döner, `reconcile_on` çağıranda |
| `follow_resync` | aynı | `qp-replica` (`resync`) | eski gövde aynen |
| `eval_parts` | aynı | `qp-replica` (`judge`) | `rd!` = `rd.unwrap_or(&doc)`; `judge.rs` farkı yalnız yol (`s.doc` → `doc`, `s.plane` → `plane`, `&mut s.cache` → `cache`, `rd!(s)` → `s.reading()`, ki `rd!(self)`'tir) |
| `seq_to` | `capsule/public.rs` | `qp-replayhost` | gövde değişmedi; yalnız `pub(crate)` → gizli `pub` |
| `conn_agent_origin` | `authority/agent.rs` | `qp-replayhost` (`control.rs`) | eski `conns.iter().position(id).is_some_and(agent_origin)`, aynen |
| `reader_parts_of` | `capsule/headsource.rs` | `qp-replayhost` `tests/headparts.rs` | `as_reader(.., headline_parts)`: eski sınamanın çağrısı |
| kök: `anchor_of`, `closed`, `eval_at`, `feeds_of` | `lib.rs` gizli yeniden dışa aktarım | `qp-replica`, `qp-viewer` | saf fonksiyonlar; yalnız görünürlük |
| `Replica::start_from` | `qp-replica` | `qp-viewer` (`OpenSide`) | `start(doc.clone(), seq)`; WASM için tek klon örneği |

`ReplayHost` dosyalarındaki fark yalnız bu üç çağrı ve `SessionView::conn` (ilk eşleşen kimlik;
eski `find` ile aynı). Davranış farkı bulunmadı.

### 2.2 Komut yolunun atlanması (Y1)

Salt okuyanlar zararsız: `seq_to`, `conn_agent_origin`, `reader_parts_of`, `eval_parts`
(yalnız önbelleği `&mut` verir) ve saf kök fonksiyonları.

Doküman ve günlüğe **girişsiz yazanlar** şunlar:
- `follow_start`: dokümanı ve `initial`'ı değiştirir, günlüğü siler.
- `mirror_apply`: değişimi dokümana uygular, günlüğe yazmaz.
- `mirror_commit`: verilen `LogEntry`'yi komut yolundan geçirmeden kaydeder.
- `follow_data` ve `follow_resync`: motor girdisi olmadan canlı yolu koşar.
- `take_provider`: veri düzlemini sağlayıcısız bırakır.

Bunlar `qp_authority::Session`'ın yöntemleri. `qp_session::Session(Authority)` `DerefMut` verdiği
için her barındırıcı (`qp-host`, `qp-wasm`, `qp-shellcore`, `qp-mcp`, `qp-cli`) onları
çağırabilir. Dolayısıyla "dokümanı değiştiren her şey komut yolundan geçer" değişmezini derleyici
değil, yalnız gelenek koruyor. Bugün kötüye kullanım yok (çağıranlar yalnız `qp-replica`).
Alt sistemler bunlara ulaşamaz, çünkü yalnız `SessionCtx` alırlar.

Plan §3.3 satır 6 bu tekniği (`#[doc(hidden)] pub`) önceden kabul etmişti. Eskiden aynı yetki
crate içi alanlarla vardı. Bu yüzden engelleyici değil.

**Daha dar yerleşim mümkün:**
- (a) **Önerilen.** Otoritede yeni bir `Follower(Session)` tipi. Yalnız `Follower::build(doc,
  provider, principal, subs)` kurar; yazan yöntemler (`mirror`, `follow_data`, `follow_resync`,
  `take_provider`) yalnız bu tipte olur, okuma için `Deref<Target = Session>` verilir. Böylece
  barındırıcıların tuttuğu `Session`'da bu yöntemler hiç bulunmaz. `Replica::session_mut`
  `&mut Session` yerine `&mut Follower` ya da `&Session` döner.
- (b) Daha küçük adım: `build_follower` ile `follow_start` tek kurucuda birleşir; o zaman
  "var olan bir oturumu sıfırla" yüzeyi kalmaz. `mirror_apply` ile `mirror_commit` de tek
  `mirror(entry) -> Result<(), _>` olur. Kanıt öğrenme uygulamadan sonraki dokümanı okur,
  `commit` dokümanı değiştirmez, yani tek çağrıdan sonra da okunabilir.
- (c) Kısa vadede: `arch-check`'e "bu adlar yalnız `qp-replica`'dan çağrılır" kuralı.

## 3. Görüntüleyici alt sistemi

**Çağrı noktaları:** `intake.rs` (2×, `has_pushers` korumalı), `settle.rs` (`!viewer_batch &&
has_pushers`) ve `clock.rs` değişmedi. `ReplayHost`'un 6 `viewer_flush` ve 3 `set_viewer_batch`
çağrısı aynı satırlarda. `claims.rs` `viewer_claim_sub` → `push_claim_sub`: aynı yer, aynı
koşul. `dispatch.rs`: `visible_msg` → `dispatch_visible`; abonelik onayından sonra ilk olay
(`viewer_start` → `push_lines(h, ci, all=true)`).

**Sıra:**
- `viewer_flush` önce işleyicileri, sonra bağlantıları sırayla dolaşır. Tek itilen işleyici
  olduğu için bağlantı sırası eskisiyle aynı; `stories_flush` yine sonda.
- `has_viewers` eskiden `!viewers.is_empty()` idi, şimdi `any_conn::<ViewerConn>()`; eşdeğer.
- `viewer_lines` / `viewer_update` / `viewer_compute` → `push` / `viewer_update` /
  `viewer_compute`: gövdeler alan alan aynı. Tek fark `dispatch_query` yerine
  `ctx.registry.query(&q).answer(conn, &q, ctx)`. Eski `dispatch_query` de bu çağrıyı
  `with_read_ctx` içinde yapıyordu; ek denetimi yok, aynı cevap.
- `seq` artık hesaplamadan önce alınıyor. Hesaplama günlüğe yazmadığı için değer aynı.

**Yuva ve dünya:**
- `ViewerConn` bağlantı başına yuvada (`Slots::conns`).
- `swap_world` yalnız `PerWorld`'leri takas eder. Bu yüzden `as_reader` içinde okunan yuva,
  eskiden dünya dışında okunan `viewers` alanıyla aynı.
- `looks` ve `conns` dünya takasına girmiyor. `ctx.looks[ci]`'deki `ci`, `conn_ids()` sırası
  olarak `self.conns` sırasıyla aynı.
- `face_scene_in` eski `face_scene(self, …)` ile aynı sırayı izler: başlık parçaları, sonra
  `warm`, sonra `DocProvider::with_headlines`. Tek fark önbelleğin bağlamdan gelmesi.

**`viewer_batch`:** otoritede kaldı. Gerekçe (`stories` de ona bağlı) doğru.

**Abonelik ve silinme:**
- Denetim artık yerleşik abonelikten **sonra** `ViewerTopic::subscribe`'da. Eskiden
  `subscribe()` içindeki konu eşlemesindeydi, ama oradan sonra başka denetim yoktu. Yani
  hataların önceliği (en çok abonelik, kimlik, sonra görüntüleyici denetimi) aynı. Ret
  aboneliği geri alır; yuva hiç kurulmadığı için eski `viewer_unsubscribed` gereksiz.
- Silinme anı (Y3): yuva artık `Phase::Unsubscribe` içinde, kayıt sırasıyla (izleme ve okumadan
  sonra) silinir. Eskiden fazdan önce siliniyordu. Ayrıca yuva artık `Phase::Subscribe`'dan önce
  kuruluyor; eskiden onaydan sonra kuruluyordu. Hiçbir alt sistem `ViewerConn` okumadığı için
  gözlenebilir fark yok; 270/270 ve 25/25 bunu doğruluyor. Yeni bir alt sistem bu fazlarda
  görüntüleyiciye bakarsa sıranın değiştiğini bilmeli; `qp-viewer` README'sine bir satır yeter.
- Bağlantı kapanışında eskiden de silme yoktu (protokolde kapanış iletisi yok); aynı.

**`visible` → `unsupported` (görüntüleyicisiz):** kabul edilebilir. Kayıtlı görüntüleyicide
`ViewerTopic::visible` her zaman `Some` döner, `not_found` dahil; bileşimde çıktı aynı. Fark
yalnız standart dışı listede çıkar ve ARCHITECTURE'ın "kayıtsız otorite `unsupported` döner"
kuralıyla tutarlı.

**Y2 (belgelenmemiş benzer fark):** görüntüleyicisiz bir otoritede `{"kind":"viewer"}`
aboneliği artık **sessizce kabul edilir**: `presence.rs` kolu boş, işleyici yok, onay
`subscribed`, `limit` ve `width` denetlenmez, hiç olay itilmez. Eskiden denetlenir ve ilk olay
itilirdi. Yalnız standart dışı listede gözlenir ve üretimdeki izleyicilere abonelik gelmez (§4).
Öneri: `rewritten.md` S3.3'e `visible`'ın yanına bir satır, ya da kayıtsız itilen konu türüne
`unsupported`. Bu round'da düzeltme zorunlu değil.

## 4. Görüntüleyicisiz FOLLOWER

- `FOLLOWER_LIST = [WATCH]`, `STANDARD = [WATCH, READ, VIEWER]`, aynı sırada; liste sınaması
  bunu kilitliyor.
- `FOLLOWER` ile kurulan otoriteler:
  - `Replica` (`qp_session::Replica::{new, with_memory}`)
  - `OpenSide` → içindeki `Replica` (`compose.rs:269`)
  - şeritler (`tape::run*`, `run_viewer`): `(STANDARD, FOLLOWER)`
- `OpenSide` yalnız bir `Replica` tutar, `OpenSide` içinde `OpenSide` yok. Yansının otoritesine
  hiçbir üretim yolu `handle_raw` / `subscribe` göndermez:
  - `qp-replica` ve `qp-viewer`'daki `handle_raw` çağrıları şeridin `STANDARD` otoritesine gider.
  - `Replica::session_mut` çağıranları yalnız okuma, sahne ya da ayarlayıcı kullanıyor.
  - `charac` düzeneğindeki `Target::R` bir `ReplayHost`'tur (`STANDARD`).
- Sonuç: görüntüleyici bağlantısı olan bir izleyici yok.

## 5. Yapı

- `arch-check`: **0 hata**. 2 adlı istisna (İ3 ve `qp-source` katmanı); borç taban çizgisi içinde
  (`fn.lines` 173, `lint.allow` 22, `file.soft` 32 …).
- `--config-diff main`: 22 değişiklik, **0 kaynaksız gevşetme**.
  - İ3 16.707 → 12.744 (sıkılaşma).
  - Üç yeni crate `oturum` katmanına yerleşti.
  - 8 `fn.lines` kalemi yer değiştirdi (uzunluk aynı); 3'ü sıkılaştı.
  - `lint.allow` `qp-authority` 36 → 25, yeni crate'lere 3 + 4 + 4 (toplam aynı).
- **İ3:** ölçülen `qp-authority` 12.744 test dışı satır = tavan, ≤ 13.000. 12.000'i aştığı için
  istisna sürüyor (karar 1).
- **`cargo tree`:** `qp-authority` normal bağımlılıkları arasında watch, read, viewer, replica
  ve replayhost **yok**; dev-dependency yok. Sıra: `qp-plane < qp-ops < qp-subsystem < qp-watch
  < qp-read < qp-authority < qp-replica < qp-viewer < qp-replayhost < qp-session`. `qp-viewer →
  qp-replica` (OpenSide) aşağı doğru; `qp-replayhost` yalnız authority, plane, schema, source
  ve subsystem'e bağlı.
- **README'ler:** üç crate kökünde ve `src/{viewer,replica,replayhost}/` alt klasörlerinde var.
- **`lib.rs`:** yalnız belge, `#![allow(clippy::result_large_err)]`, `mod`/`pub mod` ve
  `pub use`. Modül düzeyinde başka izin yok; öğe düzeyindeki izinler (`too_many_lines`,
  `expect_used`, `too_many_arguments`) taşınan öğelerle birlikte geldi ve taban çizgisinde.

## 6. WASM

`perf.sh` komutları; paketler `target/wt/audit-s33` altına kuruldu, `shells/web/pkg`'ye dokunulmadı.

| Paket | Taban | Dal | Fark | Sınır | Sınıra kalan |
|---|---|---|---|---|---|
| varsayılan raw | 2.345.821 | 2.354.884 | **+%0,386** | ±%2 | 37.853 B |
| varsayılan gzip-9 | 769.836 | 773.647 | **+%0,495** | ±%2 | 11.585 B |
| kabuk (web, `--features replica`, `--no-opt`) raw | 8.789.414 | 9.004.348 | **+%2,445** | +%2,5 (İ8) → 9.009.149 | **4.801 B** |
| kabuk gzip-9 | 2.432.388 | 2.485.016 | **+%2,164** | +%2,5 (İ8) → 2.493.197 | 8.181 B |

- Kabuk paketi İ8 içinde. Yazarın ölçümünden (9.004.524) 176 B küçük; fark büyük olasılıkla
  `e5e6e6d7`'den.
- **Not:** pay çok dar (ham 4,8 KB). İzleyici yoluna eklenecek küçük bir kod bile tavanı aşar.
  `DECISION.md`'deki kalıcı çözüm önerisi (kabuk paketinde `wasm-opt` ya da derleme ayarı)
  RF1 kapanışında Öneriler'e öncelikli girmeli.

## 7. D2 iddiası

- `grep -rnE 'Document\s*\{' crates/qp-authority/src`: yalnız dönüş tipleri (`-> Document {`,
  `-> &Document {`); **yapı değişmezi 0**.
- `ChangeOp`/`Collection` eşlemeleri, en az 3 kollu `match`'ler betikle tarandı: 7 eşleme
  (`capsule/public.rs:359`, `capsule/project.rs:317`, `capsule/settled.rs:159`,
  `capsule/public/commit.rs:50`, `authority/session/accept.rs:255`, `data/live.rs:58` ve `:76`).
  Hepsinde joker kol (`_ =>` / `other =>`) var. `Collection::` otoritede tek kullanım, `match`
  değil.
- Tek yumuşak dosya `capsule/public.rs`'teki kimlik zinciri (derleyici zorlamaz).
- **0 zorunlu + 1 yumuşak iddiası doğru.** `slice`'ın `Document { … }` değişmezi artık
  `qp-viewer/src/viewer.rs`'te; ARCHITECTURE D2 tablosu bunu gösteriyor.

## Bulgular

| # | Tür | Bulgu | Öneri |
|---|---|---|---|
| Y1 | yumuşak (tasarım) | Doküman ve günlüğe girişsiz yazan gizli yöntemler (`follow_start`, `mirror_apply`, `mirror_commit`, `follow_data`, `follow_resync`, `take_provider`) her `Session`'da ve `DerefMut` ile her barındırıcıda çağrılabilir; komut yolu değişmezini yalnız gelenek koruyor. Bugün tek çağıran `qp-replica`. | `Follower(Session)` tipi (§2.2 a) ya da kurucu ve yansıtma birleştirme (b) ya da `arch-check` çağıran kuralı (c); REPORT "Öneriler" |
| Y2 | yumuşak (belge) | Görüntüleyicisiz otoritede `viewer` aboneliği sessizce `subscribed` (denetim yok, itme yok); `rewritten.md` yalnız `visible` farkını yazıyor | `rewritten.md` S3.3'e bir satır ya da kayıtsız itilen konuya `unsupported` |
| Y3 | yumuşak (belge) | `ViewerConn` yuvası artık `Phase::Subscribe`'dan önce kuruluyor ve `Phase::Unsubscribe` içinde kayıt sırasıyla siliniyor (eskiden onaydan sonra kurulup fazdan önce siliniyordu); bugün gözlenebilir değil | `qp-viewer` README'sine bir satır |
| N1 | not | Kabuk WASM payı İ8 tavanına 4.801 B | `wasm-opt` önerisinin önceliği |
| N2 | not | İ3 ölçülen = tavan 12.744; 12.000 hedefi tutmadı, istisna sürüyor (karara uygun) | — |

## Temizlik

Worktree `.claude/worktrees/audit-s33` ve `target/wt/audit-s33` (WASM paketleri ve
`refactor-compare-*` dahil) denetimden sonra kaldırıldı. Geçici dosyalar
(scratchpad `verify.log`) silindi. Ana ağaca yalnız bu dosya yazıldı (commit edilmedi).
