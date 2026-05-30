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

// Rust target triple (cargo `TARGET`) → LDC2 `--mtriple=` value.
//
// LDC2 uses its own triple convention (e.g. `x86_64-linux-gnu`, not Rust's
// `x86_64-unknown-linux-gnu`). The mapping below tracks the LDC2 cross-runtime
// matrix; entries without a runtime build need `LDC2_LIB_DIR` to point at a
// user-built druntime/phobos.
const RUST_TO_LDC2_TRIPLE: &[(&str, &str)] = &[
    ("x86_64-unknown-linux-gnu", "x86_64-linux-gnu"),
    ("x86_64-unknown-linux-musl", "x86_64-linux-musl"),
    ("x86_64-apple-darwin", "x86_64-apple-macos10.12"),
    ("x86_64-pc-windows-msvc", "x86_64-windows-msvc"),
    ("i686-unknown-linux-gnu", "i686-linux-gnu"),
    ("i686-unknown-linux-musl", "i686-linux-musl"),
    ("i686-pc-windows-msvc", "i686-windows-msvc"),
    ("arm-unknown-linux-gnueabihf", "armv6-linux-gnueabihf"),
    (
        "armv7-linux-androideabi",
        "armv7a-unknown-linux-androideabi",
    ),
    ("aarch64-linux-android", "aarch64-linux-android"),
    ("aarch64-unknown-linux-gnu", "aarch64-linux-gnu"),
    ("aarch64-apple-darwin", "arm64-apple-macos11.0"),
    ("aarch64-apple-ios", "arm64-apple-ios12.0"),
    (
        "wasm32-unknown-unknown",
        "wasm32-unknown-unknown-webassembly",
    ),
];

fn ldc2_triple_for(target: &str) -> Option<&'static str> {
    RUST_TO_LDC2_TRIPLE
        .iter()
        .find_map(|(rust, ldc)| (*rust == target).then_some(*ldc))
}

// Parse rustc's `-C target-cpu=<cpu>` out of `CARGO_ENCODED_RUSTFLAGS`. The
// var is `\x1f`-separated; both `-Ctarget-cpu=foo` (joined) and
// `-C` `target-cpu=foo` (split) forms are valid. Returns None if rustc isn't
// pinning a CPU — leaving LDC2 to pick its own default from `--mtriple`.
fn target_cpu_from_rustflags() -> Option<String> {
    let raw = env::var("CARGO_ENCODED_RUSTFLAGS").ok()?;
    let mut parts = raw.split('\x1f');
    while let Some(flag) = parts.next() {
        if let Some(cpu) = flag.strip_prefix("-Ctarget-cpu=") {
            return Some(cpu.to_string());
        }
        if flag == "-C"
            && let Some(next) = parts.next()
            && let Some(cpu) = next.strip_prefix("target-cpu=")
        {
            return Some(cpu.to_string());
        }
    }
    None
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    // FIXME: docs.rs sandbox has no `ldc2` installed and rustdoc does not link the
    // crate anyway, so skip the native D pipeline entirely. Emit an empty
    // archive so the `links = "cxx_d_dlib"` contract is still honoured.
    if env::var("DOCS_RS").is_ok() {
        let stub = out_dir.join("cxx_d_dlib_docsrs_stub.c");
        std::fs::write(&stub, "void __cxx_d_dlib_docsrs_stub(void) {}\n").unwrap();
        cc::Build::new().file(&stub).compile("cxx_d_dlib");
        return;
    }

    let d_objs_dir = out_dir.join("d_objs");
    std::fs::create_dir_all(&d_objs_dir).expect("create d_objs dir");

    let ldc2_bin = find_ldc2();
    let ldc2_root = ldc2_bin
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("LDC2 root must be inferrable from ldc2 binary path");

    let target = env::var("TARGET").unwrap_or_default();
    let host = env::var("HOST").unwrap_or_default();
    let mtriple = (target != host).then(|| {
        ldc2_triple_for(&target).unwrap_or_else(|| {
            panic!(
                "cxx-dlang: no LDC2 triple known for Rust target `{target}`; set $LDC2_MTRIPLE \
                 and $LDC2_LIB_DIR to override, or add the mapping to RUST_TO_LDC2_TRIPLE."
            )
        })
    });
    let mtriple = env::var("LDC2_MTRIPLE").ok().or(mtriple.map(String::from));

    // Mirror rustc's `-C target-feature` onto LDC2's `-mattr=`. Cargo exposes
    // the resolved feature set as `CARGO_CFG_TARGET_FEATURE` (comma-separated,
    // unsigned); LLVM wants each entry prefixed with `+` (or `-` to disable).
    // Override with $LDC2_MATTR if a downstream needs a different baseline.
    let mattr = env::var("LDC2_MATTR").ok().or_else(|| {
        let feats = env::var("CARGO_CFG_TARGET_FEATURE").ok()?;
        let joined = feats
            .split(',')
            .filter(|f| !f.is_empty())
            .map(|f| format!("+{f}"))
            .collect::<Vec<_>>()
            .join(",");
        (!joined.is_empty()).then_some(joined)
    });

    // Mirror rustc's `-C target-cpu=` onto LDC2's `-mcpu=`. Cargo doesn't
    // expose target-cpu directly, but `CARGO_ENCODED_RUSTFLAGS` carries the
    // resolved flag set as `\x1f`-separated entries. `$LDC2_MCPU` overrides.
    let mcpu = env::var("LDC2_MCPU")
        .ok()
        .or_else(target_cpu_from_rustflags);

    let d_objects: Vec<PathBuf> = collect_d_sources("d")
        .into_iter()
        .map(|src| {
            compile_d(
                &ldc2_bin,
                &d_objs_dir,
                &src,
                mtriple.as_deref(),
                mattr.as_deref(),
                mcpu.as_deref(),
            )
        })
        .collect();

    let mut ar = cc::Build::new();
    for obj in &d_objects {
        ar.object(obj);
    }
    ar.compile("cxx_d_dlib");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let lib_dir = env::var("LDC2_LIB_DIR")
        .map(PathBuf::from)
        .ok()
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| find_ldc2_lib_dir(&ldc2_root, &target_arch, &target_os));
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=druntime-ldc");
    println!("cargo:rustc-link-lib=static=phobos2-ldc");

    // macOS folds pthread/m/dl into libSystem; Windows druntime pulls in the
    // Win32 APIs it needs directly; Android's bionic libc owns pthread; wasm
    // has no syscall layer to link against. Only Linux/BSD need them spelled out.
    if target_os == "linux" {
        println!("cargo:rustc-link-lib=dylib=pthread");
        println!("cargo:rustc-link-lib=dylib=m");
        println!("cargo:rustc-link-lib=dylib=dl");
    }

    println!("cargo:rerun-if-env-changed=DC");
    println!("cargo:rerun-if-env-changed=LDC2_PATH");
    println!("cargo:rerun-if-env-changed=LDC2_MTRIPLE");
    println!("cargo:rerun-if-env-changed=LDC2_MCPU");
    println!("cargo:rerun-if-env-changed=LDC2_MATTR");
    println!("cargo:rerun-if-env-changed=LDC2_LIB_DIR");
    for src in collect_d_sources("d") {
        println!("cargo:rerun-if-changed={}", src.display());
    }
}

fn compile_d(
    ldc2: &Path,
    out_dir: &Path,
    src: &Path,
    mtriple: Option<&str>,
    mattr: Option<&str>,
    mcpu: Option<&str>,
) -> PathBuf {
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("D source file stem");
    let obj = out_dir.join(format!("{stem}.o"));
    let mut cmd = std::process::Command::new(ldc2);
    cmd.args(LDC2_SAFETY_FLAGS)
        .arg("--extern-std=c++17")
        .arg("-relocation-model=pic")
        .arg("-c")
        .arg(format!("-of={}", obj.display()))
        .arg("-Id");
    if let Some(triple) = mtriple {
        cmd.arg(format!("--mtriple={triple}"));
    }
    if let Some(cpu) = mcpu {
        cmd.arg(format!("-mcpu={cpu}"));
    }
    if let Some(attrs) = mattr {
        cmd.arg(format!("-mattr={attrs}"));
    }
    cmd.arg(src);
    let status = cmd
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

fn find_ldc2_lib_dir(ldc2_root: &Path, target_arch: &str, target_os: &str) -> PathBuf {
    let probe =
        |p: &PathBuf| p.join("libdruntime-ldc.a").exists() || p.join("druntime-ldc.lib").exists();

    // Cross-runtime layouts shipped by LDC2 or produced by `ldc-build-runtime`:
    //   lib-arm64 / lib-x86_64           ← universal macOS LDC2
    //   lib-{android,wasm,…}-{arch}      ← user-built cross runtimes
    //   lib32 / lib64                    ← Windows multilib release
    // Probe the most-specific arch+os candidate first, then fall back to the
    // generic `lib/` so native builds keep working unchanged.
    let mut candidates: Vec<PathBuf> = Vec::new();
    let arch_alias = match target_arch {
        "aarch64" => "arm64",
        "x86" => "i686",
        other => other,
    };
    if !target_arch.is_empty() && !target_os.is_empty() {
        candidates.push(ldc2_root.join(format!("lib-{target_os}-{arch_alias}")));
        candidates.push(ldc2_root.join(format!("lib-{arch_alias}-{target_os}")));
    }
    if !target_arch.is_empty() {
        candidates.push(ldc2_root.join(format!("lib-{arch_alias}")));
    }
    match target_arch {
        "x86_64" | "aarch64" => candidates.push(ldc2_root.join("lib64")),
        "x86" | "arm" => candidates.push(ldc2_root.join("lib32")),
        _ => {}
    }
    candidates.push(ldc2_root.join("lib"));

    for c in &candidates {
        if probe(c) {
            return c.clone();
        }
    }

    let base = ldc2_root.join("lib");
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
