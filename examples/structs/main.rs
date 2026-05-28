// Demonstrates the shared Greeting struct: same memory layout on both sides.
// D reads the `count` field via a const-ref parameter (parity: cxx test_c_method_calls).

fn main() {
    use cxx_dlang::ffi::bridge::{self, Greeting};

    // Construct on the Rust side; D reads a field via const ref
    let g = Greeting {
        name: "alice".to_string(),
        count: 6,
    };
    let count = bridge::d_greeting_count(&g);
    println!("Greeting {{ name: {:?}, count: {count} }}", g.name);

    // Static size assertion: String(24B) + i32(4B) + 4B padding = 32B
    assert_eq!(std::mem::size_of::<Greeting>(), 32);
    println!("Greeting::sizeof == {}", std::mem::size_of::<Greeting>());
}
