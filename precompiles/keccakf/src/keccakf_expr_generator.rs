use path_clean::PathClean;
use std::path::Path;

use circuit::ExpressionManagerConfig;
use precompiles_helpers::keccak_f_expr;

const EXPR_VALUE_THRESHOLD: u32 = 1 << 22;
const EXPR_DEGREE_THRESHOLD: usize = 3;
const STATE_IN_BITS: usize = 1600;
const STATE_OUT_BITS: usize = 1600;
const IN_PREFIX: &str = "state_by_round";
const OUT_PREFIX: &str = "out_exprs";

const DISPLAY_NUM: usize = 3;
const GENERATE_FILES: bool = false;

// cargo run --bin keccakf_expr_generator
pub fn main() {
    let current_file_path = Path::new(file!());
    let current_dir = current_file_path.parent().expect("Error getting parent directory");
    let pil_code_path = current_dir.join("../pil/expressions/").clean();
    let rs_code_path = current_dir.join("expressions").clean();

    // Initialize the expression manager
    let config = ExpressionManagerConfig {
        value_reset_threshold: EXPR_VALUE_THRESHOLD,
        degree_reset_threshold: EXPR_DEGREE_THRESHOLD,
        sin_count: STATE_IN_BITS,
        sout_count: STATE_OUT_BITS,
        in_prefix: Some(IN_PREFIX.to_string()),
        out_prefix: Some(OUT_PREFIX.to_string()),
        pil_output_dir: Some(pil_code_path.clone()),
        rust_output_dir: Some(rs_code_path.clone()),
    };
    keccak_f_expr(config, DISPLAY_NUM, GENERATE_FILES)
        .expect("Failed to generate Keccak-f expressions");

    if GENERATE_FILES {
        println!("Keccak-f PIL expressions generated at: {}", pil_code_path.display());
        println!("Keccak-f Rust expressions generated at: {}", rs_code_path.display());
    }
}
