use std::env;

fn main() {
    if env::var("CARGO_FEATURE_TEST_PERFECT").is_ok() {
        // println!("cargo:warning=Building with 'test-perfect'");
        println!("cargo::rerun-if-changed=src/c");
        cc::Build::new()
            .file("src/c/netlist_sim.c")
            .file("src/c/perfect6502.c")
            .compile("perfect6502");
        println!("cargo:rustc-link-lib=static=perfect6502");
    }
}
