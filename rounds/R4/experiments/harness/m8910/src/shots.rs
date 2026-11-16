//! Headless Chromium for the screenshots (SPEC §0.2): the harness writes a manifest and
//! runs `node shoot.mjs <manifest>` (static sheet pages, M8) or `node demo_shots.mjs
//! <plan>` (the live demo, M9 and M10), both with the web shell's Playwright
//! (`shells/web/node_modules`). Phone viewport 390×844 CSS px, DPR 2.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::io;

/// Phone viewport (CSS px) and device scale (SPEC §0.2).
pub const VIEWPORT: (u32, u32) = (390, 844);
/// Device pixel ratio of every screenshot.
pub const DPR: u32 = 2;

fn script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
}

/// Runs `node <script> <args…>` from the harness directory.
///
/// # Errors
/// `node` missing or the script failing.
pub fn node(name: &str, args: &[&str]) -> Result<(), String> {
    let web = io::repo_root().join("shells").join("web");
    if !web.join("node_modules").join("playwright").exists() {
        return Err(format!(
            "playwright missing: run `npm ci` in {} (and `npx playwright install chromium`)",
            web.display()
        ));
    }
    let st = Command::new("node")
        .arg(script(name))
        .args(args)
        .status()
        .map_err(|e| format!("node: {e}"))?;
    if st.success() {
        Ok(())
    } else {
        Err(format!("node {name} failed ({st})"))
    }
}

/// The `qp` binary for the live demo (`QP_BIN`, else the workspace's release build).
#[must_use]
pub fn qp_bin() -> PathBuf {
    std::env::var_os("QP_BIN").map_or_else(
        || io::repo_root().join("target").join("release").join("qp"),
        PathBuf::from,
    )
}
