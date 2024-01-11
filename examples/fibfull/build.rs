use std::env;

fn main() {
    let library_folder = "../../../zkevm-prover/api";
    let library_short_name = "zkProverAPI";
    let library_name = format!("lib{}.a", library_short_name);
    let library_path = format!("{}/{}", library_folder, library_name);

    println!("cargo:rerun-if-changed={}", env::current_dir().unwrap().join(library_path).display());

    // Tipically the libraries are in: sudo find /usr /lib /lib64 /usr/lib /usr/lib64 -name "libstdc++.a"
    println!("cargo:rustc-link-search=native={}", env::current_dir().unwrap().join(library_folder).display());
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu/");
    println!("cargo:rustc-link-search=native=/usr/lib/gcc/x86_64-linux-gnu/11/");

    println!("cargo:rustc-link-arg=-fopenmp");

    println!("cargo:rustc-link-lib=static={}", library_short_name);
    println!("cargo:rustc-link-lib=static=stdc++");
    println!("cargo:rustc-link-lib=static=gmp");
    println!("cargo:rustc-link-lib=static=gmpxx");
    println!("cargo:rustc-link-lib=static=crypto");
    println!("cargo:rustc-link-lib=static=uuid");
}
