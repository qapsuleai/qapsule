//! Columnar binary frame ("qcol"): f64/i64 little-endian blocks, 8-byte aligned so a
//! JS shell can view them as Float64Array / BigInt64Array without copying.
//! Null in `values` = validity bitmap (Arrow-style), not NaN.

use std::collections::BTreeMap;

use crate::schema::{Bar, ChangeOp, Event, QueryResult, SeriesValues, ServerMsg};
use crate::{e, R};

const K_SET_LAST: u8 = 1;
const K_APPEND: u8 = 2;
const K_VALUES: u8 = 3;

pub struct W(pub Vec<u8>);
impl W {
    pub fn u8(&mut self, x: u8) {
        self.0.push(x)
    }
    pub fn u32(&mut self, x: u32) {
        self.0.extend_from_slice(&x.to_le_bytes())
    }
    pub fn u64(&mut self, x: u64) {
        self.0.extend_from_slice(&x.to_le_bytes())
    }
    pub fn str(&mut self, s: &str) {
        self.0.extend_from_slice(&(s.len() as u16).to_le_bytes());
        self.0.extend_from_slice(s.as_bytes());
    }
    fn align8(&mut self) {
        while self.0.len() % 8 != 0 {
            self.0.push(0)
        }
    }
    fn f64s(&mut self, it: impl Iterator<Item = f64>) {
        self.align8();
        for x in it {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
    }
    fn i64s(&mut self, it: impl Iterator<Item = i64>) {
        self.align8();
        for x in it {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
    }
}

pub struct Rd<'a>(pub &'a [u8], pub usize);
impl<'a> Rd<'a> {
    pub fn take(&mut self, n: usize) -> R<&'a [u8]> {
        let s = self.0.get(self.1..self.1 + n).ok_or("qcol: truncated")?;
        self.1 += n;
        Ok(s)
    }
    pub fn u8(&mut self) -> R<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn u32(&mut self) -> R<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn u64(&mut self) -> R<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn str(&mut self) -> R<String> {
        let n = u16::from_le_bytes(self.take(2)?.try_into().unwrap()) as usize;
        String::from_utf8(self.take(n)?.to_vec()).map_err(e)
    }
    fn align8(&mut self) {
        self.1 = self.1.div_ceil(8) * 8;
    }
    fn f64s(&mut self, n: usize) -> R<Vec<f64>> {
        self.align8();
        Ok(self
            .take(n * 8)?
            .chunks_exact(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }
    fn i64s(&mut self, n: usize) -> R<Vec<i64>> {
        self.align8();
        Ok(self
            .take(n * 8)?
            .chunks_exact(8)
            .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }
    pub fn end(&self) -> R<()> {
        if self.1 == self.0.len() {
            Ok(())
        } else {
            Err("qcol: trailing bytes".into())
        }
    }
}

pub fn write_bars(w: &mut Vec<u8>, bars: &[Bar]) {
    let mut ww = W(std::mem::take(w));
    ww.u32(bars.len() as u32);
    ww.i64s(bars.iter().map(|b| b.t));
    ww.f64s(bars.iter().map(|b| b.o));
    ww.f64s(bars.iter().map(|b| b.h));
    ww.f64s(bars.iter().map(|b| b.l));
    ww.f64s(bars.iter().map(|b| b.c));
    ww.f64s(bars.iter().map(|b| b.v));
    *w = ww.0;
}
pub fn read_bars(r: &mut Rd) -> R<Vec<Bar>> {
    let n = r.u32()? as usize;
    let t = r.i64s(n)?;
    let (o, h, l, c, v) = (r.f64s(n)?, r.f64s(n)?, r.f64s(n)?, r.f64s(n)?, r.f64s(n)?);
    Ok((0..n)
        .map(|i| Bar {
            t: t[i],
            o: o[i],
            h: h[i],
            l: l[i],
            c: c[i],
            v: v[i],
        })
        .collect())
}

pub fn enc(m: &ServerMsg) -> R<Vec<u8>> {
    let mut w = W(Vec::new());
    match m {
        ServerMsg::Notify {
            sub,
            event:
                Event::Change {
                    seq,
                    at,
                    author,
                    approved_by: None,
                    cause: None,
                    changes,
                },
        } if changes.len() == 1 => match &changes[0] {
            ChangeOp::SetLastBar { series, bar } => {
                w.u8(K_SET_LAST);
                w.str(sub);
                w.str(author);
                w.str(series);
                w.u64(*seq);
                w.u64(*at as u64);
                w.u64(bar.t as u64);
                for x in [bar.o, bar.h, bar.l, bar.c, bar.v] {
                    w.0.extend_from_slice(&x.to_le_bytes());
                }
            }
            ChangeOp::AppendBars { series, bars } => {
                w.u8(K_APPEND);
                w.str(sub);
                w.str(author);
                w.str(series);
                w.u64(*seq);
                w.u64(*at as u64);
                write_bars(&mut w.0, bars);
            }
            _ => return Err("qcol: unsupported change".into()),
        },
        ServerMsg::Result {
            id,
            seq,
            data: QueryResult::Values { values },
        } => {
            let n = values.t.len();
            w.u8(K_VALUES);
            w.str(id);
            w.str(&values.series);
            w.u64(*seq);
            w.u64(values.from_bar);
            w.u32(n as u32);
            w.u32(values.outputs.len() as u32);
            w.i64s(values.t.iter().copied());
            for (name, vs) in &values.outputs {
                if vs.len() != n {
                    return Err("qcol: output length".into());
                }
                w.str(name);
                let mut bm = vec![0u8; n.div_ceil(8)];
                for (i, x) in vs.iter().enumerate() {
                    if x.is_some() {
                        bm[i / 8] |= 1 << (i % 8);
                    }
                }
                w.0.extend_from_slice(&bm);
                w.f64s(vs.iter().map(|x| x.unwrap_or(0.0)));
            }
        }
        _ => return Err("qcol: unsupported message".into()),
    }
    Ok(w.0)
}

pub fn dec(b: &[u8]) -> R<ServerMsg> {
    let mut r = Rd(b, 0);
    let m = match r.u8()? {
        K_SET_LAST | K_APPEND => {
            let k = b[0];
            let (sub, author, series) = (r.str()?, r.str()?, r.str()?);
            let (seq, at) = (r.u64()?, r.u64()? as i64);
            let change = if k == K_SET_LAST {
                let t = r.u64()? as i64;
                let mut f = [0.0; 5];
                for x in &mut f {
                    *x = f64::from_le_bytes(r.take(8)?.try_into().unwrap());
                }
                ChangeOp::SetLastBar {
                    series,
                    bar: Bar {
                        t,
                        o: f[0],
                        h: f[1],
                        l: f[2],
                        c: f[3],
                        v: f[4],
                    },
                }
            } else {
                ChangeOp::AppendBars {
                    series,
                    bars: read_bars(&mut r)?,
                }
            };
            ServerMsg::Notify {
                sub,
                event: Event::Change {
                    seq,
                    at,
                    author,
                    approved_by: None,
                    cause: None,
                    changes: vec![change],
                },
            }
        }
        K_VALUES => {
            let (id, series) = (r.str()?, r.str()?);
            let (seq, from_bar) = (r.u64()?, r.u64()?);
            let (n, k) = (r.u32()? as usize, r.u32()? as usize);
            let t = r.i64s(n)?;
            let mut outputs = BTreeMap::new();
            for _ in 0..k {
                let name = r.str()?;
                let bm = r.take(n.div_ceil(8))?.to_vec();
                let vs = r.f64s(n)?;
                let col = vs
                    .into_iter()
                    .enumerate()
                    .map(|(i, x)| (bm[i / 8] >> (i % 8) & 1 == 1).then_some(x))
                    .collect();
                outputs.insert(name, col);
            }
            ServerMsg::Result {
                id,
                seq,
                data: QueryResult::Values {
                    values: SeriesValues {
                        series,
                        from_bar,
                        t,
                        outputs,
                    },
                },
            }
        }
        _ => return Err("qcol: unknown kind".into()),
    };
    r.end()?;
    Ok(m)
}
