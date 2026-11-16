//! Reader directories (SPEC §7): `run_readers.sh` asks [`setup`] for one reader's record
//! directory — `prompt.template.txt` (the exact prompt but for `@DIR@`), `images.json`
//! (`[[source, name]]`: what is copied into the reader's private directory, nothing else)
//! and `meta.json` (no design name; `score` resolves it through `raw/keys/<code>.json`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::io;
use crate::rng::Rng;
use crate::{blind, m10, m8, m9, prompts};

/// What one reader is.
#[derive(Debug, Clone)]
pub struct Reader {
    /// `m8`, `m9`, `m10`.
    pub experiment: String,
    /// Design name.
    pub design: String,
    /// `F`, `H`.
    pub persona: String,
    /// M8 page (1, 2).
    pub page: usize,
    /// Reader number (1-based).
    pub k: u64,
    /// Seed (M10: sequence order).
    pub seed: u64,
    /// Model alias.
    pub model: String,
}

fn code_of(exp: &Path, experiment: &str, design: &str, page: usize) -> Result<String, String> {
    let root = match experiment {
        "m8" => m8::root(exp),
        "m9" => m9::root(exp),
        "m10" => m10::root(exp),
        e => return Err(format!("unknown experiment `{e}`")),
    };
    let key: BTreeMap<String, Value> =
        serde_json::from_str(&io::read(&root.join("raw/key.json"))?).map_err(|e| e.to_string())?;
    key.iter()
        .find(|(_, v)| {
            v["design"] == design && (experiment != "m8" || v["page"].as_u64() == Some(page as u64))
        })
        .map(|(c, _)| c.clone())
        .ok_or(format!(
            "no {experiment} pages for `{design}` (build them first)"
        ))
}

/// Prepares reader `r`'s record directory; returns it.
///
/// # Errors
/// No pages for the design, a prompt with a forbidden word, I/O.
pub fn setup(exp: &Path, r: &Reader) -> Result<PathBuf, String> {
    io::check_name(&r.design)?;
    if !prompts::PERSONAS.contains(&r.persona.as_str()) {
        return Err("persona must be F or H".into());
    }
    let code = code_of(exp, &r.experiment, &r.design, r.page)?;
    let (root, dir, prompt, images, extra) = match r.experiment.as_str() {
        "m8" => {
            let root = m8::root(exp);
            let raw = root.join("raw");
            let dir = raw
                .join("readers")
                .join(format!("{}_p{}_{code}_r{}", r.persona, r.page, r.k));
            let prompt = io::read(
                &raw.join("prompts")
                    .join(format!("{}_{code}.txt", r.persona)),
            )?;
            let img = format!("{code}.png");
            let images = vec![(raw.join("screens").join(&img).display().to_string(), img)];
            (root, dir, prompt, images, json!({"page": r.page}))
        }
        "m9" => {
            let root = m9::root(exp);
            let raw = root.join("raw");
            let dir = raw
                .join("readers")
                .join(format!("{}_{code}_r{}", r.persona, r.k));
            let prompt = io::read(
                &raw.join("prompts")
                    .join(format!("{}_{code}.txt", r.persona)),
            )?;
            let pos: Value = serde_json::from_str(&io::read(
                &raw.join("keys").join(format!("{code}.positions.json")),
            )?)
            .map_err(|e| e.to_string())?;
            let n = pos["screens"].as_u64().unwrap_or(0);
            let images = (1..=n)
                .map(|i| {
                    let f = format!("{code}-{i}.png");
                    (raw.join("screens").join(&f).display().to_string(), f)
                })
                .collect();
            (root, dir, prompt, images, json!({}))
        }
        "m10" => {
            let root = m10::root(exp);
            let raw = root.join("raw");
            let dir = raw
                .join("readers")
                .join(format!("{}_{code}_r{}", r.persona, r.k));
            let key: m10::Key =
                serde_json::from_str(&io::read(&raw.join("keys").join(format!("{code}.json")))?)
                    .map_err(|e| e.to_string())?;
            let mut order: Vec<String> = key.seqs.keys().cloned().collect();
            Rng::derive(r.seed, &format!("m10-order|{}", r.persona)).shuffle(&mut order);
            let seqs: Vec<[String; 4]> = order
                .iter()
                .map(|i| std::array::from_fn(|j| format!("{code}-{i}-{}.png", j + 1)))
                .collect();
            let prompt = prompts::m10_persona(&r.persona, &seqs);
            let images = seqs
                .iter()
                .flatten()
                .map(|f| (raw.join("screens").join(f).display().to_string(), f.clone()))
                .collect();
            (root, dir, prompt, images, json!({"order": order}))
        }
        e => return Err(format!("unknown experiment `{e}`")),
    };
    let _ = root;
    blind::check("prompt", &prompt)?;
    io::write(&dir.join("prompt.template.txt"), prompt.as_bytes())?;
    io::write_json(&dir.join("images.json"), &images)?;
    let mut meta = json!({"experiment": r.experiment, "code": code, "persona": r.persona,
        "reader": format!("r{}", r.k), "seed": r.seed, "model": r.model});
    if let (Some(m), Some(x)) = (meta.as_object_mut(), extra.as_object()) {
        for (k, v) in x {
            m.insert(k.clone(), v.clone());
        }
    }
    io::write_json(&dir.join("meta.json"), &meta)?;
    Ok(dir)
}
