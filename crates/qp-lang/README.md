# qp-lang

**Hesap dili** (`qp-ir/0`, `docs/PROGRAMS.md`; kavramlar `docs/CONCEPTS.md` "Hesap dili
(RL1)"). Bu crate'te: **denetim** (`check`), hata modeli, statik maliyet, program ve örnek
kimliği, dilin sınırları ve **çalıştırıcı** (`Executor`: akışkan, canlı barda geçici değer,
üst periyot hizası, köken, çalışma bütçesi) ile `try_program` özeti. Metin dil ayrı crate'te
(`qp-lang-text`).
Denetim ham JSON'u okur ve bütün hataları birden döndürür (ilk hatada durmaz): Qapsule'ün
kapalı kodu + alt kod (`E_…`/`W_…`), JSON işaretçisi, `expected`, `did_you_mean`, `hint`,
iki dilde ileti. Hatasız programda çalıştırıcıya çözülmüş, tiplenmiş, saatlenmiş,
parametreleri bağlanmış bir program (`Checked`) verir.

**Katman:** alan, sıranın başı (`docs/PROGRAMS-SESSION.md` §5; yalnız şema ve çekirdeğe bağlı).

**Bağımlılıklar:** `qp-schema` (ara gösterim tipleri, `ProgramError`, `ProgramCheck`,
kayıt defteri, `ProgramTry`, `SignalRecord`), `qp-calendar` (`Interval`, seans hizası),
`qp-compute` (çekirdekler), `serde_json`. Yeni dış bağımlılık yok.

**Dışa açık yüzey (`lib.rs`):**

- `check(raw, &CheckCtx) -> CheckReport`, `check_program`; `CheckCtx` (`limits`,
  `binding: Option<Binding>`, `params`, `lang`, `purpose`), `Binding`, `Purpose`
  (`Document` = `add_program`: sinyal/durum `E_TRY_ONLY`); `CheckReport::to_answer()` →
  `ProgramCheck`.
- `Checked`, `CheckedNode`, `CheckedOutput`, `Op` (+ `WindowKind`, `BinaryKind`,
  `UnaryKind`, `CondKind`, `Literal`, `Anchor`), `OutputSpec`, `Rule`, `Slot`, `Ty`,
  `Clock`, `Tf` — çalıştırıcının okuduğu çözülmüş program.
- `Limits`, `Limit` — RL1 kayıt defteri anahtarları (metin dil sınırları dahil);
  `Limits::from_params(|key| …)`, `Limit::def()` kayıt satırı.
- `check_live_budget`, `LiveUsage` — kişi başı canlı bütçe (`E_BUDGET`, PR §8.2).
- `program_id`, `instance_id`, `sha256`, `sha256_hex`.
- Çalıştırıcı: `Executor` (`new`, `run_closed`, `push`, `update_last`, `push_closed`,
  `close_live`; imleç `value_at`, `doc_value_at`, `events_at`, `signals_at`, `state_at`,
  `lineage`, `provenance`, `record_at`, `closed_len`, `t_at`, `index_of`, `known_at`, `work`,
  `stopped`, `dropped`, `canonical`), `ExecOptions`, `Align`, `WorkBudget`, `Stopped`,
  `ExecError`, `Lineage`, `SignalHit`; `try_program`, `try_window`, `TryRequest`,
  `RecordCtx`; `program_digest`, `DIGEST_SUITE`.
- `Reason` (alt kod ↔ kapalı kod tablosu), `Lang`, `did_you_mean`, `suggest`,
  `edit_distance`, `is_node_id`, `is_lower_name`, `MAX_SUGGEST`, `MAX_CANDIDATES`, `OP_NAMES`, `OUTPUT_KINDS`,
  `BAR_FIELDS`, `TURKISH_NAMES`.

**Düzen (`src/`):** `check/` denetimin adımları (kök, parametreler, düğüm alanları, tip ve
saat, çıktılar, maliyet, uyarılar); `exec/` çalıştırıcının adımları (plan, düğüm adımı,
çekirdekler, üst periyot, akışkan çekirdek, kayıtlar, imleç, `dene` özeti, özet);
kökte hata modeli (`reason`, `message`, `diag`), `checked` (çıktı tipleri), `limits`,
`budget`, `identity`, `sha256`, `names`.

**Ne değildir:** metin ayrıştırmaz; oturuma, protokole ya da dokümana dokunmaz: barları,
sahibi, bağlı seriyi ve beslemeleri çağıran verir (`since_t` kuralını `doc_value_at`
uygular). `E_NOT_BAR_SERIES`, `E_PROGRAM_SERIES`, `E_EVENT_OPERAND` `qp-program`'ındır;
burada yalnız `Reason` eşlemesi var.

**Kurallar:** hesap kodunda model, ağ, saat, rastgelelik yok; aynı girdi → aynı rapor.
Hiçbir sınır koda gömülmez: `Limits` değerlerini ve varsayılanlarını kayıt defterinin
`program.*` satırlarından okur (`qp_schema::params::program`, PR §8.4). Dil sabitleri
parametre değildir, değişmeleri dil sürümünü değiştirir: ağırlıklar (`check/cost.rs`),
`check/args.rs`'te `MAX_T` (çizgi zamanı ≤ 2^53), `MAX_EDGES` (kutu kenarı 1…8), `MAX_DIMS`
(durum boyutu 1…4). Kötü niyete karşı iç sınırlar (dilin değil uygulamanın):
`names::MAX_SUGGEST` (64 karakterden uzun ad öneri almaz ve vermez), `names::MAX_CANDIDATES`
(bir öneri araması en çok 1024 aday tarar), `diag::MAX_SEARCHES` (bir denetimde düğüm
kimlikleri üstünde en çok 256 arama), `diag::MAX_MESSAGE` (ileti en çok 512 karakter). Hata
tavanı sırala-sonra-kes'tir: yığın en iyi `program.check.max_errors` hatayı tutar (§11.1). SHA-256 ve en yakın ad kuralı burada yeniden yazılmıştır:
`qp_capsule::trust::seal` ve `qp_plane::suggest` bu katmanın üstündedir.
