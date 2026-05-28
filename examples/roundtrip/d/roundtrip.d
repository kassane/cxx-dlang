module roundtrip;

import cxx_d;

// Callback typedef pattern (from ffi-matrix/d/lib_d.d reference).
// The fn pointer type mirrors the Rust `fn(&str) -> String` passed via
// the cxx bridge: `d_run_callback(cb: fn(&str) -> String, input: &str)`.
// On D side this is a C++ function pointer with nothrow qualification.
alias RustCallback = extern(C++) String function(Str) nothrow;

// D wrapper that invokes the Rust callback — demonstrates the roundtrip.
// By-value Str is fine on x86_64 (SysV: ptr+len fits in two registers).
// For cross-platform safety with large structs, prefer ref/pointer params.
extern(C++, "cxx_d") nothrow {
    pragma(inline, false) String d_invoke_callback(RustCallback cb, Str input) {
        return cb(input);
    }
}
