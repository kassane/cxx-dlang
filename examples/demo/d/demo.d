/**
 * D implementations for the `demo` cxx::bridge. Kept minimal — every fn here
 * is `@nogc nothrow @trusted` and avoids array bounds checks, exception
 * propagation, and stdcpp method calls so druntime/phobos are not required at
 * link time beyond what cxx-dlang's static archive already supplies.
 *
 * The richer std::* operations (SharedPtr, CxxVector, CxxString,
 * std::array, std::unique_ptr construction, throw/catch) live as inline C++
 * in include/demo.h.
 */
module demo;

import cxx_d;
import ldc.attributes : assumeUsed;

/// Mirror cxx-generated `enum class Verdict : int32_t`.
extern(C++, "demo") enum Verdict : int {
    Pass = 0,
    Fail = 1,
    Skip = 2,
}

/// Shared POD struct — D layout matches cxx-generated header.
extern(C++, "demo") struct Report {
    String name;
    int    count;
}

/// Opaque D handle — body holds nothing; dtor exists so std::unique_ptr's
/// default deleter can call ~DPayload() on the C++ side.
extern(C++, "demo") struct DPayload {
    @assumeUsed extern(C++) ~this() nothrow @nogc {}
}

extern(C++, "demo") nothrow {

    @assumeUsed pragma(inline, false)
    size_t demo_str_len(Str s) @nogc @trusted { return s.len; }

    @assumeUsed pragma(inline, false)
    ulong demo_sum_u8(Slice!(const(ubyte)) s) @nogc @trusted {
        ulong acc = 0;
        foreach (i; 0 .. s.len) acc += s.ptr[i];
        return acc;
    }

    @assumeUsed pragma(inline, false)
    void demo_fill(Slice!(ubyte) buf, ubyte byte_) @nogc @trusted {
        foreach (i; 0 .. buf.len) buf.ptr[i] = byte_;
    }

    @assumeUsed pragma(inline, false)
    void demo_double_i32(Slice!(int) buf) @nogc @trusted {
        foreach (i; 0 .. buf.len) buf.ptr[i] *= 2;
    }

    @assumeUsed pragma(inline, false)
    Verdict demo_next_verdict(Verdict v) @nogc {
        // `if` ladder avoids `final switch` (which pulls SwitchError +
        // _d_eh_personality and is hard to link from the consumer side).
        if (v == Verdict.Pass) return Verdict.Fail;
        if (v == Verdict.Fail) return Verdict.Skip;
        return Verdict.Pass;
    }

    @assumeUsed pragma(inline, false)
    int demo_report_count(ref const(Report) r) @nogc { return r.count; }

    @assumeUsed pragma(inline, false)
    int demo_vec_i32_sum(ref const(Vec!int) v) @nogc @trusted {
        int acc = 0;
        const n = v.size();
        const p = v.data();
        foreach (i; 0 .. n) acc += p[i];
        return acc;
    }

    @assumeUsed pragma(inline, false)
    int demo_divide_safe(int a, int b) @nogc { return a / b; }

    // rust::Fn callback — pin Itanium symbol via pragma(mangle).
    version (Windows) {} else {
        @assumeUsed pragma(inline, false)
        pragma(mangle, "_ZN4demo17demo_run_callbackEN4rust10cxxbridge12FnIFNS1_6StringENS1_3StrEEEES4_")
        String demo_run_callback(Fn!(String, Str) cb, Str input) @trusted {
            return cb.trampoline(input, cb.fn_);
        }
    }
}
