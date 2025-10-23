use std::fmt;

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

impl Expression {
    pub const ZERO: Expression = Expression::Constant(0);
    pub const ONE: Expression = Expression::Constant(1);

    pub fn input(ref_id: u64) -> Self {
        Expression::Input(ref_id)
    }

    pub fn constant(value: u8) -> Self {
        Expression::Constant(value & 1)
    }

    pub fn xor(exprs: Vec<Expression>) -> Self {
        Expression::Xor(exprs)
    }

    pub fn and(exprs: Vec<Expression>) -> Self {
        Expression::And(exprs)
    }

    pub fn not(expr: Expression) -> Self {
        Expression::Not(Box::new(expr))
    }

    pub fn nand(a: Expression, b: Expression) -> Self {
        Self::and(vec![Self::not(a), b])
    }

    pub fn proxy(id: u64, degree: usize, max_value: u64, expr: Expression) -> Self {
        Expression::Proxy {
            id,
            cached_degree: degree,
            cached_max_value: max_value,
            cached_expression: Box::new(expr),
        }
    }

    pub fn im(
        id: u64,
        degree: usize,
        max_value: u64,
        original_degree: usize,
        original_max_value: u64,
    ) -> Self {
        Expression::Im { id, degree, max_value, original_degree, original_max_value }
    }

    pub fn reset(id: u64, original_degree: usize, original_max_value: u64) -> Self {
        Expression::Reset { id, original_degree, original_max_value }
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
