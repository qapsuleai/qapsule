# RP3/M15b denetimi — görüntüleyici (K48)

Denetçi: bağımsız alt ajan (işi yapmadı). Dal `rp3/m15b-viewer`, son commit `cd232db`.
Çalışma: ayrı karalama worktree'si (`cd232db`, ayrıldı ve silindi); derleme hedefi işi
yapan ajanın worktree hedefi. Portlar 8500–8503. `scripts/verify.sh` koşulmadı (birleştirmede
ana oturum koşar).

## Karar: **PASS** (koşulsuz; aşağıda öneriler)

## 1. Kapsam ve değişmezler — geçti

- `git diff 53d2fb7..rp3/m15b-viewer --stat`: 32 dosya. `fixtures/` altında yalnız yeni
  `fixtures/protocol/conformance/59-viewer.json`; var olan döküm ya da `fixtures/golden/**`
  değişmedi. `qp-compute`, `qp-facts`, `qp-capsule`, `fixtures/compute` dokunulmadı.
- Hesap özeti aynı: `digest` örneği `7b1e99d40889f83c` (= `fixtures/compute/digest.txt`),
  r3 `914fcfdb9f653a6d` (= `digest-r3.txt`).
- İki test sabiti: `describe` konuları şemadan türetiliyor (`query.rs`:
  `variant_names(&t_schema, "kind")`), `Topic::Viewer` eklenince `"viewer"` listeye giriyor.
  Deneme: karalama kopyasında `viewer` describe'dan süzüldü ve
  `capsule_perf_equality.rs` 53d2fb7 hâline (FIX-E1 sabitleri) döndürüldü →
  `bound_with_flow_and_readers_unchanged` ve `inline_unchanged` **ok**. Yani değişikliğin tek
  nedeni describe; aritmetik de tutuyor (a: +18 bayt = 2×9; satırlar +90 = 10×9; günlük ve
  doküman özeti aynı). Bu deneme aynı zamanda `refresh_headlines` kirli-küme
  iyileştirmesinin çıktıyı değiştirmediğini doğruluyor. `queries.rs` 9→10 aynı neden.
  Deneme geri alındı.
- `node shells/web/arch-check.mjs`: 0 ihlal, öz-sınama 22/22. Eklenen `visible`, `viewer`,
  `check` ARCH.md'de gerekçeli (görünürlük bildirimi; ölçüm düzeneği). `main.js` değişikliği
  ince: görünürlüğü çekirdeğe iletir, güven metnini çekirdekten alıp yazar.

## 2. K48 anlamı — geçti (koddan okundu, ölçümle doğrulandı)

- Barındırıcı (`demo.rs`): `mode=viewer` (varsayılan) `start` çerçevesi `feeds: []`,
  `events: []`, yalnız sembol listesi ve yetkiler. `data` çerçeveleri istemcinin `served`
  kümesine süzülüyor; yeni abonelikte `bind` yalnız o beslemenin ilk barları + olayları
  (`ReplayHost::feed_data`). Geri sarmada (epoch) `served` sıfırlanıp yeniden bağlanıyor.
- Çekirdek (`qp-wasm/src/shell.rs`, `viewer.rs`): viewer kipinde yalnız `viewer` konusuna
  abone olunuyor (`capsules` konusu ve akış sorgusu yok), `replica.on_message` çağrılmıyor;
  veri olayı yalnız `data` listesindeki beslemeler için ve yalnız açık tarafa gidiyor.
  Akış satırları ve yüzler otoriteden (`ViewerFace.trust = Server`); cihaz yüz derlemiyor.
- Otorite (`qp-session/src/viewer.rs`): `data` = açık kapsülün dilimindeki bağlı seriler ∩
  okuyucu yetkisi; kapalıyken boş + `closed`. İddialar viewer aboneliğine yalnız bütün
  beslemeleri açık enstrümanınkiyse gidiyor (`viewer_claim_sub`).
- Açık kapsül: `OpenSide` dilimden bir yansı kurar (yalnız o enstrüman); `check_story`
  açık dedektörleri cihazın kapalı barlarında çalıştırıp hikâyenin sunucu olmayan
  olgularını kimlik + kanıtla karşılaştırıyor; itirazlı iddia → "Kaynaklar farklı".
- Kapatınca abonelik boşalıyor (parite `back`: `data []`; döküm 59 `closed`; `bound`
  kaldırılıyor).
- Güven işareti ekranda metinle ayrışıyor: akış satırı "Sunucu doğruladı" (`.face .trust`),
  açık katman "Bu cihazda sınandı" (`#trust`). Tüm evreni ya da tüm geçmişi viewer'a taşıyan
  bir yol bulamadım.

## 3. Testler — geçti

- `cargo test -p qp-wasm --features replica --test viewer_m15`: 3 geçti, 1 yok sayıldı
  (ölçüm).
- `cargo test -p qp-session`: bütün ikililer ok, 0 başarısız (replica viewer testi ve
  `dispute.rs` dahil).
- `qp replica-tape --viewer --lag 0`: 186 adım, 0 farklı özet, 0 ihlal, kesin 4/4, öteki
  enstrümana abone adım 0. `--lag 25`: 211 adım, 0 ihlal (yanlış/erken gösterim 0), 4/4,
  öteki enstrüman 0 (210 adımda özet farkı gecikmenin beklenen sonucu). Mevcut
  `replica-tape --lag 0/25`: geçti.
- `qp conform fixtures/protocol/conformance` × inproc / serve / mcp: 59/59 her birinde
  (`59-viewer` mcp: 1 hello, 5 araç çağrısı, 4 doğrudan).

## 4. Bağımsızlık ölçümü (yeniden üretim) — geçti

`viewer-measure.mjs`, başsız Chromium, 390×844 DPR 3, 9+9 adım, 2 tekrar (30/300).
Makine paylaşımlı (yük ~8–19).

| N | Kip | Tik işi akış (ms/adım) | Tik işi açık | WASM (MB) | Başlangıç (KB) | Akışta `data` / bar | Açıkken `data` / bar | Güven |
|---:|---|---:|---:|---:|---:|---|---|---|
| 30 | viewer | 1,76 / 1,84 | 5,77 / 6,31 | 16,4 / 13,8 | 0,5 | `[]` / 0 | 1 besleme / 121–125 | Bu cihazda sınandı |
| 300 | viewer | 1,65 / 1,77 | 5,48 / 5,79 | 16,4 / 16,4 | 2,9 | `[]` / 0 | 1 besleme / 121–125 | Bu cihazda sınandı |
| 30 | replica | 2,98 / 2,81 | 8,14 / 6,87 | 27,9 / 28,7 | 366–523 | 30 / 3.660+ | 30 | — |
| 300 | replica | 9,00 / 9,12 | 18,99 / 18,86 | 172,7 / 178,9 | 3.659–5.229 | 300 / 36.600+ | 300 | — |

3.000, yalnız viewer, 1 tekrar (8503): tik işi akış 1,70, açık 5,51 ms/adım; WASM 16,6 MB;
başlangıç 29,2 KB (sembol listesi); akışta `data []` / 0 bar; açıkken 1 besleme (SYN1893)
/ 121 bar; güven "Bu cihazda sınandı"; açılış → ilk sahne 715 ms (sunucu tarafı); sayfa
hatası 0. 30 → 3.000: tik işi akışta −%3…−%8, açıkken −%5…−%13, WASM 16,4 → 16,6 MB.

Viewer 30→300: tik işi ±%10 içinde, WASM 13,8–16,4 MB (tek sayfa büyüme adımı; en kötü
eşleşme +%19, sınırda ama içinde), adım başına bayt ~7 KB / ~6 KB sabit. Tam yansı
büyüyor (tik 3→9 ms, WASM 28→175 MB). Yazarın tablosuyla uyumlu.

## 5. Parite — geçti

- Yerel: `viewer_faces_and_open_scene_equal_the_full_replica` (her adımda yüz, açık sahne
  ve bakış özetleri tam yansı sürücüsününkiyle eşit; akış/geri `data []`, açıkken 1
  besleme, güven `device`) ve `viewer_core_equals_viewer_driver` geçti.
- Tarayıcı: `QP_DEMO_PORT=8502 node tests/parity.mjs` çıkış 0: replica 7/7 eşit,
  viewer 7/7 eşit, "viewer = replica" 7/7 eşit; ağ yalnız kendi kaynağı. (İlk koşuda tek
  404 benim karalama paketimde `pkg/sizes.json` eksikliğindendi; eklenince temiz.)
- Not: yüz paritesi kısmen yapısal (otorite yüzü yansının kullandığı aynı `face_scene`
  ile derliyor); açık sahne paritesi ise anlamlı — cihaz onu dilim + kendi barlarından
  derliyor. Tarayıcı viewer yüz karşılaştırması alt küme denetimi (ekrandaki yüzler).

## Öneriler (engelleyici değil)

- `verify.sh` `replica` adımına `qp replica-tape --viewer --lag 0/25` eklensin (şimdi yalnız
  `cargo test` içinde).
- `drive::viewer_frames`, `demo.rs` `Hub::broadcast/bind` mantığının ayrı bir kopyası;
  ileride ayrışabilir — ortak bir fonksiyona taşınması.
- `demo.rs`'de `broadcast` artık `route`'tan önce (replica kipini de etkiler: veri iddiadan
  önce gelir). Parite eşit; birleştirmede `demo` adımının altın görselleri bunu ayrıca
  doğrular.
- Güven işaretleri yalnız metinle ayrışıyor (aynı soluk stil); R4/R5 tasarımında görsel
  ayrım düşünülebilir.
- `check_story` dilimde bulunmayan olguları sessizce atlıyor; hepsi yoksa `Server` döner —
  dilim bir olguyu kaçırırsa "sınandı" yerine "sunucu" görünür (güvenli yönde).
