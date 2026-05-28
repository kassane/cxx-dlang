#![allow(unexpected_cfgs)]

use std::env;
use std::path::{Path, PathBuf};

// Safety flags applied to every LDC2 compilation unit.
// Flags map to Rust safety invariants we maintain at the FFI boundary:
//   safer          → stricter @safe checks (base layer)
//   dip1000        → scope pointer escape prevention (borrow checker analogue)
//   nosharedaccess → forbid direct access to `shared` data (Sync analogue)
//   fixImmutableConv → close void[]→immutable reinterpretation hole
//   systemVariables → @safe code cannot touch @system-marked variables
const LDC2_SAFETY_FLAGS: &[&str] = &[
    "--edition=2025",
    "--preview=safer",
    "--preview=dip1000",
    "--preview=nosharedaccess",
    "--preview=fixImmutableConv",
    "--preview=systemVariables",
];

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let d_objs_dir = out_dir.join("d_objs");
    let d_headers_dir = out_dir.join("d_headers");

    std::fs::create_dir_all(&d_objs_dir).unwrap();
    std::fs::create_dir_all(&d_headers_dir).unwrap();

    // Locate LDC2 binary
    let ldc2 = find_ldc2();

    // Infer LDC2 root (two levels up from bin/ldc2)
    let ldc2_root = ldc2
        .parent() // bin/
        .and_then(|p| p.parent()) // ldc2-x.y.z-linux/
        .map(Path::to_path_buf)
        .expect("Cannot determine LDC2 root from binary path");

    // Compile all .d sources to .o
    let d_sources: Vec<PathBuf> = collect_d_sources("d");
    let mut d_objects: Vec<PathBuf> = Vec::new();

    for src in &d_sources {
        let stem = src.file_stem().unwrap().to_str().unwrap();
        let obj = d_objs_dir.join(format!("{stem}.o"));
        let hdr = d_headers_dir.join(format!("{stem}.hpp"));

        let status = std::process::Command::new(&ldc2)
            .args(LDC2_SAFETY_FLAGS)
            .arg("--extern-std=c++17")
            .arg("-relocation-model=pic")
            .arg("-HC=silent")
            .arg(format!("-HCf={}", hdr.display()))
            .arg("-c")
            .arg(format!("-of={}", obj.display()))
            .arg("-Id")
            .arg(src)
            .status()
            .unwrap_or_else(|e| panic!("Failed to run ldc2: {e}"));

        assert!(
            status.success(),
            "LDC2 compilation failed for {}",
            src.display()
        );
        d_objects.push(obj);
    }

    // Also compile test D modules unconditionally
    let test_d_sources: Vec<PathBuf> = collect_d_sources("tests/d");
    for src in &test_d_sources {
        let stem = src.file_stem().unwrap().to_str().unwrap();
        let obj = d_objs_dir.join(format!("test_{stem}.o"));

        let status = std::process::Command::new(&ldc2)
            .args(LDC2_SAFETY_FLAGS)
            .arg("--extern-std=c++17")
            .arg("-relocation-model=pic")
            .arg("-c")
            .arg(format!("-of={}", obj.display()))
            .arg("-Id")
            .arg("-Itests/d")
            .arg(src)
            .status()
            .unwrap_or_else(|e| panic!("Failed to run ldc2: {e}"));

        if status.success() {
            d_objects.push(obj);
        }
        // test modules are optional — don't panic if missing
    }

    // Build cxx bridge + link D objects
    let mut build = cxx_build::bridge("src/ffi.rs");
    build.include("include").flag_if_supported("-std=c++17");

    for obj in &d_objects {
        build.object(obj);
    }

    build.compile("cxx_d_dlib");

    // Link druntime and phobos statically.
    //   Linux  : <root>/lib/libdruntime-ldc.a + libphobos2-ldc.a
    //   macOS  : <root>/lib-arm64/ or lib-x86_64/ (universal), same .a names
    //   Windows: <root>/lib64/druntime-ldc.lib + phobos2-ldc.lib  (MSVC, no lib prefix)
    let lib_dir = find_ldc2_lib_dir(&ldc2_root);
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=druntime-ldc");
    println!("cargo:rustc-link-lib=static=phobos2-ldc");

    // Platform-specific system libs — use CARGO_CFG_TARGET_OS (correct for cross-builds).
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "linux" => {
            println!("cargo:rustc-link-lib=dylib=pthread");
            println!("cargo:rustc-link-lib=dylib=m");
            println!("cargo:rustc-link-lib=dylib=dl");
        }
        "macos" => {
            // pthread, m, dl are all part of libSystem on macOS
        }
        "windows" => {
            // LDC2 Windows is MSVC-only; druntime pulls in Win32 APIs directly.
            // link-cplusplus handles the C++ runtime (vcruntime / ucrt).
        }
        _ => {}
    }

    // Rerun-if-changed
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=include/cxx_d.h");
    println!("cargo:rerun-if-env-changed=LDC2_PATH");
    for src in collect_d_sources("d")
        .iter()
        .chain(collect_d_sources("tests/d").iter())
    {
        println!("cargo:rerun-if-changed={}", src.display());
    }

    // cfg hygiene
    println!("cargo:rustc-check-cfg=cfg(cxx_d_test_modules)");
}

fn find_ldc2_lib_dir(ldc2_root: &Path) -> PathBuf {
    // Probe order:
    //   1. <root>/lib/libdruntime-ldc.a              — Linux standard
    //   2. <root>/lib-arm64/ or lib-x86_64/          — macOS universal
    //   3. <root>/lib64/                             — Windows multilib
    //   4. <root>/lib/<subdir>/                      — macOS versioned subdir
    //   5. any <root>/lib*/                          — generic scan

    // Accept both .a (MinGW / Linux / macOS) and .lib (MSVC Windows)
    let probe = |p: &PathBuf| {
        p.join("libdruntime-ldc.a").exists() || p.join("druntime-ldc.lib").exists()
    };

    let base = ldc2_root.join("lib");
    if probe(&base) {
        return base;
    }

    // macOS universal: lib-arm64 / lib-x86_64
    let arch = if cfg!(target_arch = "aarch64") { "lib-arm64" } else { "lib-x86_64" };
    let arch_dir = ldc2_root.join(arch);
    if probe(&arch_dir) {
        return arch_dir;
    }

    // Windows multilib: lib64
    let lib64 = ldc2_root.join("lib64");
    if probe(&lib64) {
        return lib64;
    }

    // Scan all lib* dirs at root level and under lib/ (covers every known layout)
    let scan_dirs: Vec<PathBuf> = [ldc2_root, &base]
        .iter()
        .flat_map(|dir| std::fs::read_dir(dir).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
                && probe(p)
        })
        .collect();

    if let Some(p) = scan_dirs.into_iter().max() {
        return p;
    }

    base
}

fn find_ldc2() -> PathBuf {
    // Priority 1: LDC2_PATH env
    if let Ok(path) = env::var("LDC2_PATH") {
        let p = PathBuf::from(&path);
        if p.is_file() {
            return p;
        }
        let p2 = p.join("ldc2");
        if p2.is_file() {
            return p2;
        }
    }

    // Priority 2: which
    if let Ok(p) = which::which("ldc2") {
        return p;
    }

    // Priority 3: well-known install location (dlang installer)
    let home = env::var("HOME").unwrap_or_default();
    let dlang_dir = PathBuf::from(&home).join(".dlang");
    if dlang_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&dlang_dir) {
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
    }

    panic!("ldc2 not found; set LDC2_PATH env var or install LDC2 >= 1.40 via https://dlang.org/download.html");
}

fn collect_d_sources(dir: &str) -> Vec<PathBuf> {
    let path = Path::new(dir);
    if !path.is_dir() {
        return Vec::new();
    }
    let mut sources = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("d") {
                sources.push(p);
            }
        }
    }
    sources.sort();
    sources
}
