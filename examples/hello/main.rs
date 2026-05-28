fn main() {
    let result = cxx_dlang::ffi::rust_greet("world");
    println!("{}", result);
}
