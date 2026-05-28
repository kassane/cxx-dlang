// Demonstrates bidirectional calls across the Rust↔D bridge:
//   - Rust implements rust_greet(), called here directly
//   - D implements d_double(), called via the cxx bridge

fn main() {
    // Rust → Rust (exposed to D via extern "Rust" block)
    let greeting = cxx_dlang::ffi::rust_greet("D");
    println!("{greeting}");

    // Rust → D: integer doubling implemented in d/cxx_d.d
    let input = 21_i32;
    let doubled = cxx_dlang::ffi::bridge::d_double(input);
    println!("d_double({input}) = {doubled}");

    // Rust → D: f64 arithmetic
    let sum = cxx_dlang::ffi::bridge::d_add_f64(1.5, 2.5);
    println!("d_add_f64(1.5, 2.5) = {sum}");

    // Rust → D: bool identity
    let flag = cxx_dlang::ffi::bridge::d_identity_bool(true);
    println!("d_identity_bool(true) = {flag}");
}
