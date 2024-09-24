use proofman_cli::commands::pil_helpers::PilHelpersCmd;

fn main() {
    let root_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("../../../../");
    let root_path = std::fs::canonicalize(root_path).expect("Failed to canonicalize root path");

    // Re-run this build script if the pil file changes
    println!(
        "cargo:rerun-if-changed={}",
        root_path
            .join("test/std/permutation/permutation.pil")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root_path
            .join("test/std/permutation/src/pil_helpers")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root_path.join("test/std/permutation/build").display()
    );

    let build_dir = root_path.join("test/std/permutation/build/");
    if !build_dir.exists() {
        std::fs::create_dir_all(&build_dir).expect("Failed to create build directory");
    }

    // Compile the pil file
    let pil_compilation = std::process::Command::new("node")
        .arg(root_path.join("../pil2-compiler/src/pil.js"))
        .arg("-I")
        .arg(root_path.join("lib/std/pil"))
        .arg(root_path.join("test/std/permutation/permutation.pil"))
        .arg("-o")
        .arg(root_path.join("test/std/permutation/build/permutation.pilout"))
        .status()
        .expect("Failed to execute pil compilation command");

    if !pil_compilation.success() {
        eprintln!("Error: Pil file compilation failed.");
        std::process::exit(1);
    }

    // Generate pil_helpers
    let pil_helpers = PilHelpersCmd {
        pilout: root_path.join("test/std/permutation/build/permutation.pilout"),
        path: root_path.join("test/std/permutation/rs/src"),
        overide: true,
    };

    if let Err(e) = pil_helpers.run() {
        eprintln!("Error: Failed to generate pil_helpers: {:?}", e);
        std::process::exit(1);
    }

    // Generate proving key
    let proving_key_generation = std::process::Command::new("node")
        .arg(root_path.join("../pil2-proofman-js/src/main_setup.js"))
        .arg("-a")
        .arg(root_path.join("test/std/permutation/build/permutation.pilout"))
        .arg("-b")
        .arg(root_path.join("test/std/permutation/build/"))
        .status()
        .expect("Failed to execute proving key generation command");

    if !proving_key_generation.success() {
        eprintln!("Error: Proving key generation failed.");
        std::process::exit(1);
    }

    println!("Build completed successfully.");
}
