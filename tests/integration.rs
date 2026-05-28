// Parity tests modeled after cxx/tests/test.rs categories:
//   primitives, strings, opaque handles, callbacks, shared structs
//
// D functions called here are declared in src/ffi.rs (unsafe extern "C++" block)
// and implemented in d/cxx_d.d.

use cxx_dlang::ffi::bridge;

// ── Existing bridge (should already pass) ──────────────────────────────────

#[test]
fn test_d_double() {
    assert_eq!(bridge::d_double(0), 0);
    assert_eq!(bridge::d_double(21), 42);
    assert_eq!(bridge::d_double(-7), -14);
}

#[test]
fn test_rust_greet() {
    let result = cxx_dlang::ffi::rust_greet("world");
    assert!(result.contains("world"), "greet must echo the name");
    assert!(result.starts_with("Hello"), "greet must start with Hello");
}

#[test]
fn test_rust_handle_roundtrip() {
    let handle = cxx_dlang::ffi::make_handle();
    let desc = cxx_dlang::ffi::handle_describe(&handle);
    assert_eq!(desc, "rust-handle");
}

#[test]
fn test_d_make_handle() {
    // UniquePtr<DHandle> constructed on the D side — must not be null
    let handle = bridge::d_make_handle();
    assert!(!handle.is_null(), "d_make_handle must return a live pointer");
}

#[test]
fn test_d_run_callback() {
    let result = bridge::d_run_callback(|s| format!("echo:{s}"), "ping");
    assert_eq!(result, "echo:ping");
}

#[test]
fn test_greeting_struct() {
    let g = bridge::Greeting {
        name: "alice".to_string(),
        count: 5,
    };
    assert_eq!(g.name, "alice");
    assert_eq!(g.count, 5);
}

// ── New bridge — parity with cxx test categories ───────────────────────────
// These tests call D functions that do not exist yet in the bridge.
// They will FAIL until src/ffi.rs + include/cxx_d.h + d/cxx_d.d are updated.

#[test]
fn test_bool_roundtrip() {
    // parity: cxx test_c_return bool
    assert!(bridge::d_identity_bool(true));
    assert!(!bridge::d_identity_bool(false));
}

#[test]
fn test_f64_arithmetic() {
    // parity: cxx test_c_take f64
    let result = bridge::d_add_f64(1.5, 2.5);
    assert!((result - 4.0_f64).abs() < f64::EPSILON);
}

#[test]
fn test_str_len_from_d() {
    // parity: cxx test_c_take &str — D reads rust::Str.len
    assert_eq!(bridge::d_str_len("hello"), 5);
    assert_eq!(bridge::d_str_len(""), 0);
    assert_eq!(bridge::d_str_len("cxx-dlang"), 9);
}

#[test]
fn test_greeting_count_from_d() {
    // parity: cxx test_c_method_calls — D accesses a shared struct field
    let g = bridge::Greeting {
        name: "bob".to_string(),
        count: 7,
    };
    assert_eq!(bridge::d_greeting_count(&g), 7);
}
