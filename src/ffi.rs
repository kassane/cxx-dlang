//! cxx bridge definitions for the cxx-dlang crate.

#[cxx::bridge(namespace = "cxx_d")]
pub mod bridge {
    /// Shared POD struct — same layout on both Rust and D sides.
    /// D: `extern(C++, "cxx_d") struct Greeting`
    pub struct Greeting {
        pub name: String,
        pub count: i32,
    }

    // -------- Rust exposes to D --------
    extern "Rust" {
        /// Opaque Rust handle; D sees it as a reference-type via extern(C++, class)
        type RustHandle;

        fn rust_greet(name: &str) -> String;
        fn make_handle() -> Box<RustHandle>;
        fn handle_describe(handle: &RustHandle) -> String;
    }

    // -------- D exposes to Rust --------
    unsafe extern "C++" {
        include!("cxx-dlang/include/cxx_d.h");

        /// Opaque D handle — D class compiled by LDC2
        type DHandle;

        fn d_double(x: i32) -> i32;
        fn d_make_handle() -> UniquePtr<DHandle>;
        fn d_run_callback(cb: fn(&str) -> String, input: &str) -> String;

        // parity: cxx test_c_return bool
        fn d_identity_bool(x: bool) -> bool;
        // parity: cxx test_c_take f64
        fn d_add_f64(a: f64, b: f64) -> f64;
        // parity: cxx test_c_take &str — D reads rust::Str.len
        fn d_str_len(s: &str) -> usize;
        // parity: cxx test_c_method_calls — D reads a shared struct field
        fn d_greeting_count(g: &Greeting) -> i32;
    }
}

// -------- Rust implementations exposed to D --------

pub struct RustHandle {
    value: String,
}

pub fn rust_greet(name: &str) -> String {
    cxx::private::prevent_unwind("rust_greet", || format!("Hello, {}! (count=1)", name))
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
