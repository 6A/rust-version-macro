rust-version-macro
==================

A Rust procedural macro that only compiles the content it marks if the given
expression matches the compiler's version.

## Usage

```rust
#![feature(use_extern_macros)]
extern crate rust_version_macro;

use rust_version_macro::rust_version;

#[rust_version(x == 0.0.0)]
fn should_never_compile() {
    // This code is not even read by the compiler, which means that it can be
    // completely invalid.
    use does_not::exist;

    exist();
}

#[rust_version(x > 1)] // No major / patch
fn should_compile_1() {}

#[rust_version(y > 1.20)] // Any identifier for comparison
fn should_compile_2() {}

#[rust_version(z >= 1.27.2)] // Exact version (at time of writing)
fn should_compile_3() {}

#[rust_version(1.27.0 < x < 2.0.0)] // Bound range
fn should_compile_4() {}
```

## Installation

> Rust >=1.29.0 is required.

```toml
[dependencies]
rust-version-macro = { git = "https://github.com/6A/rust-version-macro" }
```
