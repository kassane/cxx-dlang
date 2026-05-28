#pragma once
/// Companion C++ header for the cxx::bridge `unsafe extern "C++"` block.
/// Include paths for "rust/cxx.h" are injected by cxx_build::bridge() at build time.
#include "rust/cxx.h"
#include <cstdint>
#include <memory>

namespace cxx_d {

/// Opaque D handle compiled by LDC2. Must be a complete type so
/// std::unique_ptr can instantiate its deleter. Body lives in d/cxx_d.d.
class DHandle {
public:
    ~DHandle() noexcept;
};

/// Shared struct — layout must match Greeting in src/ffi.rs exactly.
struct Greeting;

// D-implemented functions callable from Rust via the cxx bridge.
int32_t d_double(int32_t x) noexcept;
::std::unique_ptr<DHandle> d_make_handle() noexcept;
::rust::String d_run_callback(::rust::Fn<::rust::String(::rust::Str)> cb, ::rust::Str input) noexcept;
bool d_identity_bool(bool x) noexcept;
double d_add_f64(double a, double b) noexcept;
::std::size_t d_str_len(::rust::Str s) noexcept;
int32_t d_greeting_count(const Greeting &g) noexcept;

} // namespace cxx_d
