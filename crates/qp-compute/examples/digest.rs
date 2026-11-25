//! Prints the cross-target compute digest (see `qp_compute::sweep::cross_target_digest`).
//!
//! ```sh
//! cargo run --release -p qp-compute --example digest
//! cargo run --release -p qp-compute --example digest --target x86_64-apple-darwin
//! ```
//!
//! With the argument `r3` it prints the separate digest of the R3/T3 studies (`highest`,
//! `lowest`, `atr`; `qp_compute::sweep::r3_studies_digest`) instead, and with `rl1` the
//! digest of the RL1 kernels (`stdev`, `percentrank`;
//! `qp_compute::sweep::rl1_studies_digest`), so the default output (one line, compared
//! by `scripts/verify.sh`) stays unchanged:
//!
//! ```sh
//! cargo run --release -p qp-compute --example digest -- r3
//! cargo run --release -p qp-compute --example digest -- rl1
//! ```

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("r3") => println!("{:016x}", qp_compute::sweep::r3_studies_digest()),
        Some("rl1") => println!("{:016x}", qp_compute::sweep::rl1_studies_digest()),
        _ => println!("{:016x}", qp_compute::sweep::cross_target_digest()),
    }
}
