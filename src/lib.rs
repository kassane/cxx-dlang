#![allow(unexpected_cfgs)]
//! Bidirectional Rust ↔ D (LDC2) FFI via cxx.rs and the C++ ABI.
//!
//! Define the boundary once in [`ffi::bridge`] — cxx.rs generates C++ headers
//! that LDC2 picks up via `extern(C++, "cxx_d")`, no manual `.di` files needed.
//!
//! # Safety contract
//! - Every Rust fn exposed to D is wrapped with `cxx::prevent_unwind`.
//! - Every D fn callable from Rust is declared `nothrow`.
//! - D handles returned as `UniquePtr<T>` are C++-heap-allocated (`__cpp_new_nothrow`).

pub use cxx;

pub mod ffi;
