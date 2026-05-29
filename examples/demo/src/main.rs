//! End-to-end consumer smoke for `cxx-dlang`.

#[cxx::bridge(namespace = "demo")]
mod bridge {
    /// Shared C-style enum — exercises cxx's enum-class transport.
    #[repr(i32)]
    enum Verdict {
        Pass = 0,
        Fail = 1,
        Skip = 2,
    }

    /// Shared POD struct — round-trips a name + count.
    pub struct Report {
        pub name: String,
        pub count: i32,
    }

    extern "Rust" {
        type RustCounter;
        fn make_counter(start: i32) -> Box<RustCounter>;
        fn bump(c: &mut RustCounter) -> i32;
    }

    unsafe extern "C++" {
        include!("demo/include/demo.h");

        // ── primitives + str + slice ──────────────────────────────────────
        fn demo_str_len(s: &str) -> usize;
        fn demo_sum_u8(s: &[u8]) -> u64;
        fn demo_fill(buf: &mut [u8], byte: u8);
        fn demo_double_i32(buf: &mut [i32]);

        // ── shared types ──────────────────────────────────────────────────
        fn demo_next_verdict(v: Verdict) -> Verdict;
        fn demo_report_count(r: &Report) -> i32;

        // ── rust::String / rust::Vec ─────────────────────────────────────
        fn demo_make_greeting(who: &str) -> String;
        fn demo_vec_i32_sum(v: &Vec<i32>) -> i32;

        // ── opaque handle round-trip via UniquePtr<T> ────────────────────
        type DPayload;
        fn demo_make_payload() -> UniquePtr<DPayload>;

        // ── callback through rust::Fn<R(A)> ──────────────────────────────
        fn demo_run_callback(cb: fn(&str) -> String, input: &str) -> String;

        // ── Result<T>: C++ throws std::runtime_error → Rust Err ──────────
        fn demo_divide(a: i32, b: i32) -> Result<i32>;

        // ── SharedPtr<T>: refcount tracing via inline C++ helpers ────────
        fn demo_make_shared_payload() -> SharedPtr<DPayload>;
        fn demo_shared_use_count(p: &SharedPtr<DPayload>) -> usize;

        // ── CxxVector<T>: produce + sum via inline C++ ───────────────────
        fn demo_make_int_vector() -> UniquePtr<CxxVector<i32>>;
        fn demo_int_vector_sum(v: &CxxVector<i32>) -> i32;

        // ── CxxString: D reads the size + first byte ─────────────────────
        fn demo_cxx_string_len(s: &CxxString) -> usize;

        // ── std::array<T, N> via CxxArray alias ──────────────────────────
        fn demo_array_4_sum(a: &[i32; 4]) -> i32;
    }
}

pub struct RustCounter {
    value: i32,
}

fn make_counter(start: i32) -> Box<RustCounter> {
    cxx::private::prevent_unwind("make_counter", || Box::new(RustCounter { value: start }))
}

fn bump(c: &mut RustCounter) -> i32 {
    cxx::private::prevent_unwind("bump", || {
        c.value += 1;
        c.value
    })
}

fn relay(s: &str) -> String {
    format!("[D→Rust callback] {s}")
}

fn main() {
    use bridge::Verdict;

    println!("=== cxx-dlang full-parity demo ===\n");

    // primitives
    assert_eq!(bridge::demo_str_len("hello, D!"), 9);
    let bytes = [1u8, 2, 3, 4, 10];
    assert_eq!(bridge::demo_sum_u8(&bytes), 20);
    println!("  demo_str_len + demo_sum_u8       ok");

    // mutable slices
    let mut buf = [0u8; 6];
    bridge::demo_fill(&mut buf, b'X');
    assert_eq!(&buf, b"XXXXXX");
    let mut nums = [1i32, 2, 3, 4];
    bridge::demo_double_i32(&mut nums);
    assert_eq!(nums, [2, 4, 6, 8]);
    println!("  demo_fill + demo_double_i32     ok");

    // shared enum + shared struct (cxx-generated enums only derive PartialEq; no Debug)
    assert!(bridge::demo_next_verdict(Verdict::Pass) == Verdict::Fail);
    assert!(bridge::demo_next_verdict(Verdict::Fail) == Verdict::Skip);
    assert!(bridge::demo_next_verdict(Verdict::Skip) == Verdict::Pass);
    let report = bridge::Report {
        name: "alice".into(),
        count: 7,
    };
    assert_eq!(bridge::demo_report_count(&report), 7);
    println!("  shared enum + shared struct      ok");

    // String + Vec
    let greet = bridge::demo_make_greeting("world");
    assert!(greet.contains("world"), "greeting: {greet}");
    let v: Vec<i32> = vec![1, 2, 3, 4, 5];
    assert_eq!(bridge::demo_vec_i32_sum(&v), 15);
    println!("  String roundtrip + Vec<i32> sum  ok");

    // UniquePtr opaque
    let payload = bridge::demo_make_payload();
    assert!(!payload.is_null());
    println!("  UniquePtr<DPayload>              ok");

    // Callback
    let result = bridge::demo_run_callback(relay, "ping");
    assert_eq!(result, "[D→Rust callback] ping");
    println!("  rust::Fn callback                ok");

    // Result<T>
    assert_eq!(bridge::demo_divide(20, 4).unwrap(), 5);
    let err = bridge::demo_divide(7, 0).unwrap_err();
    assert!(err.what().contains("divide by zero"));
    println!("  Result<T> via C++ throw          ok");

    // SharedPtr refcount
    let sp1 = bridge::demo_make_shared_payload();
    assert_eq!(bridge::demo_shared_use_count(&sp1), 1);
    let sp2 = sp1.clone();
    assert_eq!(bridge::demo_shared_use_count(&sp1), 2);
    drop(sp2);
    assert_eq!(bridge::demo_shared_use_count(&sp1), 1);
    println!("  SharedPtr<T> refcount            ok");

    // CxxVector
    let cv = bridge::demo_make_int_vector();
    assert_eq!(bridge::demo_int_vector_sum(cv.as_ref().unwrap()), 60);
    println!("  CxxVector<i32> sum               ok");

    // CxxString
    cxx::let_cxx_string!(s = "stdcpp-string");
    assert_eq!(bridge::demo_cxx_string_len(&s), 13);
    println!("  CxxString len                    ok");

    // std::array
    assert_eq!(bridge::demo_array_4_sum(&[10, 20, 30, 40]), 100);
    println!("  std::array<int,4> sum            ok");

    // Rust opaque handle exposed back to D-driven code
    let mut counter = make_counter(99);
    assert_eq!(bump(&mut counter), 100);
    println!("  RustCounter (extern Rust)        ok");

    println!("\n✔ every binding category exercised end-to-end.");
}
