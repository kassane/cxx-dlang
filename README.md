# cxx-dlang — Rust ↔ D bindings + LDC2 build glue for cxx.rs

[![CI](https://github.com/kassane/cxx-dlang/actions/workflows/ci.yml/badge.svg)](https://github.com/kassane/cxx-dlang/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cxx-dlang.svg)](https://crates.io/crates/cxx-dlang)
[![docs.rs](https://docs.rs/cxx-dlang/badge.svg)](https://docs.rs/cxx-dlang)
[![license](https://img.shields.io/crates/l/cxx-dlang.svg)](#license)

Reusable D-side bindings for the cxx.rs `rust::*` types plus a LDC2 build
pipeline that downstream crates can adopt without rewriting the toolchain
glue every time.

This crate is **build-only**: it does not define an FFI surface of its own.
Consumers ship their own `#[cxx::bridge]`, import the D module `cxx_d`, and
link against the static archive produced here.

```toml
[dependencies]
cxx = "1.0"
cxx-dlang = "0.2"

[build-dependencies]
cxx-build = "1.0"
```

*Requires LDC2 ≥ 1.40, Rust edition 2024 (rustc 1.85+). CI matrix: Linux / macOS / Windows MSVC.*

Set `$DC` (D-toolchain convention, e.g. `DC=ldc2`) or `$LDC2_PATH` if `ldc2` is
not on `$PATH`.

<br>

## What you get

**D-side type bindings** (`d/cxx_d.d`, imported as `import cxx_d;`):

| D binding | C++ equivalent | Notes |
|-----------|----------------|-------|
| `Str`                       | `rust::Str` (16B)              | borrowed UTF-8 slice |
| `String`                    | `rust::String` (24B)           | owned UTF-8, opaque repr |
| `Slice(T)`                  | `rust::Slice<T>` (16B)         | mutable or const via element constness |
| `Fn(R, A...)`               | `rust::Fn<R(A...)>`            | 16B `{trampoline, fn_}`; consumers need `pragma(mangle)` |
| `Vec(T)`                    | `rust::Vec<T>`                 | `size()` + `data()` direct on every stdlib |
| `unique_ptr(T, D=default_delete!T)` | `std::unique_ptr<T, D>` | hand-rolled (LDC2 1.42 `core.stdcpp.memory` is broken on libstdc++) |
| `default_delete(T)`         | `std::default_delete<T>`       | stateless, needed for MSVC decoration |
| `shared_ptr(T)`             | `std::shared_ptr<T>`           | 16B; `get()`/`use_count()` bound on libc++ + MSVC (libstdc++ delegates to private `__shared_ptr`, use inline-C++ helper) |
| `weak_ptr(T)`               | `std::weak_ptr<T>`             | 16B; `use_count()`/`expired()` same `version()`-gated availability as `shared_ptr` |
| `vector(T, Alloc=allocator!T)` | `std::vector<T>` (24B) | `size()` / `data()` / `empty()` direct on all three stdlibs |
| `allocator(T)`              | `std::allocator<T>`            | empty stateless type; encoding placeholder for two-template-param `vector` mangling |
| `CxxString` (alias for `basic_string!char`) | `std::string` | via `core.stdcpp.string` |
| `CxxArray(T, N)` (alias for `array!(T, N)`) | `std::array<T, N>` | via `core.stdcpp.array` |

Per-runtime adaptations (driven by D's predefined `version (CppRuntime_*)`):

- `CppRuntime_GNU` (Linux libstdc++) → `pragma(lib, "stdc++")`; smart-pointer methods *omitted*; use inline-C++ trampolines in the consumer's bridge header.
- `CppRuntime_LLVM` (macOS / libc++) → `pragma(lib, "c++")`; smart-pointer methods *direct*.
- `CppRuntime_Microsoft` (Windows MSVC) → linked automatically; smart-pointer methods *direct*.

(Validated empirically via `zig c++ -c -target {x86_64-linux-gnu, aarch64-macos-none, x86_64-windows-msvc}` + `llvm-nm` — see CLAUDE.md for the methodology.)

**Build pipeline** (`build.rs`):

- Locates `ldc2` via `$DC` → `$LDC2_PATH` → walks `$PATH` → `~/.dlang/ldc2-*/bin/ldc2`.
- Compiles every `.d` file under `d/` with the curated safety-preview list.
- Probes the LDC2 lib dir across Linux (`lib/`), macOS universal (`lib-arm64/`, `lib-x86_64/`) and Windows MSVC (`lib64/`).
- Bundles the D objects into `libcxx_d_dlib.a` (named after the crate's `links =` key) and links druntime + phobos2 statically.

LDC2 `--preview=` safety flags applied to every compilation unit:
`safer`, `dip1000`, `nosharedaccess`, `fixImmutableConv`, `systemVariables`.

<br>

## Sketching a consumer

```rust
// src/ffi.rs in the consumer crate
#[cxx::bridge(namespace = "my_app")]
mod bridge {
    extern "Rust" { /* ... */ }

    unsafe extern "C++" {
        include!("my_app/include/my_app.h");
        // declare D-implemented functions here
    }
}
```

```d
// d/my_app.d
module my_app;
import cxx_d; // Str, String, Slice(T), Vec(T), Fn(R,A...), unique_ptr,
              // shared_ptr, weak_ptr, vector, CxxString, CxxArray, …

extern(C++, "my_app") nothrow {
    @assumeUsed pragma(inline, false)
    size_t my_app_str_len(Str s) @nogc { return s.len; }
}
```

The consumer's `build.rs` runs `cxx_build::bridge("src/ffi.rs")`, adds its own
`d/my_app.d` as an extra object (compiled with LDC2 the same way `cxx-dlang`'s
own build does), and links everything together. `cxx-dlang` contributes the
`rust::*` D bindings and the druntime/phobos linkage.

<br>

## Testing

```bash
cargo test --tests
# test result: ok. 99 passed; 0 failed
```

`tests/integration.rs` writes one tiny fixture `.d` file per `#[test]` and
compiles it under the safety previews:

- every binding × `{const, mut}` × all D integer/float widths (Slice, Vec)
- ABI invariants via D `static assert` (`Str.sizeof == 16`, `String.sizeof == 24`,
  `Slice!T.sizeof == 16`, `Vec(T)` methods present, `Fn(R,A).sizeof == 16`, …)
- structural traits: field types, tuple lengths, disabled ctors / postblits
- `version (CppRuntime_*)` conditional smart-ptr method probes
- a "full surface" fixture that uses every binding in a single `extern(C++)` block

End-to-end validation against a real consumer lives in
[`examples/demo/`](examples/demo) — a self-contained sub-package with its
own `Cargo.toml`, `build.rs`, `#[cxx::bridge]`, and `d/demo.d` that
exercises every C++ stdlib type category we ship (UniquePtr, SharedPtr
refcount, CxxVector, CxxString, std::array, `Result<T>` throw, callbacks,
shared structs/enums). CI runs it on every push:

```bash
cd examples/demo && cargo run --bin demo
# === cxx-dlang full-parity demo ===
#   demo_str_len + demo_sum_u8       ok
#   demo_fill + demo_double_i32      ok
#   shared enum + shared struct      ok
#   String roundtrip + Vec<i32> sum  ok
#   UniquePtr<DPayload>              ok
#   rust::Fn callback                ok
#   Result<T> via C++ throw          ok
#   SharedPtr<T> refcount            ok
#   CxxVector<i32> sum               ok
#   CxxString len                    ok
#   std::array<int,4> sum            ok
#   RustCounter (extern Rust)        ok
# ✔ every binding category exercised end-to-end.
```

<br>

## Safety contract

- **D fns called from Rust through cxx must be `nothrow`** — exceptions must
  not cross the C++ ABI boundary (UB).
- **Rust fns called from D must be wrapped** in `cxx::private::prevent_unwind`
  so panics turn into aborts at the boundary.
- **Allocator discipline**: D handles returned through `UniquePtr<T>` must be
  allocated with `__cpp_new_nothrow` from `core.stdcpp.new_`, not D's GC
  `new` — `std::unique_ptr`'s default deleter calls `operator delete`.
- **GC safety**: never store a Rust `Box<T>` / `Arc<T>` in a D `class` field;
  druntime will trace and corrupt the Rust heap. Use a Rust-side registry
  keyed by an integer instead.

<br>

## Known limitations

- LDC2 only — DMD and GDC are not supported.
- `rust::Fn<R(A)>` needs `pragma(mangle)` on consumer functions because D TMP
  encodes the template arg as a pack (`J..E`) while cxx uses a function type
  (`F..E`). Itanium and MSVC use distinct symbol shapes, so a `version(Windows)`
  branch is required when targeting both.
- On Linux libstdc++, `std::shared_ptr<T>::use_count` / `std::weak_ptr<T>::expired`
  delegate to the private base classes `__shared_ptr` / `__weak_ptr`, so the
  user-facing class emits no direct method symbols. The D bindings ship the
  methods only under `version(CppRuntime_LLVM)` / `version(CppRuntime_Microsoft)`;
  libstdc++ consumers should wrap them with an inline-C++ helper in their
  bridge header (see `cxx-dlang-demo` for the pattern).
- Out of scope: async futures, D class polymorphism, variant enums (Rust
  enums with payload). Canonical alternatives:
  [cxx-async](https://github.com/pcwalton/cxx-async),
  [cxx-enumext](https://github.com/Ryex/cxx-enumext).

<br>

## References

- [D Interface to C++](https://dlang.org/spec/cpp_interface.html)
- [Calling C++ from D](https://wiki.dlang.org/Calling_C%2B%2B_from_D)
- [cxx.rs](https://cxx.rs/)
- [dlang/stdcpp](https://github.com/dlang/stdcpp)

<br>

#### License

MIT OR Apache-2.0
