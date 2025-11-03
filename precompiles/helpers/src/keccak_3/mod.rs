use std::path::Path;

use circuit::{ExpressionManager, ExpressionManagerConfig};

mod chi;
mod iota;
mod pi;
mod rho;
mod round_constants;
mod theta;
mod utils;

use chi::keccak_f_chi;
use iota::keccak_f_iota;
use pi::keccak_f_pi;
use rho::keccak_f_rho;
use round_constants::KECCAK_F_RC;
use theta::keccak_f_theta;
use utils::bit_position;

const KECCAKF_EXPR_RESET_THRESHOLD: u32 = 1 << 20;
const KECCAKF_STATE_IN_BITS: usize = 1600;
const KECCAKF_STATE_OUT_BITS: usize = 1600;

pub fn keccak_f_expr<P: AsRef<Path>>(output_dir: P) -> std::io::Result<()> {
    let output_dir = output_dir.as_ref();

    // Initialize the expression manager
    let config = ExpressionManagerConfig {
        reset_threshold: KECCAKF_EXPR_RESET_THRESHOLD,
        sin_count: KECCAKF_STATE_IN_BITS,
        sout_count: KECCAKF_STATE_OUT_BITS,
        im_prefix: "im".to_string(),
        reset_prefix: "r".to_string(),
    };
    let mut expr_manager = ExpressionManager::new(config);

    // Apply all 24 rounds of Keccak permutations
    for r in 0..24 {
        expr_manager.mark_begin_round(r);

        // θ step
        expr_manager.set_context("θ");
        keccak_f_theta(&mut expr_manager);
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // ρ step
        expr_manager.set_context("ρ");
        keccak_f_rho(&mut expr_manager);
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // π step
        expr_manager.set_context("π");
        keccak_f_pi(&mut expr_manager);
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // χ step
        expr_manager.set_context("χ");
        keccak_f_chi(&mut expr_manager);
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // ι step
        expr_manager.set_context("ι");
        keccak_f_iota(&mut expr_manager, r);

        // Reset expressions after each round
        expr_manager.set_context("End of round");
        for i in 0..1600 {
            expr_manager.sout_expr_ids[i] =
                expr_manager.create_manual_reset_expression(expr_manager.sout_expr_ids[i]);
        }

        // Mark end of round and generate file
        expr_manager.mark_end_of_round(r, output_dir)?;

        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        expr_manager.print_round_events(r, Some(5));
    }

    expr_manager.print_summary();

    Ok(())
}
