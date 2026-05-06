use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-cfg=feature=\"no_lib_link\"");
        return;
    }

    let out_dir = env::var("OUT_DIR").unwrap(); // Cargo sets this for each build
    let build_dir = Path::new(&out_dir).join("memcpp");

    // Ensure build path exists
    std::fs::create_dir_all(&build_dir).unwrap();

    // Build extra C++ defines based on enabled Cargo features
    let mut extra_defines = String::new();
    if cfg!(feature = "save_mem_align_counters") {
        extra_defines.push_str(" -DSAVE_MEM_ALIGN_COUNTERS");
    }
    if cfg!(feature = "save_mem_bus_data_asm") {
        extra_defines.push_str(" -DSAVE_MEM_BUS_DATA_ASM");
    }

    // Call make with an output path override
    let status = Command::new("make")
        .arg("all")
        .env("OUT_DIR", &build_dir)
        .env("EXTRA_CXXFLAGS", &extra_defines)
        .current_dir("cpp")
        .status()
        .expect("Failed to run make");

    assert!(status.success(), "Makefile build failed");

    // Tell Cargo where to find the library
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=memcpp");
    println!("cargo:rustc-link-lib=dylib=stdc++");

    watch_dir_recursive("cpp");

    // Optional GPU build, gated by the `gpu` cargo feature.
    if cfg!(feature = "gpu") {
        let gpu_build_dir = Path::new(&out_dir).join("memcpp_cu");
        std::fs::create_dir_all(&gpu_build_dir).unwrap();

        let path = format!("/usr/local/cuda/bin:{}", env::var("PATH").unwrap_or_default());
        let status = Command::new("make")
            .arg("all")
            .env("OUT_DIR", &gpu_build_dir)
            .env("PATH", &path)
            .current_dir("cu")
            .status()
            .expect("Failed to run make");

        assert!(status.success(), "GPU Makefile build failed");

        println!("cargo:rustc-link-search=native={}", gpu_build_dir.display());
        println!("cargo:rustc-link-lib=static=memcpp_cu");
        println!("cargo:rustc-link-search=native=/usr/local/cuda/lib64");
        println!("cargo:rustc-link-lib=dylib=cudart");
    }
}

fn watch_dir_recursive<P: AsRef<Path>>(dir: P) {
    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            watch_dir_recursive(&path);
        } else if let Some(ext) = path.extension() {
            if ext == "cpp" || ext == "hpp" {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}
