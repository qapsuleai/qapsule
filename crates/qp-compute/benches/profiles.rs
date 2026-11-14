//! RP1 load profiles A–D (see `qp_compute::profiles`).
//!
//! ```sh
//! cargo bench -p qp-compute --bench profiles -- --reps 11 [--only a,b,c,d]
//! ```

#[path = "driver/mod.rs"]
mod driver;

fn main() {
    driver::run("a,b,c,d", 11);
}
