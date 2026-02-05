use std::fmt::Display;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use super::{Expression, ExpressionOp};

pub struct ExpressionManager {
    /// Configuration
    config: ExpressionManagerConfig,

    /// Important expression IDs
    pub zero_expr_id: usize,
    pub one_expr_id: usize,
    pub sin_expr_ids: Vec<usize>,
    pub sout_expr_ids: Vec<usize>,

    /// Expressions
    exprs: Vec<Expression>,

    /// Counters
    exprs_count: usize,
    proxy_count: usize,
    im_count: usize,
    reset_count: usize,
    round_im_count: usize,
    round_reset_count: usize,

    /// All expression events
    expr_events: Vec<ExpressionEvent>,

    /// Current context for tracking
    pub current_round: usize,
    current_step: Option<String>,
    current_substep: Option<String>,

    /// Global maximum value encountered
    round_max_value: u64,
    max_values_by_round: Vec<u64>,

    /// Global maximum degree encountered
    round_max_degree: usize,
    max_degrees_by_round: Vec<usize>,
}

pub struct ExpressionManagerConfig {
    pub value_reset_threshold: u32,
    pub degree_reset_threshold: usize,
    pub sin_count: usize,
    pub sout_count: usize,
    pub in_prefix: Option<String>,
    pub out_prefix: Option<String>,
    pub pil_output_dir: Option<PathBuf>,
    pub rust_output_dir: Option<PathBuf>,
}

#[derive(Clone)]
struct ExpressionEvent {
    manual: bool,
    op_type: ExpressionOpType,
    reason: Option<ExpressionReason>,
    round: usize,
    step: Option<String>,
    substep: Option<String>,
    old_expr_id: usize,
    original_degree: usize,
    new_degree: usize,
    original_max_value: u64,
    new_max_value: u64,
    op1_max_value: Option<u64>,
    op2_max_value: Option<u64>,
    res_max_value: Option<u64>,
}

#[derive(Clone)]
enum ExpressionReason {
    MaxValue,
    Degree,
    BothThresholds,
}

impl std::fmt::Display for ExpressionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionReason::MaxValue => write!(f, "Max Value Exceeded"),
            ExpressionReason::Degree => write!(f, "Degree Exceeded"),
            ExpressionReason::BothThresholds => write!(f, "Both Thresholds Exceeded"),
        }
    }
}

#[derive(Clone)]
enum ExpressionOpType {
    Im,
    Reset,
}

impl Display for ExpressionOpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionOpType::Im => write!(f, "Im"),
            ExpressionOpType::Reset => write!(f, "Reset"),
        }
    }
}

impl ExpressionManager {
    pub fn new(config: ExpressionManagerConfig) -> Self {
        let value_reset_threshold = config.value_reset_threshold;
        let sin_count = config.sin_count;
        let sout_count = config.sout_count;

        assert!(
            value_reset_threshold > 0 && value_reset_threshold & (value_reset_threshold - 1) == 0
        );
        assert!(sin_count > 0 && sout_count > 0);

        // First expressions are 0 and 1
        let exprs_count = 2 + sin_count + sout_count;
        let mut exprs = Vec::with_capacity(exprs_count);
        exprs.push(Expression::ZERO);
        exprs.push(Expression::ONE);

        // Initialize input and output expressions
        let mut sin_expr_ids = Vec::with_capacity(sin_count);
        let mut sout_expr_ids = Vec::with_capacity(sout_count);
        for i in 0..sin_count {
            exprs.push(Expression::input(i, config.in_prefix.clone(), Some(0)));
            sin_expr_ids.push(2 + i);
        }
        for i in 0..sout_count {
            exprs.push(Expression::ZERO);
            sout_expr_ids.push(2 + sin_count + i);
        }

        Self {
            config,
            zero_expr_id: 0,
            one_expr_id: 1,
            sin_expr_ids,
            sout_expr_ids,
            exprs,
            exprs_count,
            proxy_count: 0,
            im_count: 0,
            reset_count: 0,
            round_im_count: 0,
            round_reset_count: 0,
            expr_events: Vec::new(),
            current_round: 0,
            current_step: None,
            current_substep: None,
            round_max_value: 0,
            max_values_by_round: Vec::new(),
            round_max_degree: 0,
            max_degrees_by_round: Vec::new(),
        }
    }

    pub fn mark_begin_round(&mut self, round: usize) {
        self.current_round = round;
        self.round_im_count = 0;
        self.round_reset_count = 0;
        self.round_max_value = 0;
        self.round_max_degree = 0;
    }

    pub fn set_context(&mut self, step: Option<&str>) {
        self.current_step = step.map(|s| s.to_string());
        self.current_substep = None;
    }

    pub fn set_subcontext(&mut self, substep: Option<&str>) {
        self.current_substep = substep.map(|s| s.to_string());
    }

    // TODO: Add support for im
    pub fn mark_end_of_round(&mut self, round: usize) {
        // Store the max value for this round
        while self.max_values_by_round.len() <= round {
            self.max_values_by_round.push(0);
        }
        self.max_values_by_round[round] = self.round_max_value;

        // Store the max degree for this round
        while self.max_degrees_by_round.len() <= round {
            self.max_degrees_by_round.push(0);
        }
        self.max_degrees_by_round[round] = self.round_max_degree;
    }

    pub fn generate_pil_round_file(&self, round: usize) -> std::io::Result<()> {
        // Create output directory if it doesn't exist
        let pil_output_dir = match &self.config.pil_output_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };
        fs::create_dir_all(pil_output_dir)?;

        // Get all events for this round
        let round_events: Vec<_> = self.expr_events.iter().filter(|e| e.round == round).collect();

        let file_path = pil_output_dir.join(format!("round{}.pil", round));
        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);

        // TODO: Divide between round resets and round ims
        writeln!(writer, "//! Round {} expressions for the Keccak-f[1600] permutation.", round)?;
        writeln!(writer, "//! DO NOT EDIT - This file is automatically generated.")?;
        writeln!(writer)?;
        writeln!(writer, "// Maximum value in this round: {}", self.round_max_value)?;
        writeln!(writer, "// Maximum degree in this round: {}", self.round_max_degree)?;
        writeln!(writer, "// Number of ims in this round: {}", self.round_im_count)?;
        writeln!(writer, "// Number of resets in this round: {}", self.round_reset_count)?;
        writeln!(writer)?;

        if round_events.is_empty() {
            writeln!(writer, "// No expression events occurred in this round")?;
            writer.flush()?;
            return Ok(());
        }

        for (i, event) in round_events.iter().enumerate() {
            if let Some(old_expr) = self.get_expression(event.old_expr_id) {
                writeln!(
                    writer,
                    "// [{} #{i}] Degree: {} -> {}, Max Value: {} -> {}",
                    event.op_type,
                    event.original_degree,
                    event.new_degree,
                    event.original_max_value,
                    event.new_max_value
                )?;

                match self.config.out_prefix {
                    Some(ref prefix) => {
                        writeln!(
                            writer,
                            "{}[{}][{}] = {};",
                            prefix, self.current_round, i, old_expr
                        )?;
                    }
                    None => {
                        writeln!(writer, "out[{}][{}] = {};", self.current_round, i, old_expr)?;
                    }
                }
                writeln!(writer)?;
            }
        }

        writer.flush()?;
        Ok(())
    }

    pub fn generate_rust_round_file(&self, round: usize) -> std::io::Result<()> {
        // Create output directory if it doesn't exist
        let rust_output_dir = match &self.config.rust_output_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };
        fs::create_dir_all(rust_output_dir)?;

        // Get all events for this round
        let round_events: Vec<_> = self.expr_events.iter().filter(|e| e.round == round).collect();

        let file_path = rust_output_dir.join(format!("round_{}.rs", round));
        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);

        // Generate Rust module header
        writeln!(writer, "//! Round {} expressions for the Keccak-f[1600] permutation.", round)?;
        writeln!(writer, "//! DO NOT EDIT - This file is automatically generated.")?;
        writeln!(writer)?;
        writeln!(writer, "use crate::{{Expression, ExpressionOp}};")?;
        writeln!(writer)?;

        // Generate constants
        writeln!(writer, "/// Maximum value in this round")?;
        writeln!(writer, "pub const MAX_VALUE: u64 = {};", self.round_max_value)?;
        writeln!(writer)?;
        writeln!(writer, "/// Number of intermediate expressions in this round")?;
        writeln!(writer, "pub const IM_COUNT: usize = {};", self.round_im_count)?;
        writeln!(writer)?;
        writeln!(writer, "/// Number of reset expressions in this round")?;
        writeln!(writer, "pub const RESET_COUNT: usize = {};", self.round_reset_count)?;
        writeln!(writer)?;

        if round_events.is_empty() {
            writeln!(writer, "/// No expression events occurred in this round")?;
            writeln!(writer, "pub const EXPRESSIONS: &[&str] = &[];")?;
            writer.flush()?;
            return Ok(());
        }

        // Separate events by type
        let im_events: Vec<_> =
            round_events.iter().filter(|e| matches!(e.op_type, ExpressionOpType::Im)).collect();
        let reset_events: Vec<_> =
            round_events.iter().filter(|e| matches!(e.op_type, ExpressionOpType::Reset)).collect();

        // Generate IM expressions
        if !im_events.is_empty() {
            writeln!(writer, "/// Intermediate expressions for round {}", round)?;
            writeln!(writer, "pub const IM_EXPRESSIONS: &[&str] = &[")?;
            for (i, event) in im_events.iter().enumerate() {
                if let Some(old_expr) = self.get_expression(event.old_expr_id) {
                    let expr_str = format!("{}", old_expr).replace('"', "\\\"");
                    writeln!(
                        writer,
                        "    \"{}\",  // IM[{}]: Degree {} -> {}, Max Value {} -> {}",
                        expr_str,
                        i,
                        event.original_degree,
                        event.new_degree,
                        event.original_max_value,
                        event.new_max_value
                    )?;
                }
            }
            writeln!(writer, "];")?;
            writeln!(writer)?;
        }

        // Generate Reset expressions
        if !reset_events.is_empty() {
            writeln!(writer, "/// Reset expressions for round {}", round)?;
            writeln!(writer, "pub const RESET_EXPRESSIONS: &[&str] = &[")?;
            for (i, event) in reset_events.iter().enumerate() {
                if let Some(old_expr) = self.get_expression(event.old_expr_id) {
                    let expr_str = format!("{}", old_expr).replace('"', "\\\"");
                    writeln!(
                        writer,
                        "    \"{}\",  // RESET[{}]: Degree {} -> {}, Max Value {} -> {}",
                        expr_str,
                        i,
                        event.original_degree,
                        event.new_degree,
                        event.original_max_value,
                        event.new_max_value
                    )?;
                }
            }
            writeln!(writer, "];")?;
            writeln!(writer)?;
        }

        // Generate all expressions array
        writeln!(writer, "/// All expressions for round {} (IM + Reset)", round)?;
        writeln!(writer, "pub const ALL_EXPRESSIONS: &[&str] = &[")?;
        for (i, event) in round_events.iter().enumerate() {
            if let Some(old_expr) = self.get_expression(event.old_expr_id) {
                let expr_str = format!("{}", old_expr).replace('"', "\\\"");
                let event_type = match event.op_type {
                    ExpressionOpType::Im => "IM",
                    ExpressionOpType::Reset => "RESET",
                };
                writeln!(
                    writer,
                    "    \"{}\",  // {}[{}]: Degree {} -> {}, Max Value {} -> {}",
                    expr_str,
                    event_type,
                    i,
                    event.original_degree,
                    event.new_degree,
                    event.original_max_value,
                    event.new_max_value
                )?;
            }
        }
        writeln!(writer, "];")?;
        writeln!(writer)?;

        // Generate expression metadata
        writeln!(writer, "/// Expression metadata for round {}", round)?;
        writeln!(writer, "#[derive(Debug, Clone)]")?;
        writeln!(writer, "pub struct ExpressionMetadata {{")?;
        writeln!(writer, "    pub expression: &'static str,")?;
        writeln!(writer, "    pub event_type: ExpressionEventType,")?;
        writeln!(writer, "    pub original_degree: usize,")?;
        writeln!(writer, "    pub new_degree: usize,")?;
        writeln!(writer, "    pub original_max_value: u64,")?;
        writeln!(writer, "    pub new_max_value: u64,")?;
        writeln!(writer, "    pub step: Option<&'static str>,")?;
        writeln!(writer, "    pub substep: Option<&'static str>,")?;
        writeln!(writer, "}}")?;
        writeln!(writer)?;

        writeln!(writer, "#[derive(Debug, Clone, PartialEq)]")?;
        writeln!(writer, "pub enum ExpressionEventType {{")?;
        writeln!(writer, "    Im,")?;
        writeln!(writer, "    Reset,")?;
        writeln!(writer, "}}")?;
        writeln!(writer)?;

        // Generate metadata array
        writeln!(writer, "pub const EXPRESSION_METADATA: &[ExpressionMetadata] = &[")?;
        for event in round_events.iter() {
            if let Some(old_expr) = self.get_expression(event.old_expr_id) {
                let expr_str = format!("{}", old_expr).replace('"', "\\\"");
                let event_type = match event.op_type {
                    ExpressionOpType::Im => "ExpressionEventType::Im",
                    ExpressionOpType::Reset => "ExpressionEventType::Reset",
                };
                let step = event
                    .step
                    .as_ref()
                    .map(|s| format!("Some(\"{}\")", s))
                    .unwrap_or_else(|| "None".to_string());
                let substep = event
                    .substep
                    .as_ref()
                    .map(|s| format!("Some(\"{}\")", s))
                    .unwrap_or_else(|| "None".to_string());

                writeln!(writer, "    ExpressionMetadata {{")?;
                writeln!(writer, "        expression: \"{}\",", expr_str)?;
                writeln!(writer, "        event_type: {},", event_type)?;
                writeln!(writer, "        original_degree: {},", event.original_degree)?;
                writeln!(writer, "        new_degree: {},", event.new_degree)?;
                writeln!(writer, "        original_max_value: {},", event.original_max_value)?;
                writeln!(writer, "        new_max_value: {},", event.new_max_value)?;
                writeln!(writer, "        step: {},", step)?;
                writeln!(writer, "        substep: {},", substep)?;
                writeln!(writer, "    }},")?;
            }
        }
        writeln!(writer, "];")?;

        writer.flush()?;
        Ok(())
    }

    fn get_expression(&self, id: usize) -> Option<&Expression> {
        self.exprs.get(id)
    }

    fn set_expression(&mut self, id: usize, expr: Expression) {
        self.exprs.insert(id, expr.simplify());
    }

    pub fn create_op_expression(&mut self, op: &ExpressionOp, id1: usize, id2: usize) -> usize {
        // Threshold check before performing operation, it may reset operands
        self.check_and_reset_exprs_before_operation(op, id1, id2);

        // Set the new expression
        let expr1 = self.get_expression(id1).cloned().unwrap();
        let expr2 = self.get_expression(id2).cloned().unwrap();
        let expr3 = Expression::op(op, expr1, expr2);

        // Track max value
        let expr3_max = expr3.max_value();
        if expr3_max > self.round_max_value {
            self.round_max_value = expr3_max;
        }

        // Track max degree
        let expr3_degree = expr3.degree();
        if expr3_degree > self.round_max_degree {
            self.round_max_degree = expr3_degree;
        }

        // Create new expression slot
        let new_id = self.exprs_count;
        self.exprs_count += 1;
        self.set_expression(new_id, expr3);
        new_id
    }

    /// Check if performing an operation would exceed the threshold and reset if needed
    fn check_and_reset_exprs_before_operation(
        &mut self,
        op: &ExpressionOp,
        id1: usize,
        id2: usize,
    ) {
        let expr1 = self.get_expression(id1).cloned().unwrap();
        let expr2 = self.get_expression(id2).cloned().unwrap();

        // Predict the result based on operation type
        let predicted_expr = Expression::op(op, expr1.clone(), expr2.clone());
        let res_max_value = predicted_expr.max_value();
        let res_degree = predicted_expr.degree();

        // Check if predicted result exceeds either threshold
        let max_value_threshold = self.config.value_reset_threshold;
        let degree_threshold = self.config.degree_reset_threshold;

        let exceeds_max_value = res_max_value > max_value_threshold as u64;
        let exceeds_degree = res_degree > degree_threshold;

        if exceeds_max_value || exceeds_degree {
            let op1_max_value = expr1.max_value();
            let op2_max_value = expr2.max_value();
            let op1_degree = expr1.degree();
            let op2_degree = expr2.degree();

            // Determine reset reason
            let reset_reason = if exceeds_max_value && exceeds_degree {
                ExpressionReason::BothThresholds
            } else if exceeds_max_value {
                ExpressionReason::MaxValue
            } else {
                ExpressionReason::Degree
            };

            // Determine which operand to reset first based on which threshold was exceeded
            let should_reset_first = if exceeds_max_value && exceeds_degree {
                // Both thresholds exceeded - reset the operand with higher combined "score"
                let op1_score = (op1_max_value as f64 / max_value_threshold as f64)
                    + (op1_degree as f64 / degree_threshold as f64);
                let op2_score = (op2_max_value as f64 / max_value_threshold as f64)
                    + (op2_degree as f64 / degree_threshold as f64);
                op1_score >= op2_score
            } else if exceeds_max_value {
                // Only max value exceeded - reset operand with higher max value
                op1_max_value >= op2_max_value
            } else {
                // Only degree exceeded - reset operand with higher degree
                op1_degree >= op2_degree
            };

            // Reset the chosen operand first (but only if it's not a constant)
            if should_reset_first && (op1_max_value > 1 || op1_degree > 0) {
                self.create_reset_expression(
                    id1,
                    false,
                    Some(reset_reason.clone()),
                    Some(op1_max_value),
                    Some(op2_max_value),
                    Some(res_max_value),
                );
            } else if op2_max_value > 1 || op2_degree > 0 {
                self.create_reset_expression(
                    id2,
                    false,
                    Some(reset_reason.clone()),
                    Some(op1_max_value),
                    Some(op2_max_value),
                    Some(res_max_value),
                );
            }

            // Check if we need to reset the second operand too
            let expr1 = self.get_expression(id1).cloned().unwrap();
            let expr2 = self.get_expression(id2).cloned().unwrap();
            let new_predicted_expr = Expression::op(op, expr1.clone(), expr2.clone());
            let new_predicted_max = new_predicted_expr.max_value();
            let new_predicted_degree = new_predicted_expr.degree();

            let still_exceeds_max = new_predicted_max > max_value_threshold as u64;
            let still_exceeds_degree = new_predicted_degree > degree_threshold;

            if still_exceeds_max || still_exceeds_degree {
                let new_op1_max = expr1.max_value();
                let new_op2_max = expr2.max_value();
                let new_op1_degree = expr1.degree();
                let new_op2_degree = expr2.degree();

                // Determine reset reason for second reset
                let second_reset_reason = if still_exceeds_max && still_exceeds_degree {
                    ExpressionReason::BothThresholds
                } else if still_exceeds_max {
                    ExpressionReason::MaxValue
                } else {
                    ExpressionReason::Degree
                };

                // Reset the other operand
                if should_reset_first && (new_op2_max > 1 || new_op2_degree > 0) {
                    self.create_reset_expression(
                        id2,
                        false,
                        Some(second_reset_reason),
                        Some(new_op1_max),
                        Some(new_op2_max),
                        Some(new_predicted_max),
                    );
                } else if new_op1_max > 1 || new_op1_degree > 0 {
                    self.create_reset_expression(
                        id1,
                        false,
                        Some(second_reset_reason),
                        Some(new_op1_max),
                        Some(new_op2_max),
                        Some(new_predicted_max),
                    );
                }
            }
        }
    }

    pub fn create_proxy_expression(&mut self, id: usize) -> usize {
        let expr = self.get_expression(id).cloned().unwrap();
        let original_degree = expr.degree();
        let original_max_value = expr.max_value();

        // Create new proxy expression
        let new_proxy_id = self.proxy_count;
        self.proxy_count += 1;
        let new_expr = Expression::proxy(new_proxy_id, original_degree, original_max_value, expr);

        // Create new expression slot
        let new_id = self.exprs_count;
        self.exprs_count += 1;
        self.set_expression(new_id, new_expr);
        new_id
    }

    fn create_im_expression(
        &mut self,
        id: usize,
        manual: bool,
        reason: Option<ExpressionReason>,
        op1_max_value: Option<u64>,
        op2_max_value: Option<u64>,
        res_max_value: Option<u64>,
    ) -> usize {
        let expr = self.get_expression(id).cloned().unwrap();
        let original_degree = expr.degree();
        let original_max_value = expr.max_value();

        // TODO // Create new expression by factoring out complexity
        // let new_expr = self.factor_expression(expr, original_degree, original_max_value, false);
        // let new_degree = new_expr.degree();
        // let new_max_value = new_expr.max_value();

        // Create the new intermediate expression
        let new_im_id = self.round_im_count;
        self.round_im_count += 1;
        self.im_count += 1;

        let new_expr = Expression::im(
            new_im_id,
            original_degree,
            original_max_value,
            self.config.in_prefix.clone(),
            Some(self.current_round + 1),
        );
        let new_degree = new_expr.degree();
        let new_max_value = new_expr.max_value();

        // Create new expression slot
        let new_id = self.exprs_count;
        self.exprs_count += 1;
        self.set_expression(new_id, new_expr);

        // Record the Im event
        self.record_expression_event(
            manual,
            reason,
            ExpressionOpType::Im,
            id,
            original_degree,
            new_degree,
            original_max_value,
            new_max_value,
            op1_max_value,
            op2_max_value,
            res_max_value,
        );

        new_id
    }

    pub fn create_manual_im_expression(&mut self, id: usize) -> usize {
        self.create_im_expression(id, true, None, None, None, None)
    }

    fn create_reset_expression(
        &mut self,
        id: usize,
        manual: bool,
        reason: Option<ExpressionReason>,
        op1_max_value: Option<u64>,
        op2_max_value: Option<u64>,
        res_max_value: Option<u64>,
    ) -> usize {
        let expr = self.get_expression(id).cloned().unwrap();
        let original_degree = expr.degree();
        let original_max_value = expr.max_value();

        // // Create new expression by factoring
        // let new_expr = self.factor_expression(expr, original_degree, original_max_value, true);
        // let new_degree = new_expr.degree();
        // let new_max_value = new_expr.max_value();

        // Create new reset expression
        let new_reset_id = self.round_reset_count;
        self.round_reset_count += 1;
        self.reset_count += 1;

        let new_expr = Expression::reset(
            new_reset_id,
            original_degree,
            self.config.in_prefix.clone(),
            Some(self.current_round + 1),
        );
        let new_degree = new_expr.degree();
        let new_max_value = new_expr.max_value();

        // Create new expression slot
        let new_id = self.exprs_count;
        self.exprs_count += 1;
        self.set_expression(new_id, new_expr);

        // Record the Reset event
        self.record_expression_event(
            manual,
            reason,
            ExpressionOpType::Reset,
            id,
            original_degree,
            new_degree,
            original_max_value,
            new_max_value,
            op1_max_value,
            op2_max_value,
            res_max_value,
        );

        new_id
    }

    pub fn create_manual_reset_expression(&mut self, id: usize) -> usize {
        self.create_reset_expression(id, true, None, None, None, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_expression_event(
        &mut self,
        manual: bool,
        reason: Option<ExpressionReason>,
        op_type: ExpressionOpType,
        old_expr_id: usize,
        original_degree: usize,
        new_degree: usize,
        original_max_value: u64,
        new_max_value: u64,
        op1_max_value: Option<u64>,
        op2_max_value: Option<u64>,
        res_max_value: Option<u64>,
    ) {
        let event = ExpressionEvent {
            manual,
            op_type,
            reason,
            round: self.current_round,
            step: self.current_step.clone(),
            substep: self.current_substep.clone(),
            old_expr_id,
            original_degree,
            new_degree,
            original_max_value,
            new_max_value,
            op1_max_value,
            op2_max_value,
            res_max_value,
        };

        self.expr_events.push(event.clone());
    }

    pub fn copy_sout_expr_ids_to_sin_expr_ids(&mut self) {
        assert!(self.sin_expr_ids.len() >= self.sout_expr_ids.len());

        self.sin_expr_ids.copy_from_slice(&self.sout_expr_ids);
    }

    pub fn print_expression(&self, id: usize) {
        if let Some(expr) = self.get_expression(id) {
            println!("expr[{}] = {}", id, expr);
        } else {
            println!("No expression found for ref {}", id);
        }
    }

    pub fn print_sin_expr(&self, sin_expr_id: usize) {
        let expr_id = self.sin_expr_ids[sin_expr_id];
        Self::print_expression(self, expr_id);
    }

    pub fn print_sout_expr(&self, sout_expr_id: usize) {
        let expr_id = self.sout_expr_ids[sout_expr_id];
        Self::print_expression(self, expr_id);
    }

    pub fn print_round_events(&self, round: usize, limit: Option<usize>) {
        let limit = limit.unwrap_or(usize::MAX);
        let events: Vec<_> = self.expr_events.iter().filter(|e| e.round == round).collect();

        println!("\n--- Round {} Expression Events ---", round);

        println!("There were {} expression events in round {}", events.len(), round);
        for (i, event) in events.iter().enumerate().take(limit) {
            let event_type = if event.manual { "Manual" } else { "Auto" };

            let mut reason_and_values = String::new();
            if let Some(ref r) = event.reason {
                reason_and_values.push_str(&format!(", Reason: {}", r));

                // Only show operand values for max value related resets
                if matches!(r, ExpressionReason::MaxValue | ExpressionReason::BothThresholds) {
                    if let (Some(op1), Some(op2), Some(res)) =
                        (event.op1_max_value, event.op2_max_value, event.res_max_value)
                    {
                        reason_and_values.push_str(&format!(
                            ", Op1 Max: {}, Op2 Max: {}, Result Max: {}",
                            op1, op2, res
                        ));
                    }
                }
            }

            let mut step_substep = String::new();
            match (event.step.clone(), event.substep.clone()) {
                (Some(ref s), Some(ref ss)) => {
                    step_substep.push_str(&format!("Step: {},", s));
                    step_substep.push_str(&format!(" Substep: {},", ss));
                }
                (Some(ref s), None) => {
                    step_substep.push_str(&format!("Step: {},", s));
                }
                (None, Some(ref ss)) => {
                    step_substep.push_str(&format!("Substep: {},", ss));
                }
                (None, None) => {}
            }

            println!(
                "  {}. [{}] Type: {}{}, {} Degree: {}→{}, Max Value: {}→{}",
                i + 1,
                event_type,
                event.op_type,
                reason_and_values,
                step_substep,
                event.original_degree,
                event.new_degree,
                event.original_max_value,
                event.new_max_value
            );
        }
    }

    pub fn print_summary(&self) {
        println!("\n--- Expression Summary ---");

        // Group by type
        let mut automatic_im = 0;
        let mut manual_im = 0;
        let mut automatic_reset = 0;
        let mut manual_reset = 0;

        for event in &self.expr_events {
            match (event.manual, &event.op_type) {
                (true, ExpressionOpType::Im) => manual_im += 1,
                (true, ExpressionOpType::Reset) => manual_reset += 1,
                (false, ExpressionOpType::Im) => automatic_im += 1,
                (false, ExpressionOpType::Reset) => automatic_reset += 1,
            }
        }

        let max_value = self.max_values_by_round.iter().max().cloned().unwrap_or(0);
        let max_degree = self.max_degrees_by_round.iter().max().cloned().unwrap_or(0);

        println!("  Maximum Expression Value: {}", max_value);
        println!("  Maximum Expression Degree: {}", max_degree);
        println!("  Events:");
        println!("    - Automatic Im: {}", automatic_im);
        println!("    - Automatic Reset: {}", automatic_reset);
        println!("    - Manual Im: {}", manual_im);
        println!("    - Manual Reset: {}", manual_reset);
        println!();
    }

    pub fn generate_summary_file(&self) -> std::io::Result<()> {
        // Create output directory if it doesn't exist
        let pil_output_dir = match &self.config.pil_output_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };
        fs::create_dir_all(pil_output_dir)?;

        let file_path = pil_output_dir.join("round_summary.pil");
        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);

        writeln!(writer, "//! Expression summary for the Keccak-f[1600] permutation.")?;
        writeln!(writer, "//! DO NOT EDIT - This file is automatically generated.")?;
        writeln!(writer)?;

        // Configuration
        writeln!(writer, "// Configuration:")?;
        writeln!(writer, "// --------------")?;
        writeln!(writer, "// Input number: {}", self.config.sin_count)?;
        writeln!(writer, "// Output number: {}", self.config.sout_count)?;
        writeln!(writer, "// Value threshold: {}", self.config.value_reset_threshold)?;
        writeln!(writer, "// Degree threshold: {}", self.config.degree_reset_threshold)?;
        writeln!(writer)?;

        // Overall statistics
        writeln!(writer, "// Overall Statistics:")?;
        writeln!(writer, "// -------------------")?;
        // writeln!(writer, "// Total expressions created: {}", self.exprs_count)?;
        writeln!(writer, "// Total Proxy expressions: {}", self.proxy_count)?;
        writeln!(writer, "// Total Im expressions: {}", self.im_count)?;
        writeln!(writer, "// Total Reset expressions: {}", self.reset_count)?;

        let overall_max = self.max_values_by_round.iter().max().cloned().unwrap_or(0);
        writeln!(writer, "// Maximum expression value: {}", overall_max)?;
        writeln!(writer)?;

        writeln!(writer, "const int RESET_NUM = {};", self.reset_count)?;
        writeln!(writer, "const int IM_NUM = {};", self.im_count)?;
        writeln!(writer, "const int MAX_VALUE = {};", overall_max)?;
        writeln!(writer)?;

        // Collect reset and im counts per round
        let num_rounds = self.max_values_by_round.len();
        let mut reset_counts_by_round = Vec::new();
        let mut im_counts_by_round = Vec::new();

        for round in 0..num_rounds {
            let round_events: Vec<_> =
                self.expr_events.iter().filter(|e| e.round == round).collect();

            let reset_count = round_events
                .iter()
                .filter(|e| matches!(e.op_type, ExpressionOpType::Reset))
                .count();

            let im_count =
                round_events.iter().filter(|e| matches!(e.op_type, ExpressionOpType::Im)).count();

            reset_counts_by_round.push(reset_count);
            im_counts_by_round.push(im_count);
        }

        // Per-round statistics
        if !self.max_values_by_round.is_empty() {
            writeln!(writer, "// Per-Round Statistics:")?;
            writeln!(writer, "// ---------------------")?;
            writeln!(
                writer,
                "{:<10} {:<15} {:<15} {:<15} {:<15}",
                "// Round", "Reset Count", "Im Count", "Total Events", "Max Value"
            )?;
            writeln!(writer, "// {}", "-".repeat(70))?;

            for round in 0..num_rounds {
                let max_val = self.max_values_by_round.get(round).cloned().unwrap_or(0);
                let reset_count = reset_counts_by_round[round];
                let im_count = im_counts_by_round[round];
                let total_events = reset_count + im_count;

                writeln!(
                    writer,
                    "// {:<10} {:<15} {:<15} {:<15} {:<15}",
                    round, reset_count, im_count, total_events, max_val
                )?;
            }
            writeln!(writer)?;
        }

        // Generate 2D array declaration for reset counts
        if !reset_counts_by_round.is_empty() {
            writeln!(writer, "// Reset counts by round")?;
            writeln!(writer, "const int RESET_NUM_BY_ROUND[{}];", num_rounds)?;
            writeln!(writer)?;

            // Generate array initialization (as comments for reference)
            for (round, &count) in reset_counts_by_round.iter().enumerate() {
                writeln!(writer, "RESET_NUM_BY_ROUND[{}] = {};", round, count)?;
            }
            writeln!(writer)?;
        }

        writer.flush()?;
        Ok(())
    }
}
