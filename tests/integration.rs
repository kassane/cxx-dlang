//! Exhaustive smoke tests for the D bindings in `d/cxx_d.d`.
//!
//! Each `#[test]` writes a tiny fixture `.d` file, compiles it against
//! `cxx_d` under the LDC2 safety previews, and asserts success. Categories
//! mirror the cxx.rs test taxonomy (`cxx/tests/test.rs`): primitive layouts,
//! string types, slices (every width × const/mut), vectors, callback `Fn`
//! signatures, `unique_ptr` instantiations, and cross-cutting invariants.
//!
//! Toolchain discovery: `$DC` → `$LDC2_PATH` → bare `ldc2` (relies on PATH).

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const LDC2_FLAGS: &[&str] = &[
    "--edition=2025",
    "--preview=safer",
    "--preview=dip1000",
    "--preview=nosharedaccess",
    "--preview=fixImmutableConv",
    "--preview=systemVariables",
    "--extern-std=c++17",
    "-relocation-model=pic",
];

/// Compiles a fixture wrapped with `module $name; import cxx_d;`.
/// Each call gets its own tmp file (atomic counter) so tests run in parallel.
fn check(name: &str, body: &str) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);

    let ldc2 = locate_ldc2();
    let bindings_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("d");

    let tmp_dir = std::env::temp_dir().join("cxx-dlang-tests");
    std::fs::create_dir_all(&tmp_dir).expect("tmp dir");
    let src_path = tmp_dir.join(format!("{name}_{id}.d"));
    let obj_path = tmp_dir.join(format!("{name}_{id}.o"));

    let full = format!("module fixture_{id};\nimport cxx_d;\n\n{body}\n");
    std::fs::write(&src_path, &full).expect("write fixture");

    let out = Command::new(&ldc2)
        .args(LDC2_FLAGS)
        .arg("-c")
        .arg(format!("-of={}", obj_path.display()))
        .arg(format!("-I={}", bindings_dir.display()))
        .arg(&src_path)
        .output()
        .expect("ldc2 must be invokable (set $DC or have ldc2 on PATH)");

    assert!(
        out.status.success(),
        "[{name}] fixture failed:\n--- source ---\n{full}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn locate_ldc2() -> PathBuf {
    for var in ["DC", "LDC2_PATH"] {
        if let Ok(path) = std::env::var(var) {
            let p = PathBuf::from(&path);
            if p.is_file() {
                return p;
            }
            let p2 = p.join("ldc2");
            if p2.is_file() {
                return p2;
            }
        }
    }
    PathBuf::from("ldc2")
}

// One #[test] per fixture; macro keeps boilerplate down.
macro_rules! d_check {
    ($name:ident, $body:expr) => {
        #[test]
        fn $name() {
            check(stringify!($name), $body);
        }
    };
}

// ── rust::Str ────────────────────────────────────────────────────────────────
d_check!(str_sizeof_16, "static assert(Str.sizeof == 16);");
d_check!(
    str_field_ptr_type,
    "static assert(is(typeof(Str.init.ptr) == const(char)*));"
);
d_check!(
    str_field_len_type,
    "static assert(is(typeof(Str.init.len) == size_t));"
);
d_check!(
    str_two_fields_only,
    "static assert(Str.tupleof.length == 2);"
);
d_check!(
    str_default_init,
    "Str s; static assert(__traits(compiles, Str()));"
);
d_check!(
    str_extern_fn_param,
    "extern(C++) size_t f(Str s) nothrow @nogc @trusted { return s.len; }"
);
d_check!(
    str_extern_fn_return,
    "extern(C++) Str f() nothrow @nogc @trusted { return Str(null, 0); }"
);
d_check!(
    str_ref_const_param,
    "extern(C++) size_t f(ref const(Str) s) nothrow @nogc @trusted { return s.len; }"
);

// ── rust::String ─────────────────────────────────────────────────────────────
d_check!(string_sizeof_24, "static assert(String.sizeof == 24);");
d_check!(
    string_three_words,
    "static assert(String.sizeof == 3 * size_t.sizeof);"
);
d_check!(
    string_extern_fn_param,
    "extern(C++) void f(String s) nothrow @nogc @trusted {}"
);
d_check!(
    string_extern_fn_return,
    "extern(C++) String f() nothrow @nogc @trusted { String s; return s; }"
);
d_check!(
    string_ref_const_param,
    "extern(C++) void f(ref const(String) s) nothrow @nogc @trusted {}"
);
d_check!(
    string_default_constructs,
    "void f() nothrow @nogc @trusted { String s; cast(void) s; }"
);

// ── rust::Slice<T> — sizeof for every supported element type, const + mut ────
d_check!(
    slice_const_byte_sizeof,
    "static assert(Slice!(const(byte)).sizeof   == 16);"
);
d_check!(
    slice_byte_sizeof,
    "static assert(Slice!(byte).sizeof          == 16);"
);
d_check!(
    slice_const_ubyte_sizeof,
    "static assert(Slice!(const(ubyte)).sizeof  == 16);"
);
d_check!(
    slice_ubyte_sizeof,
    "static assert(Slice!(ubyte).sizeof         == 16);"
);
d_check!(
    slice_const_short_sizeof,
    "static assert(Slice!(const(short)).sizeof  == 16);"
);
d_check!(
    slice_short_sizeof,
    "static assert(Slice!(short).sizeof         == 16);"
);
d_check!(
    slice_const_ushort_sizeof,
    "static assert(Slice!(const(ushort)).sizeof == 16);"
);
d_check!(
    slice_ushort_sizeof,
    "static assert(Slice!(ushort).sizeof        == 16);"
);
d_check!(
    slice_const_int_sizeof,
    "static assert(Slice!(const(int)).sizeof    == 16);"
);
d_check!(
    slice_int_sizeof,
    "static assert(Slice!(int).sizeof           == 16);"
);
d_check!(
    slice_const_uint_sizeof,
    "static assert(Slice!(const(uint)).sizeof   == 16);"
);
d_check!(
    slice_uint_sizeof,
    "static assert(Slice!(uint).sizeof          == 16);"
);
d_check!(
    slice_const_long_sizeof,
    "static assert(Slice!(const(long)).sizeof   == 16);"
);
d_check!(
    slice_long_sizeof,
    "static assert(Slice!(long).sizeof          == 16);"
);
d_check!(
    slice_const_ulong_sizeof,
    "static assert(Slice!(const(ulong)).sizeof  == 16);"
);
d_check!(
    slice_ulong_sizeof,
    "static assert(Slice!(ulong).sizeof         == 16);"
);
d_check!(
    slice_const_float_sizeof,
    "static assert(Slice!(const(float)).sizeof  == 16);"
);
d_check!(
    slice_float_sizeof,
    "static assert(Slice!(float).sizeof         == 16);"
);
d_check!(
    slice_const_double_sizeof,
    "static assert(Slice!(const(double)).sizeof == 16);"
);
d_check!(
    slice_double_sizeof,
    "static assert(Slice!(double).sizeof        == 16);"
);
d_check!(
    slice_const_bool_sizeof,
    "static assert(Slice!(const(bool)).sizeof   == 16);"
);
d_check!(
    slice_bool_sizeof,
    "static assert(Slice!(bool).sizeof          == 16);"
);

// rust::Slice<T> — structural traits.
d_check!(
    slice_two_fields,
    "static assert(Slice!int.tupleof.length == 2);"
);
d_check!(
    slice_const_ptr_type,
    "static assert(is(typeof(Slice!(const(int)).init.ptr) == const(int)*));"
);
d_check!(
    slice_mut_ptr_type,
    "static assert(is(typeof(Slice!(int).init.ptr) == int*));"
);
d_check!(
    slice_len_type,
    "static assert(is(typeof(Slice!int.init.len) == size_t));"
);
d_check!(
    slice_extern_fn_param,
    "extern(C++) size_t f(Slice!(const(ubyte)) s) nothrow @nogc @trusted { return s.len; }"
);
d_check!(
    slice_extern_fn_return,
    "extern(C++) Slice!(const(ubyte)) f() nothrow @nogc @trusted { return Slice!(const(ubyte))(null, 0); }"
);

// ── rust::Vec<T> ─────────────────────────────────────────────────────────────
d_check!(
    vec_size_method_exists,
    "static assert(__traits(compiles, (ref const(Vec!int) v) => v.size()));"
);
d_check!(
    vec_data_method_exists,
    "static assert(__traits(compiles, (ref const(Vec!int) v) => v.data()));"
);
d_check!(
    vec_data_returns_const_ptr,
    "static assert(is(typeof((ref const(Vec!int) v) => v.data()) R == return) && is(R == const(int)*));"
);
d_check!(
    vec_size_returns_size_t,
    "static assert(is(typeof((ref const(Vec!int) v) => v.size()) R == return) && is(R == size_t));"
);
d_check!(
    vec_default_ctor_disabled,
    "static assert(!__traits(compiles, Vec!int()));"
);
d_check!(
    vec_postblit_disabled,
    "static assert(!__traits(compiles, { Vec!int a; Vec!int b = a; }));"
);
d_check!(
    vec_extern_fn_ref_param,
    "extern(C++) size_t f(ref const(Vec!int) v) nothrow @nogc @trusted { return v.size(); }"
);
d_check!(
    vec_with_double,
    "alias V = Vec!double; extern(C++) void f(ref const(V) v) nothrow @nogc @trusted {}"
);
d_check!(
    vec_with_long,
    "alias V = Vec!long;   extern(C++) void f(ref const(V) v) nothrow @nogc @trusted {}"
);
d_check!(
    vec_with_string,
    "alias V = Vec!String; extern(C++) void f(ref const(V) v) nothrow @nogc @trusted {}"
);

// ── rust::Fn<R(A...)> ────────────────────────────────────────────────────────
d_check!(fn_void_return, "static assert(Fn!(void).sizeof == 16);");
d_check!(fn_int_no_args, "static assert(Fn!(int).sizeof == 16);");
d_check!(fn_int_one_arg, "static assert(Fn!(int, int).sizeof == 16);");
d_check!(
    fn_int_two_args,
    "static assert(Fn!(int, int, int).sizeof == 16);"
);
d_check!(
    fn_str_to_string,
    "static assert(Fn!(String, Str).sizeof == 16);"
);
d_check!(
    fn_slice_arg,
    "static assert(Fn!(size_t, Slice!(const(ubyte))).sizeof == 16);"
);
d_check!(
    fn_three_args,
    "static assert(Fn!(double, int, int, int).sizeof == 16);"
);
d_check!(
    fn_trampoline_field,
    "static assert(typeof(Fn!(int).init.trampoline).sizeof == size_t.sizeof);"
);
d_check!(
    fn_fn_field,
    "static assert(is(typeof(Fn!(int).init.fn_) == void*));"
);
d_check!(
    fn_extern_param,
    "alias F = Fn!(int, int); extern(C++) int f(F cb) nothrow @trusted { return cb.trampoline(0, cb.fn_); }"
);

// ── std::unique_ptr<T, D> ────────────────────────────────────────────────────
d_check!(
    unique_ptr_sizeof_payload,
    "struct P {} static assert(unique_ptr!P.sizeof == size_t.sizeof);"
);
d_check!(
    unique_ptr_two_param,
    "struct P {} alias D = default_delete!P; static assert(__traits(compiles, unique_ptr!(P, D)()));"
);
d_check!(
    unique_ptr_int_sizeof,
    "static assert(unique_ptr!int.sizeof == 8);"
);
d_check!(
    unique_ptr_has_destructor,
    "struct P {} static assert(__traits(hasMember, unique_ptr!P, \"__dtor\"));"
);
d_check!(
    unique_ptr_ptr_field,
    "struct P {} static assert(is(typeof(unique_ptr!P.init._ptr) == P*));"
);
d_check!(
    unique_ptr_extern_return,
    "struct P {} extern(C++) unique_ptr!P f() nothrow @nogc @trusted { unique_ptr!P r; return r; }"
);

// ── std::default_delete<T> ───────────────────────────────────────────────────
d_check!(
    default_delete_stateless,
    "struct P {} static assert(default_delete!P.sizeof == 1);"
);
d_check!(
    default_delete_instantiates,
    "struct P {} default_delete!P d;"
);

// ── cross-cutting: nothrow / @nogc / @trusted plumbing ──────────────────────
d_check!(
    all_in_one_extern_block,
    r#"extern(C++) nothrow @nogc {
        size_t a(Str s)                          @trusted { return s.len; }
        size_t b(Slice!(const(ubyte)) s)         @trusted { return s.len; }
        size_t c(ref const(Vec!int) v)           @trusted { return v.size(); }
        void   d(String s)                       @trusted {}
        void   e(unique_ptr!int p)               @trusted { cast(void) p; }
    }"#
);
d_check!(empty_extern_block_compiles, "extern(C++) {}");
d_check!(
    fixture_with_unused_imports,
    "// blank fixture; just verifies import cxx_d; resolves cleanly"
);
d_check!(
    all_abi_sizes_one_assert,
    r#"static assert(Str.sizeof + String.sizeof + Slice!(const(ubyte)).sizeof
                     + Fn!(int).sizeof == 16 + 24 + 16 + 16);"#
);

// ── std::shared_ptr<T> ───────────────────────────────────────────────────────
d_check!(
    shared_ptr_int_sizeof,
    "static assert(shared_ptr!int.sizeof == 16);"
);
d_check!(
    shared_ptr_payload_int,
    "static assert(is(typeof(shared_ptr!int.init._ptr) == int*));"
);
d_check!(
    shared_ptr_extern_param_layout_only,
    "extern(C++) bool f(ref const(shared_ptr!int) p) nothrow @nogc @trusted { return p._ptr is null; }"
);
d_check!(
    cpp_runtime_version_defined,
    r#"version (CppRuntime_GNU)       static assert(true);
       else version (CppRuntime_LLVM) static assert(true);
       else version (CppRuntime_Microsoft) static assert(true);
       else static assert(false, "unsupported C++ runtime — extend version blocks");"#
);
d_check!(
    smart_ptr_methods_when_supported,
    r#"version (CppRuntime_LLVM) {
           extern(C++) long f(ref const(shared_ptr!int) p) nothrow @nogc @trusted { return p.use_count(); }
       } else version (CppRuntime_Microsoft) {
           extern(C++) long f(ref const(shared_ptr!int) p) nothrow @nogc @trusted { return p.use_count(); }
       }
       // libstdc++: skipped (methods inherited from private base; no direct symbol)"#
);
d_check!(
    weak_ptr_methods_when_supported,
    r#"version (CppRuntime_LLVM) {
           extern(C++) bool f(ref const(weak_ptr!int) p) nothrow @nogc @trusted { return p.expired(); }
       } else version (CppRuntime_Microsoft) {
           extern(C++) bool f(ref const(weak_ptr!int) p) nothrow @nogc @trusted { return p.expired(); }
       }"#
);
d_check!(
    shared_ptr_with_payload_struct,
    "struct P {} static assert(shared_ptr!P.sizeof == 16);"
);

// ── std::weak_ptr<T> ─────────────────────────────────────────────────────────
d_check!(
    weak_ptr_int_sizeof,
    "static assert(weak_ptr!int.sizeof == 16);"
);
d_check!(
    weak_ptr_extern_param_layout_only,
    "extern(C++) bool f(ref const(weak_ptr!int) p) nothrow @nogc @trusted { return p._ptr is null; }"
);

// ── std::vector<T> (hand-rolled binding) ─────────────────────────────────────
d_check!(
    vector_int_sizeof_3ptrs,
    "static assert(vector!int.sizeof == 3 * (void*).sizeof);"
);
d_check!(
    vector_default_alloc_param,
    "static assert(is(vector!int == vector!(int, allocator!int)));"
);
d_check!(
    vector_size_method,
    "extern(C++) size_t f(ref const(vector!int) v) nothrow @nogc @trusted { return v.size(); }"
);
d_check!(
    vector_data_method,
    "extern(C++) const(int)* f(ref const(vector!int) v) nothrow @nogc @trusted { return v.data(); }"
);
d_check!(
    vector_empty_method,
    "extern(C++) bool f(ref const(vector!int) v) nothrow @nogc @trusted { return v.empty(); }"
);
d_check!(
    vector_with_double,
    "extern(C++) size_t f(ref const(vector!double) v) nothrow @nogc @trusted { return v.size(); }"
);
d_check!(
    vector_with_long,
    "extern(C++) const(long)* f(ref const(vector!long) v) nothrow @nogc @trusted { return v.data(); }"
);

// ── std::allocator<T> ────────────────────────────────────────────────────────
d_check!(
    allocator_stateless,
    "static assert(allocator!int.sizeof == 1);"
);
d_check!(
    allocator_instantiates_per_type,
    "static assert(__traits(compiles, allocator!int()) && __traits(compiles, allocator!double()));"
);

// ── CxxString (alias for basic_string!char) ──────────────────────────────────
d_check!(
    cxx_string_alias_exists,
    "static assert(is(CxxString == basic_string!char));"
);
d_check!(
    cxx_string_size_method,
    "extern(C++) size_t f(ref const(CxxString) s) nothrow @nogc @trusted { return s.size(); }"
);
d_check!(
    cxx_string_index,
    "extern(C++) char f(ref const(CxxString) s) nothrow @nogc @trusted { return s[0]; }"
);

// ── CxxArray (alias for core.stdcpp.array.array) ─────────────────────────────
d_check!(
    cxx_array_4ints_sizeof,
    "static assert(CxxArray!(int, 4).sizeof == 16);"
);
d_check!(
    cxx_array_subscript,
    "extern(C++) int f(ref const(CxxArray!(int, 8)) a) nothrow @nogc @trusted { return a[0]; }"
);
d_check!(
    cxx_array_size_is_n,
    "static assert(CxxArray!(int, 7).sizeof == 7 * int.sizeof);"
);

// ── Cross-cutting: full surface in one fixture ───────────────────────────────
d_check!(
    full_surface_in_one_block,
    r#"struct P {}
       extern(C++) nothrow @trusted {
           size_t       a(Str s)                          @nogc { return s.len; }
           size_t       b(Slice!(const(int)) s)           @nogc { return s.len; }
           size_t       c(ref const(Vec!int) v)           @nogc { return v.size(); }
           void         d(String s)                       @nogc {}
           const(int)*  e(ref const(unique_ptr!int) u)    @nogc { return u._ptr; }
           bool         f(ref const(shared_ptr!int) s)    @nogc { return s._ptr is null; }
           bool         g(ref const(weak_ptr!int) w)      @nogc { return w._ptr is null; }
           size_t       h(ref const(vector!int) v)        @nogc { return v.size(); }
           size_t       i(ref const(CxxString) s)         @nogc { return s.size(); }
           int          j(ref const(CxxArray!(int,4)) a)  @nogc { return a[0]; }
       }"#
);
