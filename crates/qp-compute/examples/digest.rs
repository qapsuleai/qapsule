//! Prints the cross-target compute digest (see `qp_compute::sweep::cross_target_digest`).
//!
//! ```sh
//! cargo run --release -p qp-compute --example digest
//! cargo run --release -p qp-compute --example digest --target x86_64-apple-darwin
//! ```

fn main() {
    println!("{:016x}", qp_compute::sweep::cross_target_digest());
}
