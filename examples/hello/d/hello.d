module hello;

import cxx_d;

// Called from the hello example to demonstrate D calling Rust
extern(C++, "cxx_d") {
    extern(C++) void d_print_greeting(Str name) nothrow {
        // In a real impl, this would call rust_greet via the bridge
        // For Phase 4, this is a placeholder
    }
}
