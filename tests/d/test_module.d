// Test D module compiled unconditionally by build.rs
module test_module;

import cxx_d;

extern(C++, "cxx_d") {
    extern(C++) int test_d_primitive(int x) nothrow {
        return d_double(x);
    }

    extern(C++) String test_d_reverse_str(Str input) nothrow {
        // Return the input as a String for now (full reverse is a follow-up)
        return String(); // placeholder — proper impl in next iteration
    }
}
