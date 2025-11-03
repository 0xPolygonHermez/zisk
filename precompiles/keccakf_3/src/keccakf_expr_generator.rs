use path_clean::PathClean;
use std::path::Path;

use precompiles_helpers::keccak_f_expr;

// cargo run --bin keccakf_expr_generator
pub fn main() {
    let current_file_path = Path::new(file!());
    let current_dir = current_file_path.parent().expect("Error getting parent directory");
    let pil_code_path = current_dir.join("../pil/expressions/").clean();

    // Generate Keccak-f expressions to PIL files
    keccak_f_expr(&pil_code_path).expect("Failed to generate Keccak-f expressions");

    println!("Keccak-f expressions generated successfully at: {}", pil_code_path.display());
}
