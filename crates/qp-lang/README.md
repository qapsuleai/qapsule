# qp-lang

**Hesap dili** (`qp-ir/0`, `docs/PROGRAMS.md`; kavramlar `docs/CONCEPTS.md` "Hesap dili
(RL1)"). Bu crate'te bugün: **denetim** (`check`), hata modeli, statik maliyet, program ve
örnek kimliği, dilin sınırları. Çalıştırıcı (RL1 LB) ve metin dil (L) sonra gelir.
Denetim ham JSON'u okur ve bütün hataları birden döndürür (ilk hatada durmaz): Qapsule'ün
kapalı kodu + alt kod (`E_…`/`W_…`), JSON işaretçisi, `expected`, `did_you_mean`, `hint`,
iki dilde ileti. Hatasız programda çalıştırıcıya çözülmüş, tiplenmiş, saatlenmiş,
parametreleri bağlanmış bir program (`Checked`) verir.

**Katman:** alan, sıranın başı (`docs/PROGRAMS-SESSION.md` §5; yalnız şema ve çekirdeğe bağlı).

**Bağımlılıklar:** `qp-schema` (ara gösterim tipleri, `ProgramError`, `ProgramCheck`,
kayıt defteri), `qp-calendar` (`Interval`), `serde_json`. Yeni dış bağımlılık yok.

**Dışa açık yüzey (`lib.rs`):**

- `check(raw, &CheckCtx) -> CheckReport`, `check_program`; `CheckCtx` (`limits`,
  `binding: Option<Binding>`, `params`, `lang`, `purpose`), `Binding`, `Purpose`
  (`Document` = `add_program`: sinyal/durum `E_TRY_ONLY`); `CheckReport::to_answer()` →
  `ProgramCheck`.
- `Checked`, `CheckedNode`, `CheckedOutput`, `Op` (+ `WindowKind`, `BinaryKind`,
  `UnaryKind`, `CondKind`, `Literal`, `Anchor`), `OutputSpec`, `Rule`, `Slot`, `Ty`,
  `Clock`, `Tf` — çalıştırıcının okuduğu çözülmüş program.
- `Limits`, `Limit` — RL1 kayıt defteri anahtarları; `Limits::from_params(|key| …)`.
- `check_live_budget`, `LiveUsage` — kişi başı canlı bütçe (`E_BUDGET`, PR §8.2).
- `program_id`, `instance_id`, `sha256`, `sha256_hex`.
- `Reason` (alt kod ↔ kapalı kod tablosu), `Lang`, `did_you_mean`, `suggest`,
  `edit_distance`, `is_node_id`, `is_lower_name`, `OP_NAMES`, `OUTPUT_KINDS`,
  `BAR_FIELDS`, `TURKISH_NAMES`.

**Düzen (`src/`):** `check/` denetimin adımları (kök, parametreler, düğüm alanları, tip ve
saat, çıktılar, maliyet, uyarılar); kökte hata modeli (`reason`, `message`, `diag`),
`checked` (çıktı tipleri), `limits`, `budget`, `identity`, `sha256`, `names`.

**Ne değildir:** çalıştırıcı değildir (bar itmez, değer hesaplamaz); metin ayrıştırmaz;
oturuma, protokole ya da dokümana dokunmaz (`E_NOT_BAR_SERIES`, `E_PROGRAM_SERIES`,
`E_EVENT_OPERAND` `qp-program`'ın; burada yalnız `Reason` eşlemesi var).

**Kurallar:** hesap kodunda model, ağ, saat, rastgelelik yok; aynı girdi → aynı rapor.
Hiçbir sınır koda gömülmez (`limits.rs`, PR §8.4); dil sabitleri (ağırlıklar, kenar 1…8,
boyut 1…4) parametre değildir. SHA-256 ve en yakın ad kuralı burada yeniden yazılmıştır:
`qp_capsule::trust::seal` ve `qp_plane::suggest` bu katmanın üstündedir.
