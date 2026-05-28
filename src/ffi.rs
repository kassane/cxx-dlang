//! cxx bridge — FFI boundary between Rust and D.
//!
//! `extern "Rust"` items are implemented below and callable from D.
//! `unsafe extern "C++"` items are implemented in `d/cxx_d.d` and callable from Rust.

#[cxx::bridge(namespace = "cxx_d")]
pub mod bridge {
    /// Shared POD struct with identical layout on both sides.
    /// D mirror: `extern(C++, "cxx_d") struct Greeting`
    pub struct Greeting {
        pub name: String,
        pub count: i32,
    }

    extern "Rust" {
        /// Opaque Rust value; D holds it as a reference-sized pointer.
        type RustHandle;

        /// Returns a greeting string — panics are caught by `prevent_unwind`.
        fn rust_greet(name: &str) -> String;
        /// Allocates a `RustHandle` on the Rust heap; D receives a `Box<RustHandle>`.
        fn make_handle() -> Box<RustHandle>;
        /// Describes a `RustHandle` without consuming it.
        fn handle_describe(handle: &RustHandle) -> String;
    }

    unsafe extern "C++" {
        include!("cxx-dlang/include/cxx_d.h");

        /// Opaque D object; Rust holds it via `UniquePtr` (C++-heap-allocated).
        type DHandle;

        /// Returns `x * 2` — implemented in `d/cxx_d.d`.
        fn d_double(x: i32) -> i32;
        /// Allocates a `DHandle` on the C++ heap and returns ownership to Rust.
        fn d_make_handle() -> UniquePtr<DHandle>;
        /// Passes a Rust fn pointer to D; D invokes it and returns the result.
        fn d_run_callback(cb: fn(&str) -> String, input: &str) -> String;
        /// Identity: returns `x` unchanged.
        fn d_identity_bool(x: bool) -> bool;
        /// Returns `a + b`.
        fn d_add_f64(a: f64, b: f64) -> f64;
        /// Returns the byte length of `s` as seen by D (`rust::Str.len`).
        fn d_str_len(s: &str) -> usize;
        /// Returns `g.count` via a D const-ref parameter.
        fn d_greeting_count(g: &Greeting) -> i32;
    }
}

/// Opaque handle allocated and owned by Rust.
pub struct RustHandle {
    value: String,
}

pub fn rust_greet(name: &str) -> String {
    cxx::private::prevent_unwind("rust_greet", || format!("Hello, {name}! (count=1)"))
}

pub fn make_handle() -> Box<RustHandle> {
    cxx::private::prevent_unwind("make_handle", || {
        Box::new(RustHandle {
            value: "rust-handle".to_owned(),
        })
    })
}

pub fn handle_describe(handle: &RustHandle) -> String {
    cxx::private::prevent_unwind("handle_describe", || handle.value.clone())
}
