//! Build-only shim: links the version-matching Proofman native archive from the registry
//! and records what was linked. Used by host tools that prove with a custom key.

use std::{env, fs, path::{Path, PathBuf}};
use sha2::{Digest, Sha256};

fn required(name: &str) -> PathBuf {
    println!("cargo:rerun-if-env-changed={name}");
    PathBuf::from(env::var(name).unwrap_or_else(|_| panic!("set {name} to an existing version-matching dependency")))
}

fn hash(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

fn main() {
    assert_eq!(env::var("CARGO_CFG_TARGET_OS").unwrap(), "linux", "diagnostic link shim supports Linux only");
    let source = required("PROOFMAN_FFI_SOURCE_DIR");
    let archive = required("PROOFMAN_NATIVE_ARCHIVE");
    let cuda = required("PROOFMAN_CUDA_LIB_DIR");
    let blst = required("PROOFMAN_BLST_ARCHIVE");
    let extra = required("PROOFMAN_EXTRA_LIB_DIR");
    println!("cargo:rerun-if-env-changed=PROOFMAN_OPENMP_LIB");
    let openmp = env::var("PROOFMAN_OPENMP_LIB").unwrap_or_else(|_| "iomp5".into());
    assert!(matches!(openmp.as_str(), "iomp5" | "gomp"), "PROOFMAN_OPENMP_LIB must be iomp5 or gomp");
    assert!(fs::read_to_string(source.join("Cargo.toml")).unwrap().contains("version = \"1.2.0-alpha\""));
    assert_eq!(archive.file_name().unwrap(), "libstarksgpu.a");
    assert_eq!(blst.file_name().unwrap(), "libblst.a");
    assert!(cuda.join("libcudart_static.a").is_file());
    let output = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut sources = serde_json::Map::new();
    for relative in ["src/lib.rs", "src/ffi_goldilocks.rs", "src/ffi_starks.rs", "bindings_starks.rs"] {
        let from = source.join(relative);
        let to = output.join("upstream").join(relative);
        println!("cargo:rerun-if-changed={}", from.display());
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(&from, &to).unwrap();
        assert_eq!(hash(&from), hash(&to));
        sources.insert(relative.into(), serde_json::Value::String(hash(&from)));
    }
    for path in [&archive, &blst] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let provenance = serde_json::json!({
        "version": "1.2.0-alpha", "sourceDirectory": source, "sourceSha256": sources,
        "archive": archive, "archiveSha256": hash(&archive),
        "blstArchive": blst, "blstSha256": hash(&blst),
        "openmpLibrary": openmp,
        "linkedCapability": "GPU-capable upstream archive; execution mode selected at runtime",
        "ffiSourcesCopiedByteForByte": true,
    });
    fs::write(output.join("link-provenance.json"), serde_json::to_vec_pretty(&provenance).unwrap()).unwrap();
    println!("cargo:rustc-env=STARKS_BUILD_MODE=GPU");
    for directory in [archive.parent().unwrap(), blst.parent().unwrap(), cuda.as_path(), extra.as_path()] {
        println!("cargo:rustc-link-search=native={}", directory.display());
    }
    println!("cargo:rustc-link-lib=static=starksgpu");
    println!("cargo:rustc-link-lib=static=cudart_static");
    println!("cargo:rustc-link-lib=static=blst");
    for library in ["dl", "rt", "sodium", "pthread", "gmp", "stdc++", "gmpxx", "crypto", "mpi"] {
        println!("cargo:rustc-link-lib={library}");
    }
    println!("cargo:rustc-link-lib={openmp}");
}
