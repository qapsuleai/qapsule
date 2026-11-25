# Ölçülen ajanın ön notu (RL1 ajan teyidi)

Bu dosyanın iki işaret arasındaki kısmı, `driver.py start` tarafından yer tutucuları
doldurularak her görevin istem dosyasının başına konur (`{QMCPX}` koşunun komut satırı,
`{CARD}` dil kartı, `{WORKDIR}` koşu klasörü, `{SERIES}` bağlı seri). Örnek:
RX4 `../refs/experiments/rx4/public/TASK-PREAMBLE.md` (kurallar aynı tutuldu; araçlar ve
dil Qapsule'ünkü). İşaretlerin dışı ajana gitmez.

<!-- PREAMBLE-START -->
# Görev ön notu (Qapsule hesap dili)

Sen Qapsule adlı grafik motoruna bağlı bir ajansın. Aşağıdaki görevi, Qapsule'ün hesap
dilinde bir **program** (ya da gerekiyorsa bir **ret**) yazarak çöz. Bu not her görevle verilir.

## Belge ve sınırlar

- Tek belgen: `{CARD}` (hesap dili kartı). Önce onu oku. Başka hiçbir dosyayı okuma: depo,
  `docs/`, kaynak kod, başka klasörler ve internet yasak.
- Yalnız `{WORKDIR}` klasörüne yazabilirsin (program dosyaların orada dursun).

## Veri

- Bağlı seri `{SERIES}`: tek sembol, taban periyot **1 saat**, 3000 **kapanmış** bar. Bar
  `{t, o, h, l, c, v}`; `t` barın açılış zamanı (epoch ms, UTC). Bar indeksi 0'dan başlar.
  Puanlama aynı programı başka bir (görmediğin) veri kümesinde de koşar; programın yalnız bu
  veriye uyması yetmez.
- **Üst periyotlar:** `4h` ve `1d`, saatlik barlardan toplanır (UTC gece yarısına hizalı).
  Görev "4 saatlik" ya da "günlük" dediğinde hesap o periyodun barlarıyla yapılır.
- **Kapanmış bar ilkesi:** bir saatlik bar kendi kapanışında yalnız o ana kadar kapanmış üst
  periyot barlarını görebilir. Henüz kapanmamış bir barın ya da gelecekteki bir değerin
  istenmesi ifade edilemez; böyle bir istek programla değil, kodlu retle karşılanır.

## Araçlar

Qapsule'e **yalnız** şu komutla erişirsin (Bash ile):

`{QMCPX}`

- `<komut> list` araçları listeler; `<komut> schema <araç>` bir aracın girdi şemasını verir.
- `<komut> call <araç> '<json>'` bir aracı çağırır. Programın metnini JSON'a gömmek yerine
  dosyadan verebilirsin: `<komut> call check_program --source-file {WORKDIR}/prog.qp '{"inputs":["{SERIES}"]}'`.
- Kullanacağın araçlar yalnız program araçlarıdır: `check_program`, `try_program`,
  `explain_program`, `add_program`. Her çağrıda `"inputs": ["{SERIES}"]` ver. Başka araç
  çağırma. Onay/ret gibi **karar aracı yoktur**: önerini insan değerlendirir.
- Programı **metin dilde** (`source`) ya da **ara gösterimde** (JSON, `program`) yazabilirsin;
  ikisi aynı motora iner. Seçim senin.
- `check_program` (derleme, tip, maliyet, bütün hatalar birden) ve `try_program` (yalnız bu
  veride kısaltılmış özet) istediğin kadar kullanılabilir. Beklenen çıktı gösterilmez.

## Teslim

- Programını (ya da retini) bir dosyaya yaz (`{WORKDIR}/prog.qp` metin ya da
  `{WORKDIR}/prog.json` ara gösterim) ve **önce `check_program` ile denetle**; hatasızsa
  `<komut> submit <dosya>` ile teslim et.
- `submit` görev başına **en çok 3** kez; **son teslim** nihai sayılır. `submit` teslimi
  kaydeder ve motorun denetim cevabını gösterir.
- **Çıktı adları:** görevde yazılı çıktı adlarını ve türlerini (seri, olay, sinyal, durum
  tanımı) aynen kullan; durum tanımında boyut adları ve sırası, sinyalde yön, sonuç kuralı ve
  kaynak görevde yazıldığı gibidir. Fazladan çıktı puanlanmaz; eksik ya da adı farklı çıktı
  yanlıştır.
- **Eşitlik ve sınırlar:** görev cümlesindeki `<`, `≤`, `>`, `≥` ve "kesin", "dahil", "hariç"
  ifadeleri bağlayıcıdır.
- **Ret:** istek dilde ifade edilemiyorsa ya da edilmemeliyse (geleceğe bakma, sınırsız
  döngü, dilde olmayan gösterge vb.) program yerine **ret** teslim et: kartın §5'teki hata
  kodlarından uygun olanı ve kısa bir gerekçe (`reject <E_KOD> "gerekçe"`). Gereksiz ret de
  yanlış sayılır.
- **Öneri:** görev yalnız seri ve/veya olay çıktısı istiyorsa, son tesliminden sonra aynı
  programı `add_program` ile **bir kez** dokümana öneri olarak ekle (`"inputs": ["{SERIES}"]`).
  Sinyal ya da durum tanımı çıktılı programlar ve retler eklenmez. Öneriyi kabul etmek ya da
  reddetmek insanın işidir.
- Bitince son mesajın yalnız `DONE` olsun.
<!-- PREAMBLE-END -->
