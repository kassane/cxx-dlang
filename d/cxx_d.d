/**
 * D-side mirror of the cxx_d bridge.
 *
 * All `extern(C++)` functions callable from Rust MUST be `nothrow` — exceptions
 * must not cross the C++ ABI boundary (UB). Rust-side panics are guarded by
 * `cxx::prevent_unwind` in src/ffi.rs.
 */
module cxx_d;

import core.stdc.stdint : uintptr_t;
import core.attribute : mustuse;
import ldc.attributes : assumeUsed;
import core.stdcpp.new_ : __cpp_new_nothrow;

// ---------------------------------------------------------------------------
// std:: types
// ---------------------------------------------------------------------------

/// Stateless deleter for std::unique_ptr — needed so the two-parameter template
/// encodes correctly in MSVC-decorated return-type symbols.
extern(C++, "std") struct default_delete(T) {}

/// Minimal std::unique_ptr binding (value type, NOT extern(C++,class)).
/// Two-parameter form matches std::unique_ptr<T, default_delete<T>> on all ABIs.
/// Empty destructor makes it non-trivially destructible → sret return on all platforms.
/// core.stdcpp.memory is unavailable (depends on core.stdcpp.tuple, absent in LDC 1.42).
extern(C++, "std") struct unique_ptr(T, D = default_delete!T) {
    T* _ptr;
    extern(C++) ~this() nothrow @nogc {}
}

// ---------------------------------------------------------------------------
// rust:: types  (inline namespace rust::cxxbridge1 → use two-level namespace)
// ---------------------------------------------------------------------------

/// rust::Str — borrowed UTF-8 slice (16B: ptr + len, register-passable on x86_64).
extern(C++, "rust", "cxxbridge1") extern(C++, class) struct Str {
    const(char)* ptr;
    size_t len;
}

/// rust::String — owned UTF-8 string (24B: opaque uintptr_t[3]). Never construct directly.
extern(C++, "rust", "cxxbridge1") extern(C++, class) struct String {
    private uintptr_t[3] repr;
}

/// rust::Fn<Ret(Args...)> — 16B value struct {trampoline, fn_}.
/// D TMP encodes this as a pack (J..E); cxx uses a function type (F..E).
/// Call via: cb.trampoline(args..., cb.fn_).
/// Functions using this type require pragma(mangle) to fix the symbol (see below).
extern(C++, "rust", "cxxbridge1") {
    extern(C++) struct Fn(Ret, Args...) {
        Ret function(Args, void*) nothrow trampoline;
        void* fn_;
    }
    alias StringFromStr = Fn!(String, Str);
}

/// rust::Vec<T> — Rust-owned vector. D must never construct or copy one.
extern(C++, "rust", "cxxbridge1") {
    extern(C++, class) struct Vec(T) {
        @disable this();
        @disable this(this);
    }
    alias VecI32    = Vec!int;
    alias VecString = Vec!String;
}

// ---------------------------------------------------------------------------
// cxx_d:: shared types
// ---------------------------------------------------------------------------

/// Shared POD struct — must match Greeting in src/ffi.rs (String=24B + i32=4B + 4B pad = 32B).
/// @mustuse: compiler errors if a returned Greeting is silently discarded.
@mustuse
extern(C++, "cxx_d") struct Greeting {
    String name;
    int count;
}
static assert(Greeting.sizeof == 32, "Greeting ABI layout mismatch with cxx bridge");

/// Opaque Rust handle — reference-sized pointer; D must never construct one.
extern(C++, "cxx_d") extern(C++, class) struct RustHandle {
    @disable this();
}

/// Opaque D handle — value type so DHandle* is a raw pointer inside unique_ptr._ptr.
/// Explicit destructor is exported so std::unique_ptr<DHandle> can call ~DHandle().
extern(C++, "cxx_d") struct DHandle {
    @assumeUsed extern(C++) ~this() nothrow @nogc {}
}

// ---------------------------------------------------------------------------
// D-implemented bridge functions (called from Rust via cxx)
// ---------------------------------------------------------------------------

extern(C++, "cxx_d") nothrow {

    @assumeUsed pragma(inline, false)
    int d_double(int x) @nogc { return x * 2; }

    /// Allocates DHandle on the C++ heap so std::unique_ptr's default_delete
    /// (operator delete) frees it correctly when UniquePtr<DHandle> is dropped.
    /// @trusted: cast(void*→DHandle*) is safe — __cpp_new_nothrow returns aligned memory.
    version(Windows) {
        @assumeUsed pragma(inline, false)
        pragma(mangle, "?d_make_handle@cxx_d@@YA?AV?$unique_ptr@VDHandle@cxx_d@@U?$default_delete@VDHandle@cxx_d@@@std@@@std@@XZ")
        unique_ptr!DHandle d_make_handle() @trusted {
            unique_ptr!DHandle result;
            result._ptr = cast(DHandle*)__cpp_new_nothrow(DHandle.sizeof);
            return result;
        }
    } else {
        @assumeUsed pragma(inline, false)
        unique_ptr!DHandle d_make_handle() @trusted {
            unique_ptr!DHandle result;
            result._ptr = cast(DHandle*)__cpp_new_nothrow(DHandle.sizeof);
            return result;
        }
    }

    /// pragma(mangle): D TMP encodes Fn!(String,Str) as a pack (J..E) but cxx.rs
    /// expects a function-type template arg (F..E). Pin the exact symbol per ABI.
    version(Windows) {
        @assumeUsed pragma(inline, false)
        pragma(mangle, "?d_run_callback@cxx_d@@YA?AVString@cxxbridge1@rust@@V?$Fn@$$A6A?AVString@cxxbridge1@rust@@VStr@23@@Z@34@VStr@34@@Z")
        String d_run_callback(StringFromStr cb, Str input) {
            return cb.trampoline(input, cb.fn_);
        }
    } else {
        @assumeUsed pragma(inline, false)
        pragma(mangle, "_ZN5cxx_d14d_run_callbackEN4rust10cxxbridge12FnIFNS1_6StringENS1_3StrEEEES4_")
        String d_run_callback(StringFromStr cb, Str input) {
            return cb.trampoline(input, cb.fn_);
        }
    }

    @assumeUsed pragma(inline, false)
    bool d_identity_bool(bool x) @nogc { return x; }

    @assumeUsed pragma(inline, false)
    double d_add_f64(double a, double b) @nogc { return a + b; }

    /// Returns rust::Str.len without copying the string data.
    @assumeUsed pragma(inline, false)
    size_t d_str_len(Str s) @nogc { return s.len; }

    /// Returns g.count via a C++ const-ref parameter (D: ref const(Greeting)).
    @assumeUsed pragma(inline, false)
    int d_greeting_count(ref const(Greeting) g) @nogc { return g.count; }
}
