# R4/H-FP — Yüzün başlık şablonu için arayüz (M8 öncesi)

Dal `r4/h-fp` (`r4/int` b02113a'dan). Görev: `rounds/R4/tasks/H-FP.md`. Hiçbir yüz tasarımı
yazılmadı; yalnız arayüz, düzenekte SPEC E1 ve başvuru ölçülerinin yeniden üretimi.

## Ne yapıldı

1. **Gramer (`qp-grammar`).**
   - `HeadlineParts` (`provider.rs`, kökten `qp_grammar::HeadlineParts`): okuyucunun dilinde
     düz metinler (`instrument`, `event`, `direction_word`, `when`, `moved`,
     `validity_phrase`, `ledger_short`) ve bayraklar (`lang`, `stage`, `validity`,
     `redacted`, `ledger_n` — yetkisizde `None`, `few_cases` — `n < GLYPH_MIN_LEDGER_N`).
   - `DataProvider::headline_parts(finding) -> Option<HeadlineParts>` (varsayılan `None`);
     `MemProvider::with_headline_parts`.
   - `faces::HeadlineFn = fn(&HeadlineParts) -> String`; `FaceDesign.headline:
     Option<HeadlineFn>` (`r3`: `None`); `faces::headline_template`, `faces::headline_text`.
   - **Etkin yüz `CompileOptions.face`** ile bilinir (tema/sonuç stili gibi bir derleme
     ayarı). Yalnız yüz sahnesi derlenirken dolu; o zaman bulgunun `headline` metin alanı
     şablonun çıktısıdır (`layer.rs`, bulgu satırlarının metinleri). Şablon yoksa ya da parça
     yoksa bugünkü cümle. Bilinmeyen ad `invalid`.
2. **Oturum (`qp-session`).**
   - `headparts.rs`: `qp_capsule::realize::parts` → `HeadlineParts`; takvim seçimi;
     `replay_calendar(ReplaySourceKind)`.
   - `Session::reader_parts` (crate içi): `reader_text_with`'in kuralıyla — okuyucunun öyküsü
     (izdüşümlü), görebildiği olgular, aynı geçerlilik (K42/A28), aynı rakamsızlık (K33).
   - `DocProvider::with_parts(session, lang)`; etkileşim yolunda (`interact.rs`) yüzün
     görünümü ve sahne derlemesi `headlines` ile birlikte bunu da kullanır;
     `capsule_scene` açılma derinliği `sentence` iken `CompileOptions.face`'i bağlantının
     `look.face`'inden doldurur (öteki derinliklerde `None`).
   - `Session::item_headline_parts(item, lang)` (açık): yansı için — kapsül öğesinin
     geçerliliği ve rakamsızlığıyla, kendi öyküsü ve barlarıyla.
   - `Session::set_headline_calendar`, `Replica::set_headline_calendar` (yeniden yüklemede
     korunur); `ReplayHost` oturumu kurarken kaynağın takvimini verir.
   - Yeni bağımlılık: `qp-session` → `qp-calendar` (depo içi crate; `ExchangeCalendar` için;
     dış bağımlılık yok).
3. **Yansının sağlayıcısı (`qp-wasm/src/shell.rs`).** `Reader` sarmalayıcısı
   `headline_parts`'ı verir (yalnız şablonlu yüzde hesaplanır); dil, kapsül öğesinin
   `sentence.text`'inin hangi dilde olduğundan (`item_lang`); `face_scene` derlemeye
   `look.face`'i geçirir. Ayrıca `ShellCore::set_replay` ve `replay` cevabı yansıya
   takvimi verir (sağlayıcının dışında iki satır; takvim yansıda başka yoldan bilinemiyor).
4. **Testler.**
   - `qp-grammar` `faces::tests::headline_template_applies_in_its_face_scene_only`: test
     içi örnek şablon (kayda girmez; `UNLISTED` yalnız `cfg(test)`), TR/EN, ≤ 80; yüz yok,
     `r3`, `card` ya da parça yok → cümle; bilinmeyen ad hata; belirleyici.
   - `qp-session` `headparts::tests::reader_parts_by_language_and_audience`: küçük gösterim
     dünyası (8 enstrüman), 60 adım; `arkin` (yetkili) ve `visitor` (yetkisiz) için bütün
     kapsüller TR/EN; örnek şablon ≤ 80, iç terim yok; yetkisizde parçalarda rakam yok,
     `ledger_n = None`; belirleyici.
   - Düzenek `replica_headline_parts_equal_the_authoritys`: S8 gösterim durumlarında yansının
     parçaları otoritenin parçalarıyla TR/EN eşit; anahtar S2 "dün" ise `moved` "dün".
   - Düzenek `s4_few_cases_is_the_products_threshold` (E1).
5. **Düzenek.** SPEC E1: `s8::trust_answer` "az örnek" kovası `n < GLYPH_MIN_LEDGER_N`.
   `s8.json` yeniden üretildi; S4'ü değişen: #4 (n 7: "çoğunlukla tutmadı" → "az örnek"),
   #6 (n 5: "çoğunlukla tuttu" → "az örnek"), #7 (n 5: "çoğunlukla tutmadı" → "az örnek"),
   #12 (n 17: "yarı yarıya" → "az örnek"). Durum seçimi (`n ≥ 5` tercihi, #10 `n < 5`) ve
   bulunan kapsüller aynı. `m8 --face r3 --no-shoot` → `M8/raw/r3/computed.{json,md}` **bayt
   eşit** (S4 hesap ölçüsüne girmez; `r3`'ün şablonu yok). README M8 bölümüne şablon arayüzü,
   "şimdi"/takvim kuralı, sapma 4'ün kapanışı.
6. `docs/CONCEPTS.md` "Başlık yapı taşları" satırı güncellendi (yeni kavram yok).

## Kararlar

- **"Şimdi":** süre ifadelerinin şimdisi serinin **son barının** açılış zamanı (bitmemiş
  olsa da), cümlenin "şimdi"si (son *bitmiş* bar) değil. İlk deneme cümlenin şimdisiyle
  yapıldı: gösterimin günlük serisinde bugünün barı sürerken dünkü olay "bugün" çıkıyordu
  (S8 #4, #10 anahtar "dün", parça "bugün"; #12 saatlik "az önce", anahtar "dün"). Son barla
  S8'deki gün kovaları anahtarla uyuşuyor (aşağıda). Geçerlilik cümleninkiyle aynı bardan.
  Oynatma saatini kullanmak yansıda mümkün değil (yansının oturumunun saati yok); son bar
  otoritede ve yansıda aynı.
- **Takvim:** enstrüman bir yerleşik takvim adlandırıyorsa o; yoksa oturumun takvimi
  (`ReplayHost`: sentetik → `synthetic_calendar()`, yerel → `XIST`; yansı `ReplayState.source`
  ile); yoksa `XIST`. Sentetik barlar gerçek BIST tatillerinde de var; `XIST` ile işlem günü
  sayısı yanlış çıkardı.
- **Etkin yüz derleme ayarıyla** (sağlayıcı sarmalayıcısı yerine): otoritede görünüm kurma ile
  derleme ayrı işlevlerde; derleme ayarı tek yerde (`layer.rs`) uygulanıyor ve `None` iken
  yol bugünküyle aynı.
- `reading.rs`'teki doküman görünümü derlemelerine `with_parts` eklenmedi: orada yüz
  derlenmez (`CompileOptions.face` hep `None`), parçalar okunmaz. `present.rs` yalnız görünüm
  kurar, başlık okumaz.

## Ölçümler

- S8 durumları için (yansı = otorite) TR parçaları, son bar "şimdi"siyle:

  | # | anahtar S2 | `when` | `moved` | `validity_phrase` | `ledger_short` |
  |---|---|---|---|---|---|
  | 1 | bugün | 5 saat önce | 4 saat önce | hâlâ geçerli | 74'te 36 tuttu |
  | 2 | birkaç gün önce | 3 işlem günü önce | 3 işlem günü önce | hâlâ geçerli | 188'de 103 tuttu |
  | 3 | birkaç gün önce | 2 işlem günü önce | cuma günü | artık geçerli değil | 41'de 16 tuttu |
  | 4 | dün | dün | dün | hâlâ geçerli | 7'de 2 tuttu (az örnek) |
  | 5 | bir haftadan önce | 11 Mayıs'ta | 15 Mayıs'ta | gerçekleşti | 104'te 54 tuttu |
  | 6 | bir haftadan önce | 7 Şubat'ta | 10 Şubat'ta | tutmadı | 5'te 4 tuttu (az örnek) |
  | 7 | bir haftadan önce | 27 Mart'ta | 10 Nisan'da | süresi doldu | 5'te 1 tuttu (az örnek) |
  | 9 (yetkisiz) | birkaç gün önce | birkaç işlem günü önce | birkaç işlem günü önce | hâlâ geçerli | yarı yarıya tuttu |
  | 10 | dün | dün | dün | hâlâ geçerli | karnesi yok |
  | 11 | birkaç saat önce | cuma 17:00'de | 2 saat önce | hâlâ geçerli | 65'te 34 tuttu |
  | 12 | dün | dün 16:00'da | dün 17:00'de | hâlâ geçerli | 17'de 9 tuttu (az örnek) |

- Var olan çıktılar: `cargo test --workspace` yeşil (dökümler, altınlar, görünüm altınları
  değişmedi); düzeneğin `r3` `computed.json`'ı bayt eşit. Değişen döküm/görsel yok.

## Açık sorular

1. **S2 ile `when`/`moved`:** anahtarın "ne zaman"ı son geçişin günü; buna denk parça
   `moved`. Bir alternatif doğuşu (`when`) yazarsa #3, #11'de anahtardan ayrılır. Anahtarın
   "birkaç gün önce" kovası takvim günü; parça hafta sonunu aşan tek işlem günü farkını "cuma
   günü" der (#3: anahtar "birkaç gün önce"). Okuyucunun "cuma günü"nü hangi kovaya koyacağı
   belirsiz; düzenek anahtarı değişmedi (SPEC'te yazılı).
2. **#1 "bugün" ile "5 saat önce":** anahtar 3 saatten sonrasını "bugün" sayar; parça saat
   sayısını verir. Uyumlu ama okuyucu "birkaç saat önce"yi seçebilir.
3. **Yetkisiz okuyucu:** yüzü bugün bütün tasarımlarda düz metin (README Açık soru 2); şablon
   orada çizilmiyor. Yansıda yetkisiz okuyucunun barı olmadığından `item_headline_parts` de
   `None` (otoritede bar var; parçalar rakamsız).
4. Yansı takvimi `ReplayState` gelmeden (ilk `replay` cevabı ya da veri çerçevesi) `XIST`'tir;
   gösterim ve düzenek ilk adımdan önce durum aldığından S8'de etkisi yok.

## Önerilen karar

- "Başlık yapı taşları: süre ifadesinin şimdisi serinin son barı (bitmemiş de olsa);
  takvim enstrümanın, yoksa oynatma kaynağının; etkin yüzün başlık şablonu
  `CompileOptions.face` ile yalnız yüz sahnesinde uygulanır."

## Öneriler

- M9 G3'teki "n ≥ 5" güven kovası da ürün eşiğine (`GLYPH_MIN_LEDGER_N`) çekilmeli mi?
  E1 yalnız S4'ü andığı için değiştirilmedi.
- Kapsül öğesine (`CapsuleItem`) okuyucunun dili eklenirse yansı dili cümleden çıkarmak
  zorunda kalmaz (şema/döküm değişikliği; bu görevin dışında).
