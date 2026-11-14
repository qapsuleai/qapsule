//! Helpers shared by the render-stack spike examples (not part of the library).
#![allow(dead_code)]

use std::path::PathBuf;
use std::time::Instant;

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

pub fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

pub fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

pub fn decode_png(bytes: &[u8]) -> Vec<u8> {
    let mut r = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut buf = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    buf
}

pub fn golden_rgba(name: &str) -> Vec<u8> {
    decode_png(&std::fs::read(root().join("fixtures/golden").join(name)).unwrap())
}

/// (differing pixels, max channel diff, mean abs channel diff over all channels)
pub fn diff(a: &[u8], b: &[u8]) -> (usize, u8, f64) {
    assert_eq!(a.len(), b.len());
    let mut n = 0;
    let mut m = 0;
    let mut sum = 0u64;
    for (x, y) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let mut d = 0;
        for (p, q) in x.iter().zip(y) {
            let ad = p.abs_diff(*q);
            sum += u64::from(ad);
            d = d.max(ad);
        }
        if d > 0 {
            n += 1;
            m = m.max(d);
        }
    }
    (n, m, sum as f64 / a.len() as f64)
}

pub fn write_png(name: &str, rgba: &[u8], w: u32, h: u32) -> PathBuf {
    let dir = root().join("target/spikes");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, qp_render::encode_png(rgba, w, h).unwrap()).unwrap();
    p
}
