//! Prints the cross-target compute digest (see `qp_compute::sweep::cross_target_digest`).
//!
//! ```sh
//! cargo run --release -p qp-compute --example digest
//! cargo run --release -p qp-compute --example digest --target x86_64-apple-darwin
//! ```
//!
//! With the argument `r3` it prints the separate digest of the R3/T3 studies (`highest`,
//! `lowest`, `atr`; `qp_compute::sweep::r3_studies_digest`) instead, so the default
//! output (one line, compared by `scripts/verify.sh`) stays unchanged:
//!
//! ```sh
//! cargo run --release -p qp-compute --example digest -- r3
//! ```

fn main() {
    if std::env::args().nth(1).as_deref() == Some("r3") {
        println!("{:016x}", qp_compute::sweep::r3_studies_digest());
    } else {
        println!("{:016x}", qp_compute::sweep::cross_target_digest());
    }
}
