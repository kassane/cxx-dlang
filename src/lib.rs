#![allow(unexpected_cfgs)]
//! Build-time helper crate that ships reusable D-side bindings for the cxx.rs
//! `rust::*` types (`Str`, `String`, `Slice<T>`, `Vec<T>`, `Fn<R(A)>`) plus a
//! minimal `std::unique_ptr<T>` value-type binding.
//!
//! Downstream crates declare their own `#[cxx::bridge]` and import the D
//! module `cxx_d` from this crate's `d/cxx_d.d` to wire the C++ ABI both ways
//! without re-writing the LDC2 toolchain glue.
//!
//! The companion `build.rs` discovers `ldc2`, compiles `d/cxx_d.d` with the
//! curated `--preview=` safety flag set, and links druntime + phobos2
//! statically into the host crate.

pub use cxx;
