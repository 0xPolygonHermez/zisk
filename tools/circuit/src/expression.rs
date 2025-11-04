use std::fmt;

// Expression constants
const BLOWUP_FACTOR: usize = 2;
const MAX_DEGREE: usize = BLOWUP_FACTOR + 1;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Input reference
    Input { id: usize, name: Option<String>, round: Option<usize> },

    /// Constant value (0 or 1)
    Constant(u8),

    /// Proxy expression that represents a complex expression with just its properties
    Proxy {
        id: usize,
        original_degree: usize,
        original_max_value: u64,
        original_expression: Box<Expression>,
    },

    /// Intermediate expression that resets degree but keeps max value
    Im { id: usize, degree: usize, max_value: u64, name: Option<String>, round: Option<usize> },

    /// Reset expression that resets degree and max value
    Reset { id: usize, degree: usize, max_value: u64, name: Option<String>, round: Option<usize> },

    /// XOR of multiple expressions
    Xor(Vec<Expression>),

    /// AND of multiple expressions
    And(Vec<Expression>),

    /// NOT of expression
    Not(Box<Expression>),
}

pub enum ExpressionOp {
    Xor,
    And,
    Not,
    Nand,
}

impl Expression {
    pub const ZERO: Expression = Expression::Constant(0);
    pub const ONE: Expression = Expression::Constant(1);

    pub fn input(ref_id: usize, name: Option<String>, round: Option<usize>) -> Self {
        Expression::Input { id: ref_id, name, round }
    }

    pub fn constant(value: u8) -> Self {
        assert!(value == 0 || value == 1, "Constant value must be 0 or 1");
        Expression::Constant(value)
    }

    pub fn proxy(id: usize, degree: usize, max_value: u64, expr: Expression) -> Self {
        Expression::Proxy {
            id,
            original_degree: degree,
            original_max_value: max_value,
            original_expression: Box::new(expr),
        }
    }

    pub fn im(
        id: usize,
        original_degree: usize,
        original_max_value: u64,
        name: Option<String>,
        round: Option<usize>,
    ) -> Self {
        let degree =
            if original_degree >= MAX_DEGREE { original_degree - BLOWUP_FACTOR } else { 1 };
        Expression::Im { id, degree, max_value: original_max_value, name, round }
    }

    pub fn reset(
        id: usize,
        original_degree: usize,
        name: Option<String>,
        round: Option<usize>,
    ) -> Self {
        let degree =
            if original_degree >= MAX_DEGREE { original_degree - BLOWUP_FACTOR } else { 1 };
        Expression::Reset { id, degree, max_value: 1, name, round }
    }

    pub fn op(op: &ExpressionOp, expr1: Expression, expr2: Expression) -> Self {
        match op {
            ExpressionOp::Xor => Expression::Xor(vec![expr1, expr2]),
            ExpressionOp::And => Expression::And(vec![expr1, expr2]),
            ExpressionOp::Not => Expression::Not(Box::new(expr1)),
            ExpressionOp::Nand => Expression::And(vec![Expression::Not(Box::new(expr1)), expr2]),
        }
    }

    // pub fn xor(exprs: Vec<Expression>) -> Self {
    //     Expression::Xor(exprs)
    // }

    // pub fn and(exprs: Vec<Expression>) -> Self {
    //     Expression::And(exprs)
    // }

    // pub fn not(expr: Expression) -> Self {
    //     Expression::Not(Box::new(expr))
    // }

    // pub fn nand(a: Expression, b: Expression) -> Self {
    //     Self::and(vec![Self::not(a), b])
    // }

    /// Computes the maximum possible value of this expression
    /// XOR as +, AND as *, NOT as +1
    pub fn max_value(&self) -> u64 {
        match self {
            Expression::Input { .. } => 1, // Input can be 0 or 1, maximum is 1
            Expression::Constant(val) => *val as u64,
            Expression::Proxy { original_max_value, .. } => *original_max_value,
            Expression::Im { max_value, .. } => *max_value,
            Expression::Reset { max_value, .. } => *max_value,
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
        }
    }

    /// Computes the algebraic degree of this expression
    pub fn degree(&self) -> usize {
        match self {
            Expression::Input { .. } => 1,
            Expression::Constant(_) => 0,
            Expression::Proxy { original_degree, .. } => *original_degree,
            Expression::Im { degree, .. } => *degree,
            Expression::Reset { degree, .. } => *degree,
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
                    Self::ZERO
                } else if simplified.len() == 1 {
                    simplified.into_iter().next().unwrap()
                } else {
                    Expression::Xor(simplified)
                }
            }
            Expression::And(exprs) => {
                let exprs: Vec<_> = exprs.into_iter().map(|e| e.simplify()).collect();
                // Check for constants
                if exprs.iter().any(|e| matches!(e, &Self::ZERO)) {
                    Self::ZERO
                } else {
                    let non_ones: Vec<_> =
                        exprs.into_iter().filter(|e| !matches!(e, &Self::ONE)).collect();
                    if non_ones.is_empty() {
                        Self::ONE
                    } else if non_ones.len() == 1 {
                        non_ones.into_iter().next().unwrap()
                    } else {
                        Expression::And(non_ones)
                    }
                }
            }
            Expression::Not(expr) => match expr.simplify() {
                Self::ZERO => Self::ONE,
                Self::ONE => Self::ZERO,
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
            Expression::Input { id, name, round } => match (name, round) {
                (Some(n), Some(r)) => write!(f, "{}[{}][{}]", n, r, id),
                (Some(n), None) => write!(f, "{}[{}]", n, id),
                (None, Some(r)) => write!(f, "in[{}][{}]", r, id),
                (None, None) => write!(f, "in[{}]", id),
            },
            Expression::Constant(value) => write!(f, "{}", value),
            Expression::Xor(_) => {
                // Flatten all XOR operations into a single chain
                let flattened = flatten_xor(self);
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
            Expression::Not(expr) => write!(f, "(1 + {})", expr),
            Expression::Proxy { id, .. } => write!(f, "P[{}]", id),
            Expression::Im { id, name, round, .. } => match (name, round) {
                (Some(n), Some(r)) => write!(f, "{}[{}][{}]", n, r, id),
                (Some(n), None) => write!(f, "{}[{}]", n, id),
                (None, Some(r)) => write!(f, "IM[{}][{}]", r, id),
                (None, None) => write!(f, "IM[{}]", id),
            },
            Expression::Reset { id, name, round, .. } => match (name, round) {
                (Some(n), Some(r)) => write!(f, "{}[{}][{}]", n, r, id),
                (Some(n), None) => write!(f, "{}[{}]", n, id),
                (None, Some(r)) => write!(f, "R[{}][{}]", r, id),
                (None, None) => write!(f, "R[{}]", id),
            },
        }
    }
}

fn flatten_xor(expr: &Expression) -> Vec<&Expression> {
    match expr {
        Expression::Xor(exprs) => {
            let mut flattened = Vec::new();
            for e in exprs {
                flattened.extend(flatten_xor(e));
            }
            flattened
        }
        other => vec![other],
    }
}

// /// Factor an expression by creating Im terms or Reset terms
// fn factor_expression(
//     &mut self,
//     expr: Expression,
//     original_degree: usize,
//     original_max_value: u64,
//     is_reset: bool,
// ) -> Expression {
//     let mut current_expr = self.expand_expression(expr);
//     let current_degree = current_expr.degree();

//     // If degree is already <= MAX_DEGREE, create a single expression
//     if current_degree <= MAX_DEGREE {
//         return if is_reset {
//             Expression::reset(self.next_id(), original_degree, original_max_value)
//         } else {
//             Expression::im(
//                 self.next_id(),
//                 1,
//                 original_max_value,
//                 original_degree,
//                 original_max_value,
//             )
//         };
//     }

//     // Degree > MAX_DEGREE, need to factor iteratively
//     while current_expr.degree() > MAX_DEGREE {
//         current_expr = self.factor_expression_once(current_expr, is_reset);
//     }

//     current_expr
// }

// fn expand_expression(&mut self, expr: Expression) -> Expression {
//     match expr {
//         Expression::Proxy { cached_expression, .. } => {
//             // Unwrap proxy and expand the underlying expression
//             self.expand_expression(*cached_expression)
//         }

//         Expression::And(factors) => {
//             // First, recursively expand all factors
//             let expanded_factors: Vec<Expression> =
//                 factors.into_iter().map(|f| self.expand_expression(f)).collect();

//             // Flatten nested AND expressions
//             let mut flat_factors = Vec::new();
//             for factor in expanded_factors {
//                 match factor {
//                     Expression::And(inner) => flat_factors.extend(inner),
//                     other => flat_factors.push(other),
//                 }
//             }

//             // Factor any factors that still have degree > MAX_DEGREE
//             let factored: Vec<Expression> = flat_factors
//                 .into_iter()
//                 .map(|f| {
//                     if f.degree() > MAX_DEGREE {
//                         let deg = f.degree();
//                         let max_val = f.max_value();
//                         // Recursively factor this sub-expression
//                         self.factor_expression(f, deg, max_val, false)
//                     } else {
//                         f
//                     }
//                 })
//                 .collect();

//             if factored.is_empty() {
//                 Expression::ONE
//             } else if factored.len() == 1 {
//                 factored.into_iter().next().unwrap()
//             } else {
//                 Expression::and(factored)
//             }
//         }

//         Expression::Xor(terms) => {
//             // First, recursively expand all terms
//             let expanded_terms: Vec<Expression> =
//                 terms.into_iter().map(|t| self.expand_expression(t)).collect();

//             // Flatten nested XOR expressions
//             let mut flat_terms = Vec::new();
//             for term in expanded_terms {
//                 match term {
//                     Expression::Xor(inner) => flat_terms.extend(inner),
//                     other => flat_terms.push(other),
//                 }
//             }

//             // Factor any terms that still have degree > MAX_DEGREE
//             let factored: Vec<Expression> = flat_terms
//                 .into_iter()
//                 .map(|t| {
//                     if t.degree() > MAX_DEGREE {
//                         let deg = t.degree();
//                         let max_val = t.max_value();
//                         // Recursively factor this sub-expression
//                         self.factor_expression(t, deg, max_val, false)
//                     } else {
//                         t
//                     }
//                 })
//                 .collect();

//             if factored.is_empty() {
//                 Expression::ZERO
//             } else if factored.len() == 1 {
//                 factored.into_iter().next().unwrap()
//             } else {
//                 Expression::xor(factored)
//             }
//         }

//         Expression::Not(inner) => {
//             let expanded = self.expand_expression(*inner);
//             if expanded.degree() > MAX_DEGREE {
//                 let deg = expanded.degree();
//                 let max_val = expanded.max_value();
//                 Expression::not(self.factor_expression(expanded, deg, max_val, false))
//             } else {
//                 Expression::not(expanded)
//             }
//         }

//         // Base cases - already expanded
//         Expression::Input(_)
//         | Expression::Constant(_)
//         | Expression::Im { .. }
//         | Expression::Reset { .. } => expr,
//     }
// }

// /// Factor an expression once by creating one Im term
// fn factor_expression_once(&mut self, expr: Expression, is_reset: bool) -> Expression {
//     match expr {
//         Expression::Proxy { cached_expression, .. } => {
//             self.factor_expression_once(*cached_expression, is_reset)
//         }

//         Expression::And(factors) if factors.len() > 1 => {
//             // Verify all factors have degree <= MAX_DEGREE
//             for factor in &factors {
//                 if factor.degree() > MAX_DEGREE {
//                     panic!(
//                         "Cannot factor AND expression: single factor has degree {} > MAX_DEGREE {}",
//                         factor.degree(),
//                         MAX_DEGREE
//                     );
//                 }
//             }

//             let total_degree: usize = factors.iter().map(|f| f.degree()).sum();

//             // Calculate target: we want remaining degree to be <= MAX_DEGREE
//             // So we need to factor out at least (total_degree - MAX_DEGREE) degrees
//             let target_remaining = if MAX_DEGREE > 0 { MAX_DEGREE } else { 0 };
//             let min_degree_to_factor = total_degree.saturating_sub(target_remaining);

//             // Sort factors: prioritize non-virtual (Im/Reset) factors first,
//             // then by degree (higher first), then by max_value (higher first)
//             let mut sorted_factors: Vec<_> = factors.into_iter().collect();
//             sorted_factors.sort_by(|a, b| {
//                 let a_virtual = Self::is_virtual_factor(a);
//                 let b_virtual = Self::is_virtual_factor(b);

//                 match a_virtual.cmp(&b_virtual) {
//                     std::cmp::Ordering::Equal => {
//                         // Both virtual or both non-virtual, sort by degree then max_value
//                         match b.degree().cmp(&a.degree()) {
//                             std::cmp::Ordering::Equal => b.max_value().cmp(&a.max_value()),
//                             other => other,
//                         }
//                     }
//                     other => other, // Non-virtual (false) comes before virtual (true)
//                 }
//             });

//             let mut im_factors = Vec::new();
//             let mut remaining_factors = Vec::new();
//             let mut factored_degree = 0usize;
//             let mut factored_max_value = 1u64;

//             // Greedily select factors until we have enough degree
//             for factor in sorted_factors {
//                 if factored_degree < min_degree_to_factor {
//                     factored_degree += factor.degree();
//                     factored_max_value =
//                         factored_max_value.saturating_mul(factor.max_value().max(1));
//                     im_factors.push(factor);
//                 } else {
//                     remaining_factors.push(factor);
//                 }
//             }

//             // Ensure we've factored enough
//             if factored_degree < min_degree_to_factor && !remaining_factors.is_empty() {
//                 let additional_factor = remaining_factors.remove(0);
//                 factored_degree += additional_factor.degree();
//                 factored_max_value =
//                     factored_max_value.saturating_mul(additional_factor.max_value().max(1));
//                 im_factors.push(additional_factor);
//             }

//             if im_factors.is_empty() {
//                 return Expression::and(remaining_factors);
//             }

//             // Create Im/Reset for the factored part
//             let factored_expr = if is_reset {
//                 Expression::reset(self.next_id(), factored_degree, factored_max_value)
//             } else {
//                 Expression::im(
//                     self.next_id(),
//                     1,
//                     factored_max_value,
//                     factored_degree,
//                     factored_max_value,
//                 )
//             };

//             if remaining_factors.is_empty() {
//                 factored_expr
//             } else {
//                 let mut result_factors = vec![factored_expr];
//                 result_factors.extend(remaining_factors);
//                 Expression::and(result_factors)
//             }
//         }

//         Expression::Xor(terms) if terms.len() > 1 => {
//             // For XOR, we need to factor the most complex term
//             let (max_idx, _) = terms
//                 .iter()
//                 .enumerate()
//                 .max_by_key(|(_, t)| (t.degree(), t.max_value()))
//                 .unwrap();

//             let most_complex_term = &terms[max_idx];
//             let most_complex_degree = most_complex_term.degree();
//             let most_complex_max = most_complex_term.max_value();

//             let mut new_terms = terms;
//             new_terms[max_idx] = if is_reset {
//                 Expression::reset(self.next_id(), most_complex_degree, most_complex_max)
//             } else {
//                 Expression::im(
//                     self.next_id(),
//                     1,
//                     most_complex_max,
//                     most_complex_degree,
//                     most_complex_max,
//                 )
//             };

//             Expression::xor(new_terms)
//         }

//         _ => {
//             panic!(
//                 "Cannot factor expression: single term has degree {} > MAX_DEGREE {}. Type: {:?}",
//                 expr.degree(),
//                 MAX_DEGREE,
//                 std::mem::discriminant(&expr)
//             );
//         }
//     }
// }

// fn is_virtual_factor(expr: &Expression) -> bool {
//     matches!(expr, Expression::Im { .. } | Expression::Reset { .. })
// }
