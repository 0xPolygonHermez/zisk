use std::{collections::HashMap, fmt};

// Expression constants
const BLOWUP_FACTOR: usize = 2;
const MAX_DEGREE: usize = BLOWUP_FACTOR + 1;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Input reference
    Input(u64),

    /// Constant value (0 or 1)
    Constant(u8),

    /// XOR of multiple expressions
    Xor(Vec<Expression>),

    /// AND of multiple expressions
    And(Vec<Expression>),

    /// NOT of expression
    Not(Box<Expression>),

    /// Proxy expression that represents a complex expression with just its properties
    Proxy {
        id: u64,                            // Unique identifier for this proxy
        cached_degree: usize,               // Cached degree
        cached_max_value: u64,              // Cached max value
        cached_expression: Box<Expression>, // Cached original expression
    },

    /// Intermediate expression
    Im {
        id: u64,                 // Unique identifier for this reset
        degree: usize,           // Degree of the intermediate
        max_value: u64,          // Max value of the intermediate
        original_degree: usize,  // Original degree before Im
        original_max_value: u64, // Original max value before Im
    },

    /// Reset expression that acts like a fresh input with reset properties
    Reset {
        id: u64,                 // Unique identifier for this reset
        original_degree: usize,  // Original degree before reset
        original_max_value: u64, // Original max value before reset
    },
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
        self.counter += 1;
        self.counter
    }

    /// Set context for tracking
    pub fn set_context(&mut self, round: usize, step: &str) {
        self.current_round = Some(round);
        self.current_step = Some(step.to_string());
    }

    pub fn set_subcontext(&mut self, substep: &str) {
        self.current_substep = Some(substep.to_string());
    }

    /// Get the expression for a specific reference
    pub fn get_expression(&self, ref_id: u64) -> Option<&Expression> {
        self.expressions.get(&ref_id)
    }

    /// Set expression for a reference
    pub fn set_expression(&mut self, ref_id: u64, expr: Expression) {
        self.expressions.insert(ref_id, expr.simplify());
    }

    /// Create a proxy expression from an expression
    pub fn create_proxy_expression(&mut self, ref_id: u64) -> bool {
        if let Some(expr) = self.get_expression(ref_id).cloned() {
            let original_degree = expr.degree();
            let original_max_value = expr.max_value();

            let new_expr = Expression::Proxy {
                id: self.next_id(),
                cached_degree: original_degree,
                cached_max_value: original_max_value,
                cached_expression: Box::new(expr),
            };

            // Replace the expression with the proxy
            self.set_expression(ref_id, new_expr);
            self.proxy_count += 1;
            true
        } else {
            false
        }
    }

    /// Create Im expression (reduces degree by 2, keeps max value)
    pub fn create_im_expression(
        &mut self,
        ref_id: u64,
        reason: Option<String>,
        predicted_op_max: Option<u64>,
    ) -> bool {
        if let Some(expr) = self.get_expression(ref_id).cloned() {
            let original_degree = expr.degree();
            let original_max_value = expr.max_value();

            // Create new expression by factoring out complexity
            let new_expr = self.factor_expression(expr, original_degree, original_max_value, false);
            let new_degree = new_expr.degree();
            let new_max_value = new_expr.max_value();

            // Update global maximum
            if new_max_value > self.max_value {
                self.max_value = new_max_value;
            }

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
                new_max_value,
                event_type,
                predicted_op_max,
            );

            self.set_expression(ref_id, new_expr);
            self.im_count += 1;
            true
        } else {
            false
        }
    }

    /// Create Reset expression (acts like fresh input)
    pub fn create_reset_expression(
        &mut self,
        ref_id: u64,
        reason: Option<String>,
        predicted_op_max: Option<u64>,
    ) -> bool {
        if let Some(expr) = self.get_expression(ref_id).cloned() {
            let original_degree = expr.degree();
            let original_max_value = expr.max_value();

            // Create new expression by factoring
            let new_expr = self.factor_expression(expr, original_degree, original_max_value, true);
            let new_degree = new_expr.degree();
            let new_max_value = new_expr.max_value();

            // Update global maximum
            if new_max_value > self.max_value {
                self.max_value = new_max_value;
            }

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

    /// Factor an expression by creating Im terms or Reset terms
    fn factor_expression(
        &mut self,
        expr: Expression,
        original_degree: usize,
        original_max_value: u64,
        is_reset: bool,
    ) -> Expression {
        let mut current_expr = self.expand_expression(expr);
        let current_degree = current_expr.degree();

        // If degree is already <= MAX_DEGREE, create a single expression
        if current_degree <= MAX_DEGREE {
            return if is_reset {
                Expression::Reset { id: self.next_id(), original_degree, original_max_value }
            } else {
                Expression::Im {
                    id: self.next_id(),
                    degree: 1,
                    max_value: original_max_value,
                    original_degree,
                    original_max_value,
                }
            };
        }

        // Degree > MAX_DEGREE, need to factor iteratively
        while current_expr.degree() > MAX_DEGREE {
            current_expr = self.factor_expression_once(current_expr, is_reset);
        }

        current_expr
    }

    fn expand_expression(&mut self, expr: Expression) -> Expression {
        match expr {
            Expression::Proxy { cached_expression, .. } => {
                // Unwrap proxy and expand the underlying expression
                self.expand_expression(*cached_expression)
            }

            Expression::And(factors) => {
                // First, recursively expand all factors
                let expanded_factors: Vec<Expression> =
                    factors.into_iter().map(|f| self.expand_expression(f)).collect();

                // Flatten nested AND expressions
                let mut flat_factors = Vec::new();
                for factor in expanded_factors {
                    match factor {
                        Expression::And(inner) => flat_factors.extend(inner),
                        other => flat_factors.push(other),
                    }
                }

                // Factor any factors that still have degree > MAX_DEGREE
                let factored: Vec<Expression> = flat_factors
                    .into_iter()
                    .map(|f| {
                        if f.degree() > MAX_DEGREE {
                            let deg = f.degree();
                            let max_val = f.max_value();
                            // Recursively factor this sub-expression
                            self.factor_expression(f, deg, max_val, false)
                        } else {
                            f
                        }
                    })
                    .collect();

                if factored.is_empty() {
                    Expression::Constant(1)
                } else if factored.len() == 1 {
                    factored.into_iter().next().unwrap()
                } else {
                    Expression::And(factored)
                }
            }

            Expression::Xor(terms) => {
                // First, recursively expand all terms
                let expanded_terms: Vec<Expression> =
                    terms.into_iter().map(|t| self.expand_expression(t)).collect();

                // Flatten nested XOR expressions
                let mut flat_terms = Vec::new();
                for term in expanded_terms {
                    match term {
                        Expression::Xor(inner) => flat_terms.extend(inner),
                        other => flat_terms.push(other),
                    }
                }

                // Factor any terms that still have degree > MAX_DEGREE
                let factored: Vec<Expression> = flat_terms
                    .into_iter()
                    .map(|t| {
                        if t.degree() > MAX_DEGREE {
                            let deg = t.degree();
                            let max_val = t.max_value();
                            // Recursively factor this sub-expression
                            self.factor_expression(t, deg, max_val, false)
                        } else {
                            t
                        }
                    })
                    .collect();

                if factored.is_empty() {
                    Expression::Constant(0)
                } else if factored.len() == 1 {
                    factored.into_iter().next().unwrap()
                } else {
                    Expression::Xor(factored)
                }
            }

            Expression::Not(inner) => {
                let expanded = self.expand_expression(*inner);
                if expanded.degree() > MAX_DEGREE {
                    let deg = expanded.degree();
                    let max_val = expanded.max_value();
                    Expression::Not(Box::new(self.factor_expression(expanded, deg, max_val, false)))
                } else {
                    Expression::Not(Box::new(expanded))
                }
            }

            // Base cases - already expanded
            Expression::Input(_)
            | Expression::Constant(_)
            | Expression::Im { .. }
            | Expression::Reset { .. } => expr,
        }
    }

    /// Factor an expression once by creating one Im term
    fn factor_expression_once(&mut self, expr: Expression, is_reset: bool) -> Expression {
        match expr {
            Expression::Proxy { cached_expression, .. } => {
                self.factor_expression_once(*cached_expression, is_reset)
            }

            Expression::And(factors) if factors.len() > 1 => {
                // Verify all factors have degree <= MAX_DEGREE
                for factor in &factors {
                    if factor.degree() > MAX_DEGREE {
                        panic!(
                            "Cannot factor AND expression: single factor has degree {} > MAX_DEGREE {}",
                            factor.degree(),
                            MAX_DEGREE
                        );
                    }
                }

                let total_degree: usize = factors.iter().map(|f| f.degree()).sum();

                // Calculate target: we want remaining degree to be <= MAX_DEGREE
                // So we need to factor out at least (total_degree - MAX_DEGREE) degrees
                let target_remaining = if MAX_DEGREE > 0 { MAX_DEGREE } else { 0 };
                let min_degree_to_factor = total_degree.saturating_sub(target_remaining);

                // Sort factors: prioritize non-virtual (Im/Reset) factors first,
                // then by degree (higher first), then by max_value (higher first)
                let mut sorted_factors: Vec<_> = factors.into_iter().collect();
                sorted_factors.sort_by(|a, b| {
                    let a_virtual = Self::is_virtual_factor(a);
                    let b_virtual = Self::is_virtual_factor(b);

                    match a_virtual.cmp(&b_virtual) {
                        std::cmp::Ordering::Equal => {
                            // Both virtual or both non-virtual, sort by degree then max_value
                            match b.degree().cmp(&a.degree()) {
                                std::cmp::Ordering::Equal => b.max_value().cmp(&a.max_value()),
                                other => other,
                            }
                        }
                        other => other, // Non-virtual (false) comes before virtual (true)
                    }
                });

                let mut im_factors = Vec::new();
                let mut remaining_factors = Vec::new();
                let mut factored_degree = 0usize;
                let mut factored_max_value = 1u64;

                // Greedily select factors until we have enough degree
                for factor in sorted_factors {
                    if factored_degree < min_degree_to_factor {
                        factored_degree += factor.degree();
                        factored_max_value =
                            factored_max_value.saturating_mul(factor.max_value().max(1));
                        im_factors.push(factor);
                    } else {
                        remaining_factors.push(factor);
                    }
                }

                // Ensure we've factored enough
                if factored_degree < min_degree_to_factor && !remaining_factors.is_empty() {
                    let additional_factor = remaining_factors.remove(0);
                    factored_degree += additional_factor.degree();
                    factored_max_value =
                        factored_max_value.saturating_mul(additional_factor.max_value().max(1));
                    im_factors.push(additional_factor);
                }

                if im_factors.is_empty() {
                    return Expression::And(remaining_factors);
                }

                // Create Im/Reset for the factored part
                let factored_expr = if is_reset {
                    Expression::Reset {
                        id: self.next_id(),
                        original_degree: factored_degree,
                        original_max_value: factored_max_value,
                    }
                } else {
                    Expression::Im {
                        id: self.next_id(),
                        degree: 1,
                        max_value: factored_max_value,
                        original_degree: factored_degree,
                        original_max_value: factored_max_value,
                    }
                };

                if remaining_factors.is_empty() {
                    factored_expr
                } else {
                    let mut result_factors = vec![factored_expr];
                    result_factors.extend(remaining_factors);
                    Expression::And(result_factors)
                }
            }

            Expression::Xor(terms) if terms.len() > 1 => {
                // For XOR, we need to factor the most complex term
                let (max_idx, _) = terms
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, t)| (t.degree(), t.max_value()))
                    .unwrap();

                let most_complex_term = &terms[max_idx];
                let most_complex_degree = most_complex_term.degree();
                let most_complex_max = most_complex_term.max_value();

                let mut new_terms = terms;
                new_terms[max_idx] = if is_reset {
                    Expression::Reset {
                        id: self.next_id(),
                        original_degree: most_complex_degree,
                        original_max_value: most_complex_max,
                    }
                } else {
                    Expression::Im {
                        id: self.next_id(),
                        degree: 1,
                        max_value: most_complex_max,
                        original_degree: most_complex_degree,
                        original_max_value: most_complex_max,
                    }
                };

                Expression::Xor(new_terms)
            }

            _ => {
                panic!(
                    "Cannot factor expression: single term has degree {} > MAX_DEGREE {}. Type: {:?}",
                    expr.degree(),
                    MAX_DEGREE,
                    std::mem::discriminant(&expr)
                );
            }
        }
    }

    fn is_virtual_factor(expr: &Expression) -> bool {
        matches!(expr, Expression::Im { .. } | Expression::Reset { .. })
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

    /// Get maximum value encountered
    pub fn get_max_value(&self) -> u64 {
        self.max_value
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

    pub fn get_im_events(&self) -> Vec<&ExpressionEvent> {
        self.expression_events.iter().filter(|e| e.operation_type == "im").collect()
    }

    pub fn get_reset_events(&self) -> Vec<&ExpressionEvent> {
        self.expression_events.iter().filter(|e| e.operation_type == "reset").collect()
    }
}

impl Expression {
    pub const ZERO: Expression = Expression::Constant(0);
    pub const ONE: Expression = Expression::Constant(1);

    pub fn input(ref_id: u64) -> Self {
        Expression::Input(ref_id)
    }

    pub fn constant(value: u8) -> Self {
        Expression::Constant(value & 1)
    }

    pub fn xor(a: Expression, b: Expression) -> Self {
        Expression::Xor(vec![a, b])
    }

    pub fn and(a: Expression, b: Expression) -> Self {
        Expression::And(vec![a, b])
    }

    pub fn not(expr: Expression) -> Self {
        Expression::Not(Box::new(expr))
    }

    pub fn nand(a: Expression, b: Expression) -> Self {
        Self::and(Self::not(a), b)
    }

    fn flatten_xor(&self) -> Vec<&Expression> {
        match self {
            Expression::Xor(exprs) => {
                let mut flattened = Vec::new();
                for expr in exprs {
                    flattened.extend(expr.flatten_xor());
                }
                flattened
            }
            other => vec![other],
        }
    }

    /// Computes the maximum possible value of this expression
    /// XOR as +, AND as *, NOT as +1
    pub fn max_value(&self) -> u64 {
        match self {
            Expression::Input(_) | Expression::Reset { .. } => 1, // Input can be 0 or 1, maximum is 1
            Expression::Constant(val) => *val as u64,
            Expression::Xor(exprs) => {
                // XOR as addition: sum of maximums
                exprs.iter().map(|e| e.max_value()).sum()
            }
            Expression::And(exprs) => {
                // AND as multiplication: product of maximums
                exprs.iter().map(|e| e.max_value()).product()
            }
            Expression::Not(expr) => {
                // NOT as +1
                1 + expr.max_value()
            }
            Expression::Proxy { cached_max_value, .. } => *cached_max_value,
            Expression::Im { max_value, .. } => *max_value,
        }
    }

    /// Computes the algebraic degree of this expression
    pub fn degree(&self) -> usize {
        match self {
            Expression::Input(_) | Expression::Reset { .. } => 1,
            Expression::Constant(_) => 0,
            Expression::Xor(exprs) => {
                // XOR is linear - take maximum degree of operands
                exprs.iter().map(|e| e.degree()).max().unwrap_or(0)
            }
            Expression::Not(expr) => {
                // NOT doesn't change degree
                expr.degree()
            }
            Expression::And(exprs) => {
                // AND multiplies terms - sum the degrees
                exprs.iter().map(|e| e.degree()).sum()
            }
            Expression::Proxy { cached_degree, .. } => *cached_degree,
            Expression::Im { degree, .. } => *degree,
        }
    }

    /// Simplify the expression
    pub fn simplify(self) -> Self {
        match self {
            Expression::Xor(mut exprs) => {
                exprs = exprs.into_iter().map(|e| e.simplify()).collect();
                // Remove duplicates (A ⊕ A = 0)
                let mut simplified = Vec::new();
                for expr in exprs {
                    if simplified.contains(&expr) {
                        simplified.retain(|e| e != &expr);
                    } else {
                        simplified.push(expr);
                    }
                }
                if simplified.is_empty() {
                    Expression::Constant(0)
                } else if simplified.len() == 1 {
                    simplified.into_iter().next().unwrap()
                } else {
                    Expression::Xor(simplified)
                }
            }
            Expression::And(exprs) => {
                let exprs: Vec<_> = exprs.into_iter().map(|e| e.simplify()).collect();
                // Check for constants
                if exprs.iter().any(|e| matches!(e, Expression::Constant(0))) {
                    Expression::Constant(0)
                } else {
                    let non_ones: Vec<_> = exprs
                        .into_iter()
                        .filter(|e| !matches!(e, Expression::Constant(1)))
                        .collect();
                    if non_ones.is_empty() {
                        Expression::Constant(1)
                    } else if non_ones.len() == 1 {
                        non_ones.into_iter().next().unwrap()
                    } else {
                        Expression::And(non_ones)
                    }
                }
            }
            Expression::Not(expr) => match expr.simplify() {
                Expression::Constant(0) => Expression::Constant(1),
                Expression::Constant(1) => Expression::Constant(0),
                Expression::Not(inner) => *inner,
                simplified => Expression::Not(Box::new(simplified)),
            },
            other => other,
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Input(ref_id) => write!(f, "in[{}]", ref_id),
            Expression::Constant(value) => write!(f, "{}", value),
            Expression::Xor(_) => {
                // Flatten all XOR operations into a single chain
                let flattened = self.flatten_xor();
                if flattened.len() == 1 {
                    write!(f, "{}", flattened[0])
                } else {
                    write!(f, "(")?;
                    for (i, expr) in flattened.iter().enumerate() {
                        if i > 0 {
                            write!(f, " + ")?;
                        }
                        write!(f, "{}", expr)?;
                    }
                    write!(f, ")")
                }
            }
            Expression::And(exprs) => {
                if exprs.len() == 1 {
                    write!(f, "{}", exprs[0])
                } else {
                    write!(f, "(")?;
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            write!(f, " * ")?;
                        }
                        write!(f, "{}", expr)?;
                    }
                    write!(f, ")")
                }
            }
            Expression::Not(expr) => write!(f, "1 + {}", expr),
            Expression::Proxy { id, .. } => write!(f, "P[{}]", id),
            Expression::Im { id, .. } => write!(f, "Im[{}]", id),
            Expression::Reset { id, .. } => write!(f, "R[{}]", id),
        }
    }
}
