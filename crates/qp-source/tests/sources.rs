//! File and synthetic sources; a live (revised) stream drives a study to the same bits
//! as a batch run over the final bars.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_compute::{create, run, Field};
use qp_source::{
    apply, collect_bars, collect_series, open_binding, synthetic_bars, BarEvent, FileSource,
    MemorySource, SeriesMeta, SeriesSource, SyntheticSource,
};

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

#[test]
fn synthetic_generator_reproduces_the_sample_fixture() {
    let mut f = FileSource::open(&repo("fixtures/bars/sample.json")).unwrap();
    assert_eq!(f.meta().id, "sample");
    let from_file = collect_bars(&mut f).unwrap();
    assert_eq!(from_file, synthetic_bars(200, 20_251_114));
}

#[test]
fn file_source_reads_a_document_series() {
    let mut f =
        FileSource::open_in_document(&repo("fixtures/schema/document.example.json"), "thyao-1d")
            .unwrap();
    assert_eq!(f.meta().instrument.as_ref().unwrap().symbol, "THYAO");
    assert_eq!(collect_bars(&mut f).unwrap().len(), 3);
    assert!(
        FileSource::open_in_document(&repo("fixtures/schema/document.example.json"), "nope")
            .is_err()
    );
    assert!(FileSource::open(&repo("fixtures/does-not-exist.json")).is_err());
}

#[test]
fn ticked_synthetic_source_converges_to_the_final_bars() {
    for ticks in [1, 2, 5] {
        let mut s = SyntheticSource::new(300, 9, ticks);
        let mut news = 0;
        let mut bars = Vec::new();
        while let Some(ev) = s.next_event().unwrap() {
            news += usize::from(matches!(ev, BarEvent::New(_)));
            apply(&mut bars, ev).unwrap();
            qp_schema::validate_bars(&bars).unwrap();
        }
        assert_eq!(news, 300);
        assert_eq!(bars, synthetic_bars(300, 9));
    }
    let series = collect_series(&mut SyntheticSource::new(10, 1, 3)).unwrap();
    assert_eq!(series.bars().unwrap().len(), 10);
}

#[test]
fn live_stream_drives_a_study_to_batch_bits() {
    let bars = synthetic_bars(400, 3);
    for name in qp_compute::STUDY_NAMES {
        let mut batch_study = create(name, &BTreeMap::new(), Field::Close).unwrap();
        let close = Field::Close.column(&bars);
        let batch = run(batch_study.as_mut(), &[&close]).unwrap();

        let mut study = create(name, &BTreeMap::new(), Field::Close).unwrap();
        let mut out = vec![None; study.outputs().len()];
        let mut live: Vec<Vec<Option<f64>>> = vec![Vec::new(); out.len()];
        let mut src = SyntheticSource::new(400, 3, 4);
        let mut pending = false;
        while let Some(ev) = src.next_event().unwrap() {
            match ev {
                BarEvent::New(b) => {
                    if pending {
                        for (c, v) in live.iter_mut().zip(&out) {
                            c.push(*v);
                        }
                    }
                    study.push(&[b.c], &mut out);
                    pending = true;
                }
                BarEvent::UpdateLast(b) => study.update_last(&[b.c], &mut out),
            }
        }
        for (c, v) in live.iter_mut().zip(&out) {
            c.push(*v);
        }
        let bits = |v: &Vec<Vec<Option<f64>>>| -> Vec<Vec<Option<u64>>> {
            v.iter()
                .map(|c| c.iter().map(|x| x.map(f64::to_bits)).collect())
                .collect()
        };
        assert_eq!(bits(&live), bits(&batch), "{name}");
    }
}

#[test]
fn ordering_rules_and_bindings() {
    let b = synthetic_bars(2, 1);
    let mut v = Vec::new();
    assert!(apply(&mut v, BarEvent::UpdateLast(b[0].clone())).is_err());
    apply(&mut v, BarEvent::New(b[0].clone())).unwrap();
    assert!(apply(&mut v, BarEvent::New(b[0].clone())).is_err());
    assert!(apply(&mut v, BarEvent::UpdateLast(b[1].clone())).is_err());

    let mut m = MemorySource::new(SeriesMeta::default(), b.clone());
    assert_eq!(collect_bars(&mut m).unwrap(), b);

    let doc: qp_schema::Document = serde_json::from_str(
        &std::fs::read_to_string(repo("fixtures/schema/document.example.json")).unwrap(),
    )
    .unwrap();
    // `matriks` binding: not available in R1 (R6).
    assert!(open_binding(&doc.series[0], &repo("fixtures")).is_err());
    let file_binding = qp_schema::Series {
        id: "s".into(),
        instrument: None,
        interval: None,
        source: qp_schema::SeriesSource::File {
            path: "bars/sample.json".into(),
        },
        data: None,
    };
    let mut src = open_binding(&file_binding, &repo("fixtures")).unwrap();
    assert_eq!(collect_bars(src.as_mut()).unwrap().len(), 200);
}

#[test]
fn file_bindings_cannot_escape_the_base_dir() {
    let binding = |path: &str| qp_schema::Series {
        id: "s".into(),
        instrument: None,
        interval: None,
        source: qp_schema::SeriesSource::File { path: path.into() },
        data: None,
    };
    // Base `fixtures/schema`; every target below is a *valid* series file, so only the
    // path rule can reject it.
    let base = repo("fixtures/schema");
    let abs = repo("fixtures/bars/sample.json").canonicalize().unwrap();
    let abs = abs.to_str().unwrap();
    assert!(
        FileSource::open(Path::new(abs)).is_ok(),
        "target itself is valid"
    );
    for bad in [
        "../bars/sample.json",
        "../schema/../bars/sample.json",
        abs,
        "",
    ] {
        let err = open_binding(&binding(bad), &base).err();
        let msg = err.map(|e| e.to_string()).unwrap_or_default();
        assert!(
            msg.contains("plain relative path") || msg.contains("outside"),
            "{bad:?} must be rejected by the path rule, got {msg:?}"
        );
    }
    // Inside the base it works.
    let mut ok = open_binding(&binding("bars/sample.json"), &repo("fixtures")).unwrap();
    assert_eq!(collect_bars(ok.as_mut()).unwrap().len(), 200);
    // A directory is not a regular file.
    assert!(FileSource::open(&repo("fixtures/bars")).is_err());
    assert!(qp_source::resolve_binding_path(&repo("fixtures"), "bars/sample.json").is_ok());
}

/// A symlink inside the base dir that points outside it is rejected (canonical check).
#[cfg(unix)]
#[test]
fn symlink_escape_is_rejected() {
    let dir = std::env::temp_dir().join(format!("qp-source-symlink-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let link = dir.join("link.json");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(repo("fixtures/bars/sample.json"), &link).unwrap();
    let r = qp_source::resolve_binding_path(&dir, "link.json");
    std::fs::remove_dir_all(&dir).unwrap();
    let msg = r.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(msg.contains("outside"), "{msg:?}");
}
