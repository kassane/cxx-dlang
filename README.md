# cxx-dlang — safe FFI between Rust and D

Bidirectional FFI between Rust and D (LDC2) using **[cxx.rs](https://cxx.rs/)** and the **C++ ABI** as interchange — no `extern "C"` glue, no bindgen, no unsafe pointer casts on the Rust side.

```toml
[dependencies]
cxx = "1.0"

[build-dependencies]
cxx-build = "1.0"
```

*Requires: LDC2 ≥ 1.40, Rust edition 2024 (rustc 1.85+)

<br>

## How it works

Define the FFI boundary once in a `#[cxx::bridge]` module. cxx.rs generates
C++ headers; LDC2's `extern(C++, "ns")` picks them up directly — no manual
`.di` files, no `pragma(mangle)` for common cases.

```rust
// src/ffi.rs
#[cxx::bridge(namespace = "cxx_d")]
pub mod bridge {
    struct Greeting { pub name: String, pub count: i32 }

    extern "Rust" {
        type RustHandle;
        fn rust_greet(name: &str) -> String;
        fn make_handle() -> Box<RustHandle>;
    }

    unsafe extern "C++" {
        include!("cxx-dlang/include/cxx_d.h");
        type DHandle;
        fn d_double(x: i32) -> i32;
        fn d_make_handle() -> UniquePtr<DHandle>;
        fn d_run_callback(cb: fn(&str) -> String, input: &str) -> String;
    }
}
```

```d
// d/cxx_d.d
extern(C++, "cxx_d") nothrow {
    @assumeUsed pragma(inline, false)
    int d_double(int x) @nogc { return x * 2; }

    @assumeUsed pragma(inline, false)
    unique_ptr!DHandle d_make_handle() @trusted {
        unique_ptr!DHandle r;
        r._ptr = cast(DHandle*)__cpp_new_nothrow(DHandle.sizeof);
        return r;
    }

    @assumeUsed pragma(inline, false)
    pragma(mangle, "_ZN5cxx_d14d_run_callbackEN4rust10cxxbridge12FnIFNS1_6StringENS1_3StrEEEES4_")
    String d_run_callback(StringFromStr cb, Str input) {
        return cb.trampoline(input, cb.fn_);
    }
}
```

<br>

## Quick start

```bash
cargo run --example hello
# Hello, D! (count=1)
# d_double(21) = 42

cargo run --example roundtrip
# callback result: [hello]
# d_str_len("roundtrip") = 9
# d_make_handle is_null: false
# rust handle: rust-handle

cargo run --example structs
# Greeting { name: "alice", count: 6 }
# Greeting::sizeof == 32

cargo test --tests
# test result: ok. 10 passed
```

Set `LDC2_PATH=/path/to/ldc2` if ldc2 is not on `PATH`.

<br>

## Type mapping

| Rust | C++ (cxx) | D |
|------|-----------|---|
| `bool`, `i32`, `f64` | `bool`, `int32_t`, `double` | `bool`, `int`, `double` |
| `&str` | `rust::Str` (16B) | `extern(C++,"rust","cxxbridge1") extern(C++,class) struct Str` |
| `String` | `rust::String` (24B) | `extern(C++,"rust","cxxbridge1") extern(C++,class) struct String` |
| `Box<T>` opaque | `rust::Box<T>` | `extern(C++,"cxx_d") extern(C++,class) struct T` |
| `UniquePtr<T>` | `std::unique_ptr<T>` | hand-declared value `struct unique_ptr(T)` + empty dtor |
| `fn(A) -> R` | `rust::Fn<R(A)>` | `extern(C++) struct Fn(R,A)` + `pragma(mangle)` for sret |
| `struct Greeting` | POD struct | `extern(C++,"cxx_d") struct Greeting` |

`rust::Str/String/Fn/Vec` live in `inline namespace cxxbridge1` — D must use
`extern(C++, "rust", "cxxbridge1")` to match the linker's mangled names.

<br>

## Safety

**Panic / exception boundary** — every Rust function exposed to D is wrapped
with `cxx::prevent_unwind`; every D function callable from Rust is `nothrow`.
Violating either is UB.

**GC safety** — D's druntime scans the stack and heap for pointers. Never
store a Rust `Box<T>` or `Arc<T>` in a D `class` field; druntime will trace
and corrupt Rust memory. Keep opaque Rust handles in a Rust-side registry and
give D an integer key.

**Allocator discipline** — D handles returned through `UniquePtr<T>` must be
allocated with `__cpp_new_nothrow` (from `core.stdcpp.new_`), not D's GC
`new`. `std::unique_ptr`'s default deleter calls `operator delete`; GC memory
is not compatible with that.

**LDC2 safety flags** — all D compilation units use:
`--preview=safer,dip1000,nosharedaccess,fixImmutableConv,systemVariables`

<br>

## Known limitations

- LDC2 only (DMD and GDC not supported)
- `rust::Fn<R(A)>` requires `pragma(mangle)` — D TMP encodes the template arg
  as a pack `J..E` but cxx uses a function type `F..E`
- Async, D class polymorphism are out of scope

<br>

## References

- [D Interface to C++](https://dlang.org/spec/cpp_interface.html)
- [Calling C++ from D](https://wiki.dlang.org/Calling_C%2B%2B_from_D)
- [cxx.rs](https://cxx.rs/)

<br>

#### License

MIT OR Apache-2.0
