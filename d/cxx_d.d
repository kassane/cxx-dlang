/**
 * D-side mirror of the cxx_d bridge types.
 *
 * IMPORTANT: All extern(C++) functions callable from Rust MUST be nothrow.
 * Exceptions MUST NOT cross the C++ ABI boundary.
 *
 * Struct layout: D structs == C++ structs (by value).
 * D classes == C++ classes (by reference) — use extern(C++, class) struct.
 */
module cxx_d;

import core.stdc.stdint : uintptr_t;
import core.attribute : mustuse;
import ldc.attributes : assumeUsed;
import core.stdcpp.new_ : __cpp_new_nothrow; // nothrow C++ operator new — compatible with delete

// Minimal unique_ptr — value type (NOT extern(C++, class)) so T* _ptr is a raw pointer.
// core.stdcpp.memory is unavailable (pulls in core.stdcpp.tuple which is absent).
// The empty extern(C++) destructor makes this non-trivially destructible, so LDC2
// uses sret for return — matching Itanium ABI for non-trivially-destructible types.
extern(C++, "std") struct unique_ptr(T) {
    T* _ptr;
    extern(C++) ~this() nothrow @nogc {}
}

// rust::Str / rust::String / rust::Fn / rust::Vec live in the inline namespace
// rust::cxxbridge1 — the linker uses the full cxxbridge1-qualified mangled name.
// D must mirror this with extern(C++, "rust", "cxxbridge1") so symbol names match.

// rust::Str — borrowed UTF-8 slice from Rust (16B: ptr + len, SysV register-passable)
extern(C++, "rust", "cxxbridge1") extern(C++, class) struct Str {
    const(char)* ptr;
    size_t len;
}

// rust::String — owned UTF-8 string from Rust (24B: std::array<uintptr_t, 3>)
// Never construct directly; use cxx helper functions only.
extern(C++, "rust", "cxxbridge1") extern(C++, class) struct String {
    // opaque 24-byte repr — do not access fields
    private uintptr_t[3] repr;
}

// rust::Fn<Ret(Args...)> — 16-byte value struct: (trampoline ptr + context void*)
// Layout from cxx/include/cxx.h:416: { Ret(*trampoline)(Args..., void* fn); void* fn; }
// Use extern(C++) struct (NOT extern(C++, class)) for by-value semantics (16B on x86_64).
// Call by invoking: cb.trampoline(args..., cb.fn_);
extern(C++, "rust", "cxxbridge1") {
    extern(C++) struct Fn(Ret, Args...) {
        Ret function(Args, void*) nothrow trampoline;
        void* fn_;
    }
    alias StringFromStr = Fn!(String, Str);
}

// rust::Vec<T> template binding.
// Pattern: D's extern(C++, class) struct Name(T) auto-mangles to the matching
// Itanium template-instantiation symbol (verified: tmpffi.sh, 5-toolchain, 0 undefined syms).
// C++ must explicitly instantiate (e.g. `template class rust::Vec<int>;`) — cxx.rs
// does this automatically for every Vec<T> that appears in a bridge block.
// @disable this() / this(this): rust::Vec<T> is exclusively owned and managed by Rust;
// D must never construct or copy one directly.
extern(C++, "rust", "cxxbridge1") {
    extern(C++, class) struct Vec(T) {
        @disable this();
        @disable this(this);
    }
    alias VecI32    = Vec!int;
    alias VecString = Vec!String;  // rust::Vec<rust::String>
}

// Fallback via pragma(mangle) for specific Vec instantiations whose Itanium symbol
// D TMP cannot express (e.g. const-qualified element types, nested templates):
// extern(C++) nothrow @nogc {
//     pragma(mangle, "_ZN4rust3VecIiE...") void vec_i32_push(ref Vec!int v, int x);
// }

// Shared POD struct — must match src/ffi.rs Greeting layout exactly.
// @mustuse: D compiler errors if a returned Greeting is discarded (parity with Rust #[must_use]).
// NOTE: @mustuse applies to struct/union types only in LDC2 1.42 — no function-level equivalent.
// Rust side uses #[must_use] on both the type and functions; D side covers the type only.
@mustuse
extern(C++, "cxx_d") struct Greeting {
    String name;
    int count;
}

// String(24B, align 8) + i32(4B) + 4B padding = 32B total
static assert(Greeting.sizeof == 32, "Greeting ABI layout mismatch with cxx bridge");

// Opaque Rust handle — reference semantics (D class == C++ class == pointer-sized)
extern(C++, "cxx_d") extern(C++, class) struct RustHandle {
    @disable this();
}

// Opaque D handle — value type (NOT extern(C++, class)) so DHandle* is a raw
// pointer inside unique_ptr<DHandle>._ptr. Explicit destructor exported so
// std::unique_ptr<DHandle> can call ~DHandle() without a linker error.
extern(C++, "cxx_d") struct DHandle {
    @assumeUsed extern(C++) ~this() nothrow @nogc {}
}

// D-side implementations exposed to Rust.
// @assumeUsed: prevents the Rust linker from DCE-eliminating these symbols
// (parity: Rust #[used]; see ldc.attributes.assumeUsed).
extern(C++, "cxx_d") nothrow {

    @assumeUsed pragma(inline, false) int d_double(int x) @nogc {
        return x * 2;
    }

    // @trusted: cast(void*→DHandle*) is safe here — __cpp_new_nothrow returns a
    // freshly allocated, properly aligned block. @trusted suppresses the -preview=safer
    // restriction on void* casts without disabling safety checks in callers.
    @assumeUsed pragma(inline, false) unique_ptr!DHandle d_make_handle() @trusted {
        unique_ptr!DHandle result;
        result._ptr = cast(DHandle*)__cpp_new_nothrow(DHandle.sizeof);
        return result;
    }

    // D TMP mangles Fn!(String,Str) as Fn<String,J(Str)E> (pack), but cxx.rs
    // expects Fn<F(String)(Str)E> (function-type template arg). Use pragma(mangle)
    // to pin the exact Itanium symbol. Verified via nm comparison.
    @assumeUsed pragma(inline, false)
    pragma(mangle, "_ZN5cxx_d14d_run_callbackEN4rust10cxxbridge12FnIFNS1_6StringENS1_3StrEEEES4_")
    String d_run_callback(StringFromStr cb, Str input) {
        return cb.trampoline(input, cb.fn_);
    }

    // parity: cxx test_c_return bool
    @assumeUsed pragma(inline, false) bool d_identity_bool(bool x) @nogc {
        return x;
    }

    // parity: cxx test_c_take f64
    @assumeUsed pragma(inline, false) double d_add_f64(double a, double b) @nogc {
        return a + b;
    }

    // parity: cxx test_c_take &str — D reads rust::Str.len field
    @assumeUsed pragma(inline, false) size_t d_str_len(Str s) @nogc {
        return s.len;
    }

    // parity: cxx test_c_method_calls — D reads a shared struct field via const ref
    @assumeUsed pragma(inline, false) int d_greeting_count(ref const(Greeting) g) @nogc {
        return g.count;
    }
}
