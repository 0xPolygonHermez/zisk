use std::fmt::Display;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

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
    current_round: Option<usize>,
    current_step: Option<String>,
    current_substep: Option<String>,

    /// Global maximum value encountered
    round_max_value: u64,
    max_values_by_round: Vec<u64>,
}

pub struct ExpressionManagerConfig {
    pub reset_threshold: u32,
    pub sin_count: usize,
    pub sout_count: usize,
    pub reset_prefix: String,
    pub im_prefix: String,
}

#[derive(Clone)]
struct ExpressionEvent {
    manual: bool,
    op_type: ExpressionOpType,
    round: Option<usize>,
    step: Option<String>,
    substep: Option<String>,
    old_expr_id: usize,
    new_expr_id: usize,
    original_degree: usize,
    new_degree: usize,
    original_max_value: u64,
    new_max_value: u64,
    op1_max_value: Option<u64>,
    op2_max_value: Option<u64>,
    res_max_value: Option<u64>,
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
        let reset_threshold = config.reset_threshold;
        let sin_count = config.sin_count;
        let sout_count = config.sout_count;

        assert!(reset_threshold > 0 && reset_threshold & (reset_threshold - 1) == 0);
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
            exprs.push(Expression::Input(i));
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
            current_round: None,
            current_step: None,
            current_substep: None,
            round_max_value: 0,
            max_values_by_round: Vec::new(),
        }
    }

    pub fn mark_begin_round(&mut self, round: usize) {
        self.current_round = Some(round);
        self.round_im_count = 0;
        self.round_reset_count = 0;
        self.round_max_value = 0;
    }

    pub fn set_context(&mut self, step: &str) {
        self.current_step = Some(step.to_string());
    }

    pub fn set_subcontext(&mut self, substep: &str) {
        self.current_substep = Some(substep.to_string());
    }

    // TODO: Add support for im
    pub fn mark_end_of_round(&mut self, round: usize, output_dir: &Path) -> std::io::Result<()> {
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;

        // Get all events for this round
        let round_events: Vec<_> =
            self.expr_events.iter().filter(|e| e.round == Some(round)).collect();

        if round_events.is_empty() {
            return Ok(());
        }

        // Store the max value for this round
        while self.max_values_by_round.len() <= round {
            self.max_values_by_round.push(0);
        }
        self.max_values_by_round[round] = self.round_max_value;

        // Generate file for this round
        let file_path = output_dir.join(format!("round{}.pil", round));
        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);

        // TODO: Divide between round resets and round ims
        writeln!(writer, "//! Round {} expressions for the Keccak-f[1600] permutation.", round)?;
        writeln!(writer, "//! DO NOT EDIT - This file is automatically generated.")?;
        writeln!(writer)?;
        writeln!(writer, "// Maximum value in this round: {}", self.round_max_value)?;
        writeln!(writer)?;
        writeln!(writer, "const int R{}_RESETS = {};", round, round_events.len())?;
        writeln!(writer)?;
        writeln!(writer, "const expr {}{}[R{}_RESETS];", self.config.reset_prefix, round, round)?;
        writeln!(writer)?;

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
                writeln!(writer, "{}{}[{}] = {};", self.config.reset_prefix, round, i, old_expr)?;
                writeln!(writer)?;
            }
        }

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
        self.check_val_and_reset_exprs_before_operation(op, id1, id2);

        // Set the new expression
        let expr1 = self.get_expression(id1).cloned().unwrap();
        let expr2 = self.get_expression(id2).cloned().unwrap();
        let expr3 = Expression::op(op, expr1, expr2);

        // Track max value
        let expr3_max = expr3.max_value();
        if expr3_max > self.round_max_value {
            self.round_max_value = expr3_max;
        }

        // Create new expression slot
        let new_id = self.exprs_count;
        self.exprs_count += 1;
        self.set_expression(new_id, expr3);
        new_id
    }

    /// Check if performing an operation would exceed the threshold and reset if needed
    fn check_val_and_reset_exprs_before_operation(
        &mut self,
        op: &ExpressionOp,
        id1: usize,
        id2: usize,
    ) {
        let expr1 = self.get_expression(id1).cloned().unwrap();
        let expr2 = self.get_expression(id2).cloned().unwrap();

        // Predict the result based on operation type
        let res_max_value = Self::eval_op_on_exprs(op, expr1.clone(), expr2.clone());

        // If predicted result exceeds threshold, reset the largest operand(s)
        let threshold = self.config.reset_threshold;
        if res_max_value > threshold as u64 {
            let op1_max_value = expr1.max_value();
            let op2_max_value = expr2.max_value();

            // Reset the largest operand first
            if op1_max_value >= op2_max_value && op1_max_value > 1 {
                self.create_reset_expression(
                    id1,
                    false,
                    Some(op1_max_value),
                    Some(op2_max_value),
                    Some(res_max_value),
                );
            } else if op2_max_value > 1 {
                self.create_reset_expression(
                    id2,
                    false,
                    Some(op1_max_value),
                    Some(op2_max_value),
                    Some(res_max_value),
                );
            }

            // Check if we need to reset the second operand too
            let expr1 = self.get_expression(id1).cloned().unwrap();
            let expr2 = self.get_expression(id2).cloned().unwrap();
            let new_predicted = Self::eval_op_on_exprs(op, expr1, expr2);

            if new_predicted > threshold as u64 {
                // Reset the other operand
                if op1_max_value >= op2_max_value && op2_max_value > 1 {
                    self.create_reset_expression(
                        id2,
                        false,
                        Some(op1_max_value),
                        Some(op2_max_value),
                        Some(res_max_value),
                    );
                } else if op1_max_value > 1 {
                    self.create_reset_expression(
                        id1,
                        false,
                        Some(op1_max_value),
                        Some(op2_max_value),
                        Some(res_max_value),
                    );
                }
            }
        }
    }

    fn eval_op_on_exprs(op: &ExpressionOp, expr1: Expression, expr2: Expression) -> u64 {
        let expr = Expression::op(op, expr1, expr2);
        expr.max_value()
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
            self.current_round,
            Some(self.config.im_prefix.clone()),
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
            ExpressionOpType::Im,
            id,
            new_id,
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
        self.create_im_expression(id, true, None, None, None)
    }

    fn create_reset_expression(
        &mut self,
        id: usize,
        manual: bool,
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
            self.current_round,
            Some(self.config.reset_prefix.clone()),
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
            ExpressionOpType::Reset,
            id,
            new_id,
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
        self.create_reset_expression(id, true, None, None, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_expression_event(
        &mut self,
        manual: bool,
        op_type: ExpressionOpType,
        old_expr_id: usize,
        new_expr_id: usize,
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
            round: self.current_round,
            step: self.current_step.clone(),
            substep: self.current_substep.clone(),
            old_expr_id,
            new_expr_id,
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
        let events: Vec<_> = self.expr_events.iter().filter(|e| e.round == Some(round)).collect();

        println!("\n--- Round {} Expression Events ---", round);

        println!("There were {} expression events in round {}", events.len(), round);
        for (i, event) in events.iter().enumerate().take(limit) {
            let event_type = if event.manual { "Manual" } else { "Auto" };

            println!(
                "{}. [{}] Type: {}, Step: \"{}\", Substep: \"{}\", Degree: {}→{}, Max Value: {}→{}",
                i + 1,
                event_type,
                event.op_type,
                event.step.clone().unwrap_or("N/A".to_string()),
                event.substep.clone().unwrap_or("N/A".to_string()),
                event.original_degree,
                event.new_degree,
                event.original_max_value,
                event.new_max_value
            );
            // println!("Expr: {}", self.get_expression(event.old_expr_id).unwrap());

            if let Some(predicted) = event.res_max_value {
                println!(
                    "\tOp1 Max: {:?}, Op2 Max: {:?}, Predicted Result Max: {}",
                    event.op1_max_value, event.op2_max_value, predicted
                );
            }
        }
    }

    pub fn print_summary(&self) {
        println!("\n--- Expression Event Summary ---");

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

        println!("\tAutomatic Im: {}", automatic_im);
        println!("\tAutomatic Reset: {}", automatic_reset);
        println!("\tManual Im: {}", manual_im);
        println!("\tManual Reset: {}", manual_reset);

        let max_value = self.max_values_by_round.iter().max().cloned().unwrap_or(0);
        println!("\tMaximum expression value: {}", max_value);
    }
}
