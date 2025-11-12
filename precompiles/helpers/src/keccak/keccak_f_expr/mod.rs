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

pub fn keccak_f_expr(
    config: ExpressionManagerConfig,
    display_num: usize,
    generate_files: bool,
) -> std::io::Result<()> {
    // Initialize the expression manager
    let mut expr_manager = ExpressionManager::new(config);

    // Apply all 24 rounds of Keccak permutations
    for r in 0..24 {
        // Mark beginning of round
        expr_manager.mark_begin_round(r);

        // θ step
        expr_manager.set_context(Some("θ"));
        keccak_f_theta(&mut expr_manager);
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // ρ step
        expr_manager.set_context(Some("ρ"));
        keccak_f_rho(&mut expr_manager);
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // π step
        expr_manager.set_context(Some("π"));
        keccak_f_pi(&mut expr_manager);
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // Reset expressions
        for i in 0..1600 {
            expr_manager.sin_expr_ids[i] =
                expr_manager.create_manual_reset_expression(expr_manager.sin_expr_ids[i]);
        }

        // χ step
        expr_manager.set_context(Some("χ"));
        keccak_f_chi(&mut expr_manager);
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // ι step
        expr_manager.set_context(Some("ι"));
        keccak_f_iota(&mut expr_manager, r);

        // End of round
        expr_manager.set_context(Some("End of round"));

        // Mark end of round
        expr_manager.mark_end_of_round(r);

        // Generate round file if required
        if generate_files {
            expr_manager.generate_pil_round_file(r)?;
            expr_manager.generate_rust_round_file(r)?;
        }

        // Prepare for next round
        if r < 23 {
            expr_manager.copy_sout_expr_ids_to_sin_expr_ids();
        }

        // Print round events
        expr_manager.print_round_events(r, Some(display_num));
    }

    // Print final summary
    expr_manager.print_summary();

    // Generate summary file if required
    if generate_files {
        expr_manager.generate_summary_file()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccak_f_expr() {
        let config = ExpressionManagerConfig {
            value_reset_threshold: 1 << 22,
            degree_reset_threshold: 3,
            sin_count: 1600,
            sout_count: 1600,
            in_prefix: None,
            out_prefix: None,
            pil_output_dir: None,
            rust_output_dir: None,
        };
        let result = keccak_f_expr(config, 5, false);
        assert!(result.is_ok());
    }
}
