// Consumer build script — demonstrates how a downstream crate consumes
// `cxx-dlang`. Now lives at `cxx-dlang/examples/demo/`, so the path to the
// shipped D bindings (`../../d/cxx_d.d`) goes up two levels.
//
// Pipeline:
//   1. Locate ldc2 ($DC → $LDC2_PATH → walk $PATH → ~/.dlang/ldc2-*/bin/ldc2).
//   2. Compile this crate's d/demo.d with -I<cxx-dlang>/d so `import cxx_d;`
//      resolves to the bindings shipped by cxx-dlang.
//   3. Run cxx_build::bridge("src/main.rs") and attach demo.o so the bridge
//      static lib includes our D-implemented functions.
// The druntime/phobos2 link plus the cxx_d_dlib archive come for free via
// cxx-dlang's `links = "cxx_d_dlib"` and its own build script.

use std::env;
use std::path::PathBuf;

const LDC2_SAFETY_FLAGS: &[&str] = &[
    "--edition=2025",
    "--preview=safer",
    "--preview=dip1000",
    "--preview=nosharedaccess",
    "--preview=fixImmutableConv",
    "--preview=systemVariables",
];

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    std::fs::create_dir_all(&out_dir).unwrap();

    let ldc2 = find_ldc2();

    let cxx_dlang_d = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..") // .. → examples/
        .join("..") // .. → cxx-dlang root
        .join("d");
    assert!(
        cxx_dlang_d.join("cxx_d.d").exists(),
        "expected ../../d/cxx_d.d at {}",
        cxx_dlang_d.display()
    );

    // Compile d/demo.d → demo.o (with -I to cxx-dlang/d so `import cxx_d;` works).
    let demo_src = PathBuf::from("d/demo.d");
    let demo_obj = out_dir.join("demo.o");
    let status = std::process::Command::new(&ldc2)
        .args(LDC2_SAFETY_FLAGS)
        .arg("--extern-std=c++17")
        .arg("-relocation-model=pic")
        .arg("-c")
        .arg(format!("-of={}", demo_obj.display()))
        .arg(format!("-I={}", cxx_dlang_d.display()))
        .arg("-Id")
        .arg(&demo_src)
        .status()
        .expect("ldc2 must be invokable (set $DC or $LDC2_PATH)");
    assert!(status.success(), "ldc2 failed compiling d/demo.d");

    cxx_build::bridge("src/main.rs")
        .include("include")
        .object(&demo_obj)
        .flag_if_supported("-std=c++17")
        .compile("demo_bridge");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=include/demo.h");
    println!("cargo:rerun-if-changed=d/demo.d");
    println!("cargo:rerun-if-env-changed=DC");
    println!("cargo:rerun-if-env-changed=LDC2_PATH");
}

fn find_ldc2() -> PathBuf {
    for var in ["DC", "LDC2_PATH"] {
        if let Ok(path) = env::var(var) {
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
    if let Some(p) = which_on_path("ldc2") {
        return p;
    }
    let home = env::var("HOME").unwrap_or_default();
    let dlang_dir = PathBuf::from(&home).join(".dlang");
    if dlang_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&dlang_dir)
    {
        let mut candidates: Vec<PathBuf> = entries
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| n.starts_with("ldc2-"))
                    .unwrap_or(false)
            })
            .map(|e| e.path().join("bin").join("ldc2"))
            .filter(|p| p.is_file())
            .collect();
        candidates.sort();
        if let Some(p) = candidates.pop() {
            return p;
        }
    }
    panic!("ldc2 not found; set $DC or $LDC2_PATH");
}

fn which_on_path(name: &str) -> Option<PathBuf> {
    let path = env::var("PATH").ok()?;
    let exts: &[&str] = if cfg!(target_os = "windows") {
        &["", ".exe"]
    } else {
        &[""]
    };
    for dir in env::split_paths(&path) {
        for ext in exts {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
