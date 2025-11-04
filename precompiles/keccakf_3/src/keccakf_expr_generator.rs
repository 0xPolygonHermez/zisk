use path_clean::PathClean;
use std::path::Path;

use circuit::ExpressionManagerConfig;
use precompiles_helpers::keccak_f_expr;

const KECCAKF_EXPR_RESET_THRESHOLD: u32 = 1 << 20;
const KECCAKF_STATE_IN_BITS: usize = 1600;
const KECCAKF_STATE_OUT_BITS: usize = 1600;

// cargo run --bin keccakf_expr_generator
pub fn main() {
    let current_file_path = Path::new(file!());
    let current_dir = current_file_path.parent().expect("Error getting parent directory");
    let pil_code_path = current_dir.join("../pil/expressions/").clean();

    // Initialize the expression manager
    let config = ExpressionManagerConfig {
        reset_threshold: KECCAKF_EXPR_RESET_THRESHOLD,
        sin_count: KECCAKF_STATE_IN_BITS,
        sout_count: KECCAKF_STATE_OUT_BITS,
        in_prefix: Some("state_by_round".to_string()),
        out_prefix: Some("out_exprs".to_string()),
        output_dir: Some(pil_code_path.clone()),
    };
    keccak_f_expr(config, true).expect("Failed to generate Keccak-f expressions");

    println!("Keccak-f expressions generated successfully at: {}", pil_code_path.display());
}
