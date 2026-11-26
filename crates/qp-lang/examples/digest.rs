//! Prints the executor's cross-target determinism digest (`qp_lang::program_digest`),
//! compared with `fixtures/compute/digest-program.txt` by `scripts/verify.sh` on the host,
//! x86_64 and wasm32 (wasm32-wasip1 under node):
//!
//! ```sh
//! cargo run --release -p qp-lang --example digest
//! cargo run --release -p qp-lang --example digest --target x86_64-apple-darwin
//! ```

fn main() {
    println!("{}", qp_lang::program_digest());
}
