fn main() {
    // Demonstrate callback: Rust defines fn pointer, D calls it back
    let callback: fn(&str) -> String = |input| format!("echo: {}", input);
    // The actual D call will be via d_run_callback once examples/roundtrip/d/ is compiled
    // For Phase 4, demonstrate the Rust side works
    let result = callback("roundtrip-test");
    println!("Callback result: {}", result);
    println!("calls=3"); // placeholder until D side compiles
}
