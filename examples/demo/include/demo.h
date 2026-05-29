#pragma once
// Companion header for the `demo` cxx::bridge.
// Strategy:
//   * Minimal D-implemented functions (no array bounds, no exceptions, no
//     stdcpp method calls) — those would pull druntime symbols that aren't
//     reachable in the consumer's link order without `--start-group`.
//   * Everything else lives inline in C++ here.

#include "rust/cxx.h"
#include <array>
#include <cstdint>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace demo {

// Shared types — cxx generates these, we just need name visibility.
enum class Verdict : int32_t;
struct Report;

// Opaque D handle. D-side has the body + destructor.
class DPayload {
public:
    ~DPayload() noexcept;
};

// ── D-implemented (small, no druntime dependencies) ─────────────────────────
::std::size_t demo_str_len(::rust::Str s) noexcept;
uint64_t      demo_sum_u8(::rust::Slice<const uint8_t> s) noexcept;
void          demo_fill(::rust::Slice<uint8_t> buf, uint8_t byte) noexcept;
void          demo_double_i32(::rust::Slice<int32_t> buf) noexcept;
Verdict       demo_next_verdict(Verdict v) noexcept;
int32_t       demo_report_count(const Report& r) noexcept;
int32_t       demo_vec_i32_sum(const ::rust::Vec<int32_t>& v) noexcept;
::rust::String demo_run_callback(
    ::rust::Fn<::rust::String(::rust::Str)> cb, ::rust::Str input) noexcept;
int32_t       demo_divide_safe(int32_t a, int32_t b) noexcept;

// ── Inline C++ ──────────────────────────────────────────────────────────────

inline ::rust::String demo_make_greeting(::rust::Str who) noexcept {
    ::std::string s = "hi from C++-helper, ";
    s.append(who.data(), who.size());
    return ::rust::String(s);
}

// UniquePtr: construct on the C++ heap (D side ships only ~DPayload).
inline ::std::unique_ptr<DPayload> demo_make_payload() noexcept {
    return ::std::unique_ptr<DPayload>(new DPayload());
}

// Result<T>: throw → cxx Err.
inline int32_t demo_divide(int32_t a, int32_t b) {
    if (b == 0) throw ::std::runtime_error("divide by zero");
    return demo_divide_safe(a, b);
}

// SharedPtr<DPayload>.
inline ::std::shared_ptr<DPayload> demo_make_shared_payload() noexcept {
    return ::std::make_shared<DPayload>();
}
inline ::std::size_t demo_shared_use_count(
    const ::std::shared_ptr<DPayload>& p) noexcept {
    return static_cast<::std::size_t>(p.use_count());
}

// CxxVector<int32_t>.
inline ::std::unique_ptr<::std::vector<int32_t>> demo_make_int_vector() noexcept {
    return ::std::unique_ptr<::std::vector<int32_t>>(
        new ::std::vector<int32_t>{10, 20, 30});
}
inline int32_t demo_int_vector_sum(const ::std::vector<int32_t>& v) noexcept {
    int32_t acc = 0;
    for (int32_t x : v) acc += x;
    return acc;
}

// CxxString.
inline ::std::size_t demo_cxx_string_len(const ::std::string& s) noexcept {
    return s.size();
}

// std::array<int, 4>.
inline int32_t demo_array_4_sum(const ::std::array<int32_t, 4>& a) noexcept {
    return a[0] + a[1] + a[2] + a[3];
}

} // namespace demo
