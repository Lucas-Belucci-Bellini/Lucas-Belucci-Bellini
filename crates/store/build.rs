//! As migrations são embutidas no binário por `sqlx::migrate!`; sem isto, uma
//! migration nova não recompilaria o crate.
fn main() {
    println!("cargo:rerun-if-changed=../../db/migrations");
}
