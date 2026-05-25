//! Replicate qedsvm-rs's link-arg setup. cargo:rustc-link-arg directives
//! from a dependency's build.rs only apply to bins/tests in the SAME
//! package, so consuming qedsvm as a library from a different package
//! requires re-emitting them here. This is a known issue (filed
//! upstream); when qedsvm ships a usable helper this file goes away.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let qedsvm_root = std::env::var("QEDSVM_ROOT")
        .unwrap_or_else(|_| "../../qedsvm".to_string());
    let qedsvm_root = std::fs::canonicalize(&qedsvm_root).unwrap_or_else(|e| {
        panic!(
            "QEDSVM_ROOT={qedsvm_root} not found: {e}. \
             Clone qedsvm next to janus or set QEDSVM_ROOT."
        )
    });
    let lake_lean_dir = qedsvm_root.join(".lake/build/lib/lean");
    let lake_lib_dir = qedsvm_root.join(".lake/build/lib");
    if !lake_lean_dir.exists() {
        panic!(
            "Lake artifacts missing at {}. Run `lake build` in {}.",
            lake_lean_dir.display(),
            qedsvm_root.display(),
        );
    }

    // Lean toolchain prefix
    let prefix = Command::new("lean")
        .arg("--print-prefix")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .expect("`lean --print-prefix` failed — Lean must be on PATH");
    let lean_lib_dir = PathBuf::from(&prefix).join("lib/lean");
    let lean_lib_root = PathBuf::from(&prefix).join("lib");

    // Search paths + rpaths
    for d in [&lean_lib_dir, &lean_lib_root, &lake_lean_dir, &lake_lib_dir] {
        println!("cargo:rustc-link-search=native={}", d.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", d.display());
    }

    // Lean runtime + Lake's shared lib
    println!("cargo:rustc-link-lib=dylib=leanshared");
    println!("cargo:rustc-link-lib=dylib=Lake_shared");

    // Every qedsvm_*.dylib from Lake's output (each Lean module is its
    // own dylib; ~80 of them).
    let mut count = 0;
    for entry in std::fs::read_dir(&lake_lean_dir).expect("read lake lib dir") {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else { continue };
        let is_dyn = name.ends_with(".dylib") || name.ends_with(".so");
        if !is_dyn || !name.starts_with("qedsvm_") { continue }
        println!("cargo:rustc-link-arg={}", path.display());
        count += 1;
    }
    if count == 0 {
        panic!("Found 0 qedsvm_*.dylib in {}", lake_lean_dir.display());
    }

    // Force-pull FFI symbols from libleanbridge.a. Parse the bridge
    // source for `pub extern "C" fn lean_*` and emit `-Wl,-u,<sym>`
    // for each — same trick qedsvm-rs's build.rs uses.
    let bridge_src = qedsvm_root.join("qedsvm-rs/lean-bridge/src/lib.rs");
    let ffi_syms = parse_lean_exports(&bridge_src);
    let prefix_char = if cfg!(target_os = "macos") { "_" } else { "" };
    for sym in &ffi_syms {
        println!("cargo:rustc-link-arg=-Wl,-u,{prefix_char}{sym}");
    }
    let leanbridge_a = lake_lib_dir.join("libleanbridge.a");
    if !leanbridge_a.exists() {
        panic!("Missing {}. Run `lake build`.", leanbridge_a.display());
    }
    println!("cargo:rustc-link-arg={}", leanbridge_a.display());

    // Positional dylib paths to the Lean shared libs (cargo strips
    // -l<dylib> if it can't see direct refs from user code).
    let leanshared = PathBuf::from(&prefix).join("lib/lean/libleanshared.dylib");
    let lake_shared = PathBuf::from(&prefix).join("lib/lean/libLake_shared.dylib");
    for p in [&leanshared, &lake_shared] {
        if p.exists() {
            println!("cargo:rustc-link-arg={}", p.display());
        }
    }

    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-arg=-rdynamic");
    }
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-export_dynamic");
    }

    println!("cargo:rerun-if-changed=build.rs");
}

fn parse_lean_exports(path: &std::path::Path) -> Vec<String> {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let needle = "pub extern \"C\" fn lean_";
    let mut out = Vec::new();
    for line in src.lines() {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix(needle) else { continue };
        let name_tail: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !name_tail.is_empty() {
            out.push(format!("lean_{name_tail}"));
        }
    }
    out
}
