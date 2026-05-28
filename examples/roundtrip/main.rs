// Demonstrates the callback roundtrip: Rust passes a fn pointer to D,
// D calls it back with a &str argument and returns the resulting String.
// This exercises rust::Fn<String(Str)> across the C++ ABI boundary.

fn main() {
    use cxx_dlang::ffi::bridge;

    // Pass a Rust closure to D; D invokes cb.trampoline(input, cb.fn_)
    let result = bridge::d_run_callback(|s| format!("[{s}]"), "hello");
    println!("callback result: {result}");

    // D reads the length of a rust::Str without copying
    let s = "roundtrip";
    let len = bridge::d_str_len(s);
    println!("d_str_len({s:?}) = {len}");

    // Opaque D handle — allocated on the C++ heap, dropped by UniquePtr
    let handle = bridge::d_make_handle();
    println!("d_make_handle is_null: {}", handle.is_null());

    // Opaque Rust handle — allocated in Rust, described back through the bridge
    let rhandle = cxx_dlang::ffi::make_handle();
    let desc = cxx_dlang::ffi::handle_describe(&rhandle);
    println!("rust handle: {desc}");

    println!("calls=4");
}
