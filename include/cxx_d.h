#pragma once
#include "rust/cxx.h"
#include <cstdint>
#include <memory>

// Companion header for the cxx::bridge unsafe extern "C++" block.
// Include paths for "rust/cxx.h" are injected by cxx_build::bridge() at build time.

namespace cxx_d {

// DHandle: opaque D class type compiled by LDC2.
// Must be a complete type here so std::unique_ptr can instantiate its deleter.
// The actual definition lives in d/cxx_d.d; this stub satisfies the C++ side.
class DHandle {
public:
    ~DHandle() noexcept;
};

// Shared struct — must match Greeting in src/ffi.rs exactly.
struct Greeting;

// Free functions implemented in D (d/cxx_d.d), compiled by LDC2
int32_t d_double(int32_t x) noexcept;
::std::unique_ptr<DHandle> d_make_handle() noexcept;
::rust::String d_run_callback(::rust::Fn<::rust::String(::rust::Str)> cb, ::rust::Str input) noexcept;

bool d_identity_bool(bool x) noexcept;
double d_add_f64(double a, double b) noexcept;
::std::size_t d_str_len(::rust::Str s) noexcept;
int32_t d_greeting_count(const Greeting &g) noexcept;

} // namespace cxx_d
