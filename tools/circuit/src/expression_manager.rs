use std::collections::HashMap;

use super::Expression;

#[derive(Debug)]
pub struct ExpressionManager {
    /// Maps each reference to its expression
    expressions: HashMap<u64, Expression>,

    /// Counter for generating unique IDs
    counter: u64,

    /// All expression events (both Im and Reset)
    expression_events: Vec<ExpressionEvent>,

    /// Expression events grouped by round
    events_by_round: HashMap<usize, Vec<ExpressionEvent>>,

    /// Current context for tracking
    current_round: Option<usize>,
    current_step: Option<String>,
    current_substep: Option<String>,

    /// Global maximum value encountered
    max_value: u64,

    /// Counters
    proxy_count: usize,
    im_count: usize,
    reset_count: usize,
}

#[derive(Debug, Clone)]
struct ExpressionEvent {
    manual: bool,
    op_type: ExpressionOpType,
    step: Option<String>,
    substep: Option<String>,
    original_degree: usize,
    new_degree: usize,
    original_max_value: u64,
    new_max_value: u64,
    predicted_op_max: Option<u64>,
    // pub first_value: u64,
    // pub second_value: u64,
    // pub op_value: u64,
}

#[derive(Debug, Clone)]
enum ExpressionOpType {
    Im,
    Reset,
}

impl Default for ExpressionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpressionManager {
    pub fn new() -> Self {
        Self {
            expressions: HashMap::new(),
            counter: 0,
            expression_events: Vec::new(),
            events_by_round: HashMap::new(),
            current_round: None,
            current_step: None,
            current_substep: None,
            max_value: 0,
            proxy_count: 0,
            im_count: 0,
            reset_count: 0,
        }
    }

    /// Get next unique ID
    pub fn next_id(&mut self) -> u64 {
        let id = self.counter;
        self.counter += 1;
        id
    }

    pub fn set_context(&mut self, round: usize, step: &str) {
        self.current_round = Some(round);
        self.current_step = Some(step.to_string());
    }

    pub fn set_subcontext(&mut self, substep: &str) {
        self.current_substep = Some(substep.to_string());
    }

    pub fn get_expression(&self, ref_id: u64) -> Option<&Expression> {
        self.expressions.get(&ref_id)
    }

    pub fn set_expression(&mut self, ref_id: u64, expr: Expression) {
        self.expressions.insert(ref_id, expr.simplify());
    }

    pub fn set_new_expression(&mut self, expr: Expression) -> u64 {
        let expr_ref = self.next_id();
        self.set_expression(expr_ref, expr);
        expr_ref
    }

    pub fn get_max_value(&self) -> u64 {
        self.max_value
    }

    pub fn create_proxy_expression(&mut self, ref_id: u64) {
        if let Some(expr) = self.get_expression(ref_id).cloned() {
            let original_degree = expr.degree();
            let original_max_value = expr.max_value();

            let new_expr =
                Expression::proxy(self.next_id(), original_degree, original_max_value, expr);

            // Replace the old expression with the proxy
            self.set_expression(ref_id, new_expr);
            self.proxy_count += 1;
        }
    }

    pub fn create_im_expression(
        &mut self,
        ref_id: u64,
        manual: bool,
        predicted_op_max: Option<u64>,
    ) {
        if let Some(expr) = self.get_expression(ref_id).cloned() {
            let original_degree = expr.degree();
            let original_max_value = expr.max_value();

            // TODO // Create new expression by factoring out complexity
            // let new_expr = self.factor_expression(expr, original_degree, original_max_value, false);
            // let new_degree = new_expr.degree();
            // let new_max_value = new_expr.max_value();

            let new_expr = Expression::im(self.next_id(), original_degree, original_max_value);
            let new_degree = new_expr.degree();

            // Record the Im event
            self.record_expression_event(
                manual,
                ExpressionOpType::Im,
                original_degree,
                new_degree,
                original_max_value,
                original_max_value,
                predicted_op_max,
            );

            self.set_expression(ref_id, new_expr);
            self.im_count += 1;
        }
    }

    pub fn create_reset_expression(
        &mut self,
        ref_id: u64,
        manual: bool,
        predicted_op_max: Option<u64>,
    ) -> bool {
        if let Some(expr) = self.get_expression(ref_id).cloned() {
            let original_degree = expr.degree();
            let original_max_value = expr.max_value();

            // // Create new expression by factoring
            // let new_expr = self.factor_expression(expr, original_degree, original_max_value, true);
            // let new_degree = new_expr.degree();
            // let new_max_value = new_expr.max_value();

            let new_expr = Expression::reset(self.next_id(), original_degree);
            let new_degree = new_expr.degree();
            let new_max_value = new_expr.max_value();

            // Record the Reset event
            self.record_expression_event(
                manual,
                ExpressionOpType::Reset,
                original_degree,
                new_degree,
                original_max_value,
                new_max_value,
                predicted_op_max,
            );

            self.set_expression(ref_id, new_expr);
            self.reset_count += 1;
            true
        } else {
            false
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn record_expression_event(
        &mut self,
        manual: bool,
        op_type: ExpressionOpType,
        original_degree: usize,
        new_degree: usize,
        original_max_value: u64,
        new_max_value: u64,
        predicted_op_max: Option<u64>,
    ) {
        let event = ExpressionEvent {
            manual,
            op_type,
            step: self.current_step.clone(),
            substep: self.current_substep.clone(),
            original_degree,
            new_degree,
            original_max_value,
            new_max_value,
            predicted_op_max,
        };

        self.expression_events.push(event.clone());

        if let Some(round) = self.current_round {
            self.events_by_round.entry(round).or_default().push(event);
        }
    }

    pub fn print_expression(&self, ref_id: u64) {
        if let Some(expr) = self.get_expression(ref_id) {
            println!("expr[{}] = {}", ref_id, expr);
        } else {
            println!("No expression found for ref {}", ref_id);
        }
    }

    pub fn print_round_events(&self, round: usize, limit: Option<usize>) {
        let limit = limit.unwrap_or(usize::MAX);

        println!("\n--- Round {} Expression Events ---", round);
        if let Some(events) = self.events_by_round.get(&round) {
            println!("There were {} expression events in round {}", events.len(), round);
            for (i, event) in events.iter().enumerate().take(limit) {
                let event_type = if event.manual { "Manual" } else { "Auto" };
                let op_type = match &event.op_type {
                    ExpressionOpType::Im => "Im",
                    ExpressionOpType::Reset => "Reset",
                };

                // 1. Step: "θ", Substep: "Compute A'[x, y, z]", Op: Unknown, First max: 2477476, Second max: 0, Op max: 0
                println!(
                    "{}. [{}] Type: \"{}\", Step: \"{}\", Substep: \"{}\", Degree: {}→{}, Max Value: {}→{}",
                    i + 1,
                    event_type,
                    op_type,
                    event.step.clone().unwrap_or("N/A".to_string()),
                    event.substep.clone().unwrap_or("N/A".to_string()),
                    event.original_degree,
                    event.new_degree,
                    event.original_max_value,
                    event.new_max_value
                );

                if let Some(predicted) = event.predicted_op_max {
                    println!("    (predicted operation max would have been: {})", predicted);
                }
            }
        } else {
            println!("No expression events in round {}", round);
        }
    }

    pub fn print_expression_summary(&self) {
        println!("\n--- Expression Event Summary ---");

        // Group by type
        let mut automatic_im = 0;
        let mut manual_im = 0;
        let mut automatic_reset = 0;
        let mut manual_reset = 0;

        for event in &self.expression_events {
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

        println!("\tMaximum expression value: {}", self.max_value);
    }
}
