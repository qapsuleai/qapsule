# qp-cli

`qp` komut satırı: ajanlar, testler ve geliştirici için tek giriş. **Kabuk** katmanı
(`docs/ENGINEERING.md` §2): yalnız bağlama. `clap` ile komut ağacını tanımlar, her komutu alt
crate'lerdeki işe bağlar; alan kararı vermez.

## Ne var
- `src/main.rs`: `Cli` / `Command` (`#[command(name = "qp", version)]`), argüman yapıları,
  komutların dağıtımı.
- `src/cmd/`: komut başına bağlama dosyaları (`replay`, `scene`, `compute`, `schema`, `serve`).
- Barındırma komutları (`demo`, `drive`, `sim`, `live`, `mcp serve`, `replay --raw`) RG1'de
  `qp-host`'a taşındı; burada yalnız `use qp_host::{demo, live, mcp_serve, sim}` ile çağrılır.
- `tests/`: `qp` ikilisini koşan bütünleşik testler (`assert_cmd`).

## Bağımlılıklar
Normal: `qp-host`, `qp-session`, `qp-mcp`, `qp-schema` (`validate`), `qp-render`, `qp-grammar`,
`qp-compute`, `qp-source`, `clap`, `serde_json`. Geliştirme: testlerin kullandığı `qp-live`
(`mock`), `qp-matriks`, `qp-calendar`, `flate2`, `prost`, `assert_cmd`, `tempfile`.

## Dışa açık
Yalnız `qp` ikilisi. Komut satırı arayüzü (`qp --help` ağacı) RG1 tutanağında
(`rounds/RG1/experiments/baseline/`, `cli/`) bayt bayt sabitlenmiştir.

## Ne değildir
Kütüphane değildir; barındırma, protokol, hesap ya da çizim mantığı taşımaz. Kabuk bütçesi
4 bin satır (bugün ≈ 1,4 bin).
