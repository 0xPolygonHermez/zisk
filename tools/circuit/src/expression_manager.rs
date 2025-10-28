use std::collections::HashMap;

use super::Expression;

#[derive(Debug)]
pub struct ExpressionManager {
    /// Maps each reference to its expression
    pub expressions: HashMap<u64, Expression>,

    /// Counter for generating unique IDs
    pub counter: u64,

    /// All expression events (both Im and Reset)
    pub expression_events: Vec<ExpressionEvent>,

    /// Expression events grouped by round
    pub events_by_round: HashMap<usize, Vec<ExpressionEvent>>,

    /// Current context for tracking
    pub current_round: Option<usize>,
    pub current_step: Option<String>,
    pub current_substep: Option<String>,

    /// Global maximum value encountered
    pub max_value: u64,

    /// Counters
    pub proxy_count: usize,
    pub im_count: usize,
    pub reset_count: usize,
}

#[derive(Debug, Clone)]
pub struct ExpressionEvent {
    pub round: Option<usize>,
    pub step: Option<String>,
    pub substep: Option<String>,
    pub operation_type: String,
    pub ref_id: u64,
    pub original_degree: usize,
    pub original_max_value: u64,
    pub new_degree: usize,
    pub new_max_value: u64,
    pub event_type: ExpressionEventType,
    pub predicted_operation_max: Option<u64>,
    // pub first_value: u64,
    // pub second_value: u64,
    // pub op_value: u64,
    // pub reset_type: ExpressionEventType,
}

#[derive(Debug, Clone)]
pub enum ExpressionEventType {
    Automatic,      // Reset/Im due to threshold
    Manual(String), // Manual reset/Im with reason
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

    /// Set context for tracking
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

    /// Create a proxy expression from an expression
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

    /// Create Im expression
    pub fn create_im_expression(
        &mut self,
        ref_id: u64,
        reason: Option<String>,
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
            let event_type = if let Some(reason) = reason {
                ExpressionEventType::Manual(reason)
            } else {
                ExpressionEventType::Automatic
            };

            self.record_expression_event(
                ref_id,
                "im",
                original_degree,
                original_max_value,
                new_degree,
                original_max_value,
                event_type,
                predicted_op_max,
            );

            self.set_expression(ref_id, new_expr);
            self.im_count += 1;
        }
    }

    /// Create Reset expression
    pub fn create_reset_expression(
        &mut self,
        ref_id: u64,
        reason: Option<String>,
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
            let event_type = if let Some(reason) = reason {
                ExpressionEventType::Manual(reason)
            } else {
                ExpressionEventType::Automatic
            };

            self.record_expression_event(
                ref_id,
                "reset",
                original_degree,
                original_max_value,
                new_degree,
                new_max_value,
                event_type,
                predicted_op_max,
            );

            self.set_expression(ref_id, new_expr);
            self.reset_count += 1;
            true
        } else {
            false
        }
    }

    /// Record an expression event (Im or Reset)
    fn record_expression_event(
        &mut self,
        ref_id: u64,
        operation_type: &str,
        original_degree: usize,
        original_max_value: u64,
        new_degree: usize,
        new_max_value: u64,
        event_type: ExpressionEventType,
        predicted_op_max: Option<u64>,
    ) {
        let event = ExpressionEvent {
            round: self.current_round,
            step: self.current_step.clone(),
            substep: self.current_substep.clone(),
            operation_type: operation_type.to_string(),
            ref_id,
            original_degree,
            original_max_value,
            new_degree,
            new_max_value,
            event_type,
            predicted_operation_max: predicted_op_max,
        };

        self.expression_events.push(event.clone());

        if let Some(round) = self.current_round {
            self.events_by_round.entry(round).or_insert_with(Vec::new).push(event);
        }
    }

    pub fn print_expression(&self, ref_id: u64) {
        if let Some(expr) = self.get_expression(ref_id) {
            println!("expr[{}] = {}", ref_id, expr);
        } else {
            println!("No expression found for ref {}", ref_id);
        }
    }

    /// Print summary of all expression events
    pub fn print_expression_summary(&self) {
        println!("\n--- Expression Event Summary ---");
        println!("Total events: {}", self.expression_events.len());
        println!("  Im events: {}", self.im_count);
        println!("  Reset events: {}", self.reset_count);
        println!("Maximum expression value: {}", self.max_value);

        // Group by type
        let mut automatic_im = 0;
        let mut manual_im = 0;
        let mut automatic_reset = 0;
        let mut manual_reset = 0;

        for event in &self.expression_events {
            match (&event.operation_type as &str, &event.event_type) {
                ("im", ExpressionEventType::Automatic) => automatic_im += 1,
                ("im", ExpressionEventType::Manual(_)) => manual_im += 1,
                ("reset", ExpressionEventType::Automatic) => automatic_reset += 1,
                ("reset", ExpressionEventType::Manual(_)) => manual_reset += 1,
                _ => {}
            }
        }

        println!("  Automatic Im: {}", automatic_im);
        println!("  Manual Im: {}", manual_im);
        println!("  Automatic Reset: {}", automatic_reset);
        println!("  Manual Reset: {}", manual_reset);
    }

    /// Print events for a specific round
    pub fn print_round_events(&self, round: usize, limit: Option<usize>) {
        let limit = limit.unwrap_or(usize::MAX);

        println!("\n--- Round {} Expression Events ---", round);
        if let Some(events) = self.events_by_round.get(&round) {
            println!("There were {} expression events in round {}", events.len(), round);
            for (i, event) in events.iter().enumerate().take(limit) {
                let reason = match &event.event_type {
                    ExpressionEventType::Automatic => "auto".to_string(),
                    ExpressionEventType::Manual(reason) => format!("manual: {}", reason),
                };

                // 1. Step: "θ", Substep: "Compute A'[x, y, z]", Op: Unknown, First max: 2477476, Second max: 0, Op max: 0
                println!(
                    "{}. Step: {}, Substep: {}, Type: {}, Degree: {}→{}, Max Value: {}→{} ({})",
                    i + 1,
                    event.step.clone().unwrap_or("N/A".to_string()),
                    event.substep.clone().unwrap_or("N/A".to_string()),
                    event.operation_type.to_uppercase(),
                    event.original_degree,
                    event.new_degree,
                    event.original_max_value,
                    event.new_max_value,
                    reason
                );

                if let Some(predicted) = event.predicted_operation_max {
                    println!("    (predicted operation max would have been: {})", predicted);
                }
            }
        } else {
            println!("No expression events in round {}", round);
        }
    }
}
