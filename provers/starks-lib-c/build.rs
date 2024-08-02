use std::env;

fn main() {
    //if std::env::var("CARGO_FEATURE_NO_LIB_LINK").is_err() {
    let library_folder = "../../../zkevm-prover/lib";
    let library_sm_folder = "../../../zkevm-prover-rust/target/release";
    let library_short_name = "starks";
    let library_name = format!("lib{}.a", library_short_name);
    let library_path = format!("{}/{}", library_folder, library_name);

    println!("cargo:rerun-if-changed={}", env::current_dir().unwrap().join(library_path).display());

    // Tipically the libraries are in: sudo find /usr /lib /lib64 /usr/lib /usr/lib64 -name "libstdc++.a"
    println!("cargo:rustc-link-search=native={}", env::current_dir().unwrap().join(library_folder).display());
    println!("cargo:rustc-link-search=native={}", env::current_dir().unwrap().join(library_sm_folder).display());

    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu/");
    println!("cargo:rustc-link-search=native=/usr/lib/gcc/x86_64-linux-gnu/11/");

    println!("cargo:rustc-link-arg=-fopenmp");
    println!("cargo:rustc-link-arg=-MMD");
    println!("cargo:rustc-link-arg=-MP");
    println!("cargo:rustc-link-arg=-std=c++17");
    println!("cargo:rustc-link-arg=-Wall");
    println!("cargo:rustc-link-arg=-flarge-source-files");
    println!("cargo:rustc-link-arg=-Wno-unused-label");
    println!("cargo:rustc-link-arg=-rdynamic");
    println!("cargo:rustc-link-arg=-mavx2");

    println!("cargo:rustc-link-lib=static={}", library_short_name);

    println!("cargo:rustc-link-lib=protobuf");
    println!("cargo:rustc-link-lib=sodium");
    println!("cargo:rustc-link-lib=grpc++");
    println!("cargo:rustc-link-lib=grpc");
    println!("cargo:rustc-link-lib=gpr");
    println!("cargo:rustc-link-lib=grpc++_reflection");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=pqxx");
    println!("cargo:rustc-link-lib=pq");
    println!("cargo:rustc-link-lib=gmp");
    println!("cargo:rustc-link-lib=stdc++");
    println!("cargo:rustc-link-lib=gmpxx");
    println!("cargo:rustc-link-lib=secp256k1");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=uuid");

    //-lgrpc++ -lgrpc -lgpr
    println!("cargo:rustc-link-lib=iomp5");
    //}
}
