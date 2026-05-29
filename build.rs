// Build script — compiles `d/*.d` under the LDC2 safety previews, bundles
// the objects into `libcxx_d_dlib.a`, and links druntime + phobos2 statically.

use std::env;
use std::path::{Path, PathBuf};

// LDC2 `--preview=` safety flags applied to every D compilation unit.
//
// Mirror Rust safety invariants we maintain at the FFI boundary:
//   safer            stricter @safe checks (base layer)
//   dip1000          scope pointer escape prevention (borrow-checker analogue)
//   nosharedaccess   forbid direct access to `shared` data (Sync analogue)
//   fixImmutableConv close void[]→immutable reinterpretation hole
//   systemVariables  @safe code cannot touch @system-marked variables
const LDC2_SAFETY_FLAGS: &[&str] = &[
    "--edition=2025",
    "--preview=safer",
    "--preview=dip1000",
    "--preview=nosharedaccess",
    "--preview=fixImmutableConv",
    "--preview=systemVariables",
];

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let d_objs_dir = out_dir.join("d_objs");
    std::fs::create_dir_all(&d_objs_dir).expect("create d_objs dir");

    let ldc2_bin = find_ldc2();
    let ldc2_root = ldc2_bin
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("LDC2 root must be inferrable from ldc2 binary path");

    let d_objects: Vec<PathBuf> = collect_d_sources("d")
        .into_iter()
        .map(|src| compile_d(&ldc2_bin, &d_objs_dir, &src))
        .collect();

    let mut ar = cc::Build::new();
    for obj in &d_objects {
        ar.object(obj);
    }
    ar.compile("cxx_d_dlib");

    let lib_dir = find_ldc2_lib_dir(&ldc2_root);
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=druntime-ldc");
    println!("cargo:rustc-link-lib=static=phobos2-ldc");

    // macOS folds pthread/m/dl into libSystem; Windows druntime pulls in the
    // Win32 APIs it needs directly. Only Linux needs them spelled out.
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "linux" {
        println!("cargo:rustc-link-lib=dylib=pthread");
        println!("cargo:rustc-link-lib=dylib=m");
        println!("cargo:rustc-link-lib=dylib=dl");
    }

    println!("cargo:rerun-if-env-changed=DC");
    println!("cargo:rerun-if-env-changed=LDC2_PATH");
    for src in collect_d_sources("d") {
        println!("cargo:rerun-if-changed={}", src.display());
    }
}

fn compile_d(ldc2: &Path, out_dir: &Path, src: &Path) -> PathBuf {
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("D source file stem");
    let obj = out_dir.join(format!("{stem}.o"));
    let status = std::process::Command::new(ldc2)
        .args(LDC2_SAFETY_FLAGS)
        .arg("--extern-std=c++17")
        .arg("-relocation-model=pic")
        .arg("-c")
        .arg(format!("-of={}", obj.display()))
        .arg("-Id")
        .arg(src)
        .status()
        .unwrap_or_else(|e| panic!("Failed to invoke ldc2: {e}"));
    assert!(
        status.success(),
        "LDC2 compilation failed for {}",
        src.display()
    );
    obj
}

fn collect_d_sources(dir: &str) -> Vec<PathBuf> {
    let path = Path::new(dir);
    if !path.is_dir() {
        return Vec::new();
    }
    let mut sources: Vec<PathBuf> = std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("d"))
        .collect();
    sources.sort();
    sources
}

// ─── LDC2 toolchain discovery ───────────────────────────────────────────────
// Priority: $DC → $LDC2_PATH → walk $PATH → ~/.dlang/ldc2-*/bin/ldc2.

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
    panic!(
        "ldc2 not found; set $DC or $LDC2_PATH, or install LDC2 >= 1.40 via https://dlang.org/download.html"
    );
}

fn find_ldc2_lib_dir(ldc2_root: &Path) -> PathBuf {
    let probe =
        |p: &PathBuf| p.join("libdruntime-ldc.a").exists() || p.join("druntime-ldc.lib").exists();
    let base = ldc2_root.join("lib");
    if probe(&base) {
        return base;
    }
    let arch = if cfg!(target_arch = "aarch64") {
        "lib-arm64"
    } else {
        "lib-x86_64"
    };
    let arch_dir = ldc2_root.join(arch);
    if probe(&arch_dir) {
        return arch_dir;
    }
    let lib64 = ldc2_root.join("lib64");
    if probe(&lib64) {
        return lib64;
    }
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
    scan_dirs.into_iter().max().unwrap_or(base)
}

/// Mini `which` — walks `$PATH` for an executable, adding `.exe` on Windows.
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
