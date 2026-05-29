/**
 * Reusable D-side bindings for cxx.rs `rust::*` types and a minimal
 * `std::unique_ptr<T>` value-type binding.
 *
 * Imported by downstream consumers as `import cxx_d;` after wiring the crate
 * into their build via `cxx-dlang` (build dependency) + their own
 * `#[cxx::bridge]`.
 *
 * Every `extern(C++)` fn callable from Rust through cxx must be `nothrow` —
 * exceptions must not cross the C++ ABI boundary (UB). Pair with
 * `cxx::private::prevent_unwind` on the Rust side.
 */
module cxx_d;

version (CppRuntime_GNU)
    pragma(lib, "stdc++");
else version (CppRuntime_LLVM)
    pragma(lib, "c++");

public import core.stdcpp.string : basic_string;
public import core.stdcpp.array : array;

import core.stdc.stdint : uintptr_t;
import core.stdcpp.new_ : __cpp_new_nothrow;

/// `std::string` alias re-exported for convenience.
alias CxxString = basic_string!char;

/// `std::array<T, N>` alias re-exported for convenience.
alias CxxArray(T, size_t N) = array!(T, N);

/// Stateless allocator placeholder. Used as the second template argument of
/// `std::vector<T, std::allocator<T>>` so the Itanium mangled name matches.
/// (The real `std::allocator<T>` has private state but is empty-base-optimised
/// to size 0; binding it as an empty struct preserves the mangling.)
extern (C++,"std") struct allocator(T)
{
}

/// Stateless deleter; needed so the two-parameter `unique_ptr(T, D)` template
/// encodes correctly in MSVC-decorated return-type symbols.
extern (C++,"std") struct default_delete(T)
{
}

/// Minimal `std::unique_ptr<T, D>` value-type binding.
///
/// Value layout (NOT `extern(C++, class)`) so `T* _ptr` is a raw pointer.
/// Empty destructor makes the type non-trivially destructible — that forces
/// sret return on all ABIs (Itanium and MSVC), which is what cxx expects.
///
/// `core.stdcpp.memory` is unavailable in LDC2 1.42 binary distributions
/// (depends on `core.stdcpp.tuple`), so this hand-declared minimum stands in.
extern (C++,"std") struct unique_ptr(T, D = default_delete!T)
{
    T* _ptr;
    extern (C++) ~this() nothrow @nogc
    {
    }
}

// ---------------------------------------------------------------------------
// rust:: types — live in the inline namespace `rust::cxxbridge1`, so D must
// mirror with `extern(C++, "rust", "cxxbridge1")` for the linker to match.
// ---------------------------------------------------------------------------

/// `rust::Str` — borrowed UTF-8 slice (16B: ptr + len, register-passable on x86_64).
extern (C++,"rust","cxxbridge1") extern (C++,class) struct Str
{
    const(char)* ptr;
    size_t len;
}

/// `rust::Slice<T>` — borrowed `&[T]` (16B: ptr + len). Same layout as Str.
///
/// The template element carries its own constness:
///   * `Slice!(const(ubyte))` ≡ `rust::Slice<const uint8_t>` (from `&[u8]`)
///   * `Slice!(ubyte)`        ≡ `rust::Slice<uint8_t>`       (from `&mut [u8]`)
extern (C++,"rust","cxxbridge1") extern (C++,class) struct Slice(T)
{
    T* ptr;
    size_t len;
}

/// `rust::String` — owned UTF-8 string (24B: opaque `uintptr_t[3]`).
/// Never construct directly; use cxx helper functions only.
extern (C++,"rust","cxxbridge1") extern (C++,class) struct String
{
    private uintptr_t[3] repr;
}

/// `rust::Fn<Ret(Args...)>` — 16B value struct `{trampoline, fn_}`.
///
/// D TMP encodes the template arg as a pack (`J..E`); cxx uses a function
/// type (`F..E`). Functions that take a `Fn` parameter require
/// `pragma(mangle)` to pin the exact Itanium / MSVC symbol — see consumer
/// crates for the pattern.
extern (C++,"rust","cxxbridge1") struct Fn(Ret, Args...)
{
    Ret function(Args, void*) nothrow trampoline;
    void* fn_;
}

/// `rust::Vec<T>` — Rust-owned vector. D must never construct or copy one.
/// `size()` and `data()` are bound as `extern(C++) const` so D can call
/// `rust::cxxbridge1::Vec<T>::size()` and `::data()` to walk the buffer.
extern (C++,"rust","cxxbridge1") extern (C++,class) struct Vec(T)
{
    @disable this();
    @disable this(this);
    extern (C++) size_t size() const nothrow @nogc;
    extern (C++) const(T)* data() const nothrow @nogc;
}

// ---------------------------------------------------------------------------
// Additional std:: types — bound by-hand because the corresponding
// `core.stdcpp.*` modules are either MSVC-only (vector) or absent in LDC2 1.42
// binary distributions (memory → shared_ptr/weak_ptr).
//
// Layouts target the Itanium-stable subset used by libstdc++, libc++ and
// MSVC's STL alike: a small fixed number of pointer-sized words. Methods are
// looked up via Itanium / MSVC mangling, so field-level breakage on exotic
// stdlibs only affects direct field access — method calls keep working.
// ---------------------------------------------------------------------------

// Method-binding availability differs by C++ runtime:
//   libstdc++ (GNU)    — shared_ptr / weak_ptr inherit use_count / get / expired
//                        from the PRIVATE base `std::__shared_ptr` (Itanium
//                        symbol is not on shared_ptr itself); methods omitted.
//                        Consumers wrap with an inline C++ helper.
//   libc++ (LLVM)      — methods emitted directly on shared_ptr / weak_ptr.
//   MSVC               — methods emitted directly on shared_ptr / weak_ptr.
version (CppRuntime_LLVM) version = CxxDlang_HasSmartPtrMethods;
version (CppRuntime_Microsoft) version = CxxDlang_HasSmartPtrMethods;

/// `std::shared_ptr<T>` value-type binding (16B: payload ptr + control block ptr).
extern (C++,"std") struct shared_ptr(T)
{
    T* _ptr;
    void* _ctrl;
    extern (C++) ~this() nothrow @nogc
    {
    }

    version (CxxDlang_HasSmartPtrMethods)
    {
        extern (C++) T* get() const nothrow @nogc;
        extern (C++) long use_count() const nothrow @nogc;
    }
}

/// `std::weak_ptr<T>` value-type binding (16B, same layout as shared_ptr).
extern (C++,"std") struct weak_ptr(T)
{
    T* _ptr;
    void* _ctrl;
    extern (C++) ~this() nothrow @nogc
    {
    }

    version (CxxDlang_HasSmartPtrMethods)
    {
        extern (C++) long use_count() const nothrow @nogc;
        extern (C++) bool expired() const nothrow @nogc;
    }
}

/// `std::vector<T, std::allocator<T>>` value-type binding
/// (24B: `{begin, end, end_of_capacity}` on all three stdlibs).
/// The two-parameter form matches the Itanium and MSVC mangled symbols.
extern (C++,"std") struct vector(T, Alloc = allocator!T)
{
    T* _begin;
    T* _end;
    T* _end_capacity;
    extern (C++) ~this() nothrow @nogc
    {
    }

    extern (C++) size_t size() const nothrow @nogc;
    extern (C++) const(T)* data() const nothrow @nogc;
    extern (C++) bool empty() const nothrow @nogc;
}
