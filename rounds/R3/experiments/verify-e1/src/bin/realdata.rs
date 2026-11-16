//! Gerçek veri (yalnız yerel, `QAPSULE_LOCAL_BARS`): yedi dedektörün toplu tetik sayıları.
//! Sembol adı, fiyat, tarih yazdırmaz; yalnız toplamlar. Veri depoya kopyalanmaz.
use qp_facts::{default_detectors, run_batch};
use qp_source::{load_local_bars, local_bars_dir};

fn main() {
    let dir = local_bars_dir();
    for period in ["1day", "1hour"] {
        let sets = load_local_bars(&dir, period);
        let bars_total: usize = sets.iter().map(|(_, b)| b.len()).sum();
        println!("period {period}: symbols {} bars total {bars_total}", sets.len());
        for k in 0..default_detectors().len() {
            let mut total = 0usize;
            let mut with_any = 0usize;
            let mut id = "";
            for (_, bars) in &sets {
                let mut d = default_detectors().remove(k);
                id = d.id();
                let n = run_batch(d.as_mut(), bars, None).len();
                total += n;
                with_any += usize::from(n > 0);
            }
            let per = if sets.is_empty() { 0.0 } else { total as f64 / sets.len() as f64 };
            println!("  {id:26} total {total:6}  mean/symbol {per:8.1}  symbols with >=1: {with_any}/{}", sets.len());
        }
    }
}
