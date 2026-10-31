//! Turning the expression DAG back into readable expressions.
//!
//! Both backends face the same problem. The pilout's expression pool is a DAG
//! with heavy sharing, and it is flat: every operation is its own node, so
//! `a - (b + c * d)` arrives as four nodes. Printing one definition per node is
//! faithful but unreadable; inlining everything duplicates shared subterms and,
//! for the wider AIRs, explodes. So a node is given a name when it is shared
//! (or the PIL already named it) and inlined into its user otherwise.

use std::collections::HashMap;

use crate::ir::{Ir, Node, Operand};

/// Inlining budget: a node whose inlined form would cover more than this many
/// nodes gets a name instead. Keeps generated definitions readable and bounds
/// the recursion depth of [`render`] on the wide AIRs.
pub const MAX_INLINE: u32 = 40;

/// Which expressions get their own definition, and in what order to emit them.
pub struct Plan {
    /// Reachable expression indices, dependencies before dependents.
    pub order: Vec<u32>,
    /// Indexed by expression index: does this node get its own definition?
    pub defined: Vec<bool>,
    /// The names the PIL gave to intermediates, by expression index.
    pub named: HashMap<u32, String>,
}

impl Plan {
    pub fn build(ir: &Ir, max_inline: u32) -> Plan {
        let n = ir.expressions.len();
        let named: HashMap<u32, String> =
            ir.columns.intermediates.iter().map(|i| (i.expr, i.name.clone())).collect();
        let uses = ir.use_counts();
        let order = post_order(ir);

        let mut defined = vec![false; n];
        let mut size = vec![1u32; n];
        // Post-order, so a node's children are already sized when it is reached.
        for &idx in &order {
            let i = idx as usize;
            let mut total = 1;
            for operand in ir.expressions[i].operands() {
                if let Operand::Expr { idx: child, .. } = operand {
                    if !defined[*child as usize] {
                        total += size[*child as usize];
                    }
                }
            }
            size[i] = total;
            defined[i] = named.contains_key(&idx) || uses[i] > 1 || total > max_inline;
        }
        Plan { order, defined, named }
    }

    pub fn is_defined(&self, idx: u32) -> bool {
        self.defined.get(idx as usize).copied().unwrap_or(false)
    }

    /// The definitions to emit, dependencies first.
    pub fn definitions(&self) -> impl Iterator<Item = u32> + '_ {
        self.order.iter().copied().filter(|idx| self.is_defined(*idx))
    }
}

/// Reachable expressions with every dependency before its dependents.
///
/// Iterative on purpose: expression trees in the wide AIRs are deep enough that
/// proofman raises its own stack limits to walk them.
fn post_order(ir: &Ir) -> Vec<u32> {
    #[derive(Clone, Copy)]
    enum Step {
        Enter(u32),
        Exit(u32),
    }

    let n = ir.expressions.len();
    let mut state = vec![0u8; n]; // 0 = unseen, 1 = entered, 2 = emitted
    let mut order = Vec::new();
    let mut stack: Vec<Step> = ir.constraints.iter().rev().map(|c| Step::Enter(c.expr)).collect();

    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(idx) => {
                let i = idx as usize;
                if i >= n || state[i] != 0 {
                    continue;
                }
                state[i] = 1;
                stack.push(Step::Exit(idx));
                for operand in ir.expressions[i].operands() {
                    if let Operand::Expr { idx, .. } = operand {
                        stack.push(Step::Enter(*idx));
                    }
                }
            }
            Step::Exit(idx) => {
                let i = idx as usize;
                if state[i] == 1 {
                    state[i] = 2;
                    order.push(idx);
                }
            }
        }
    }
    order
}

/// How a backend renders the things at the leaves of an expression.
pub trait Leaves {
    /// A base field constant, as a decimal string.
    fn constant(&self, value: &str) -> String;
    /// A column, at a row offset relative to the current row.
    fn column(&self, name: &str, offset: i32) -> String;
    /// An air value, challenge, public, proof value or airgroup value: the same
    /// in every row.
    fn global(&self, name: &str) -> String;
    /// A reference to an expression that has its own definition.
    fn reference(&self, idx: u32) -> String;
}

const PREC_SUM: u8 = 1;
const PREC_PROD: u8 = 2;
const PREC_NEG: u8 = 3;
const PREC_ATOM: u8 = 4;

/// Render expression `idx`, inlining every node the plan did not name.
pub fn render(ir: &Ir, plan: &Plan, leaves: &dyn Leaves, idx: u32) -> String {
    node(ir, plan, leaves, idx, 0).0
}

fn node(ir: &Ir, plan: &Plan, leaves: &dyn Leaves, idx: u32, min_prec: u8) -> (String, u8) {
    let Some(expr) = ir.expressions.get(idx as usize) else {
        return (format!("<missing expression {idx}>"), PREC_ATOM);
    };
    let (text, prec) = match expr {
        Node::Add { lhs, rhs } => (binary(ir, plan, leaves, lhs, "+", rhs, PREC_SUM), PREC_SUM),
        Node::Sub { lhs, rhs } => (binary(ir, plan, leaves, lhs, "-", rhs, PREC_SUM), PREC_SUM),
        Node::Mul { lhs, rhs } => (binary(ir, plan, leaves, lhs, "*", rhs, PREC_PROD), PREC_PROD),
        Node::Neg { value } => {
            let (inner, _) = operand(ir, plan, leaves, value, PREC_NEG);
            (format!("-{inner}"), PREC_NEG)
        }
    };
    (paren(text, prec, min_prec), prec)
}

fn binary(
    ir: &Ir,
    plan: &Plan,
    leaves: &dyn Leaves,
    lhs: &Operand,
    op: &str,
    rhs: &Operand,
    prec: u8,
) -> String {
    // Left operand at the operator's own precedence, right operand one above:
    // `a - b - c` needs no parentheses, `a - (b - c)` does.
    let (lhs, _) = operand(ir, plan, leaves, lhs, prec);
    let (rhs, _) = operand(ir, plan, leaves, rhs, prec + 1);
    format!("{lhs} {op} {rhs}")
}

fn operand(
    ir: &Ir,
    plan: &Plan,
    leaves: &dyn Leaves,
    operand: &Operand,
    min_prec: u8,
) -> (String, u8) {
    match operand {
        Operand::Const { value } => (leaves.constant(value), PREC_ATOM),
        Operand::Witness { name, offset, .. } => (leaves.column(name, *offset), PREC_ATOM),
        Operand::Fixed { name, offset, .. } => (leaves.column(name, *offset), PREC_ATOM),
        Operand::Periodic { name, offset, .. } => (leaves.column(name, *offset), PREC_ATOM),
        Operand::Custom { name, offset, .. } => (leaves.column(name, *offset), PREC_ATOM),
        Operand::AirValue { name, .. }
        | Operand::AirGroupValue { name, .. }
        | Operand::Public { name, .. }
        | Operand::ProofValue { name, .. }
        | Operand::Challenge { name, .. } => (leaves.global(name), PREC_ATOM),
        Operand::Expr { idx, .. } => {
            if plan.is_defined(*idx) {
                (leaves.reference(*idx), PREC_ATOM)
            } else {
                node(ir, plan, leaves, *idx, min_prec)
            }
        }
    }
}

fn paren(text: String, prec: u8, min_prec: u8) -> String {
    if prec < min_prec {
        format!("({text})")
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::tests::sample;

    struct Plain;

    impl Leaves for Plain {
        fn constant(&self, value: &str) -> String {
            value.to_string()
        }
        fn column(&self, name: &str, offset: i32) -> String {
            if offset == 0 {
                name.to_string()
            } else {
                format!("{name}@{offset:+}")
            }
        }
        fn global(&self, name: &str) -> String {
            name.to_string()
        }
        fn reference(&self, idx: u32) -> String {
            format!("e{idx}")
        }
    }

    #[test]
    fn shared_and_named_nodes_get_definitions_and_the_rest_inline() {
        let ir = sample();
        let plan = Plan::build(&ir, MAX_INLINE);

        assert!(plan.is_defined(0), "used twice");
        assert!(!plan.is_defined(1), "used once");
        assert!(plan.is_defined(2), "named by the PIL");
        assert!(!plan.is_defined(3), "used once, by a constraint");

        // Dependencies come first, and the unreachable node is absent.
        let defs: Vec<u32> = plan.definitions().collect();
        assert_eq!(defs, vec![0, 2]);
        assert!(plan.order.iter().position(|i| *i == 0) < plan.order.iter().position(|i| *i == 1));
        assert!(!plan.order.contains(&4));
    }

    #[test]
    fn subtraction_on_the_right_keeps_its_parentheses() {
        let ir = sample();
        let plan = Plan::build(&ir, MAX_INLINE);
        // Expression 2 is `a - (b + e0)`: the sum must stay bracketed, or it
        // would read as `a - b + e0`.
        assert_eq!(render(&ir, &plan, &Plain, 2), "a - (b + e0)");
        // Expression 3 is `a * e0`: a product of atoms needs none.
        assert_eq!(render(&ir, &plan, &Plain, 3), "a * e0");
    }

    #[test]
    fn a_node_over_the_inline_budget_is_given_a_name() {
        let ir = sample();
        // A budget of zero: every reachable node is over it, so nothing inlines.
        // Note that `b + e0` counts as one node, not two — `e0` is shared and
        // therefore already a definition of its own.
        let plan = Plan::build(&ir, 0);
        assert!(plan.is_defined(1));
        assert_eq!(render(&ir, &plan, &Plain, 2), "a - e1");
    }
}
