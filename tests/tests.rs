#![feature(use_extern_macros, stmt_expr_attributes, proc_macro_expr)]
extern crate rust_version_macro;

use rust_version_macro::rust_version;

#[rust_version(x == 0.0.0)]
fn should_never_compile() {
    use does_not::exist;

    exist();
}

#[rust_version(x > 1)] // No major / patch
fn should_compile_1() {}

#[rust_version(y > 1.20)] // Any identifier
fn should_compile_2() {}

#[rust_version(z >= 1.27.2)] // Exact version (at time of writing)
fn should_compile_3() {}

#[rust_version(1.27.0 < x < 2.0.0)] // Bound range
fn should_compile_4() {}

#[test]
fn tests() {
    should_compile_1();
    should_compile_2();
    should_compile_3();
    should_compile_4();

    #[rust_version(x < 1.27)] // Statement #1
    assert!(false);

    #[allow(unused_assignments)]
    let mut modified = false;

    #[rust_version(x > 1.27)] // Statement #2
    modified = true;

    assert!(modified);
}
