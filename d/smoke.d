// Smoke test: verifies LDC2 can parse and compile the bridge module.
module smoke;

import cxx_d;

// Simple sanity check — callable from build.rs to verify compilation works.
extern(C++, "cxx_d") int smoke_test() nothrow {
    return d_double(21); // should return 42
}
