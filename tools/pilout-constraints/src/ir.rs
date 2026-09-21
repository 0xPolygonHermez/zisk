//! The IR the extractor writes and the backends read.
//!
//! It is a flattened, name-resolved view of one AIR of a `.pilout`: the
//! constraint list, the expression pool the constraints index into, and the
//! symbol tables needed to turn operand indices back into PIL names.
//!
//! Two properties are deliberate:
//!
//! - **Expression indices are the pilout's own.** `expressions[i]` is
//!   `air.expressions[i]` of the protobuf, including nodes no constraint
//!   reaches (they belong to hints or to the global constraint). This keeps the
//!   IR diffable against the pilout; use [`Ir::reachable`] to get the subset
//!   the constraints actually use.
//! - **The expression pool stays a DAG.** Expressions reference each other by
//!   index rather than being inlined, because the sharing is heavy: Main is
//!   146 constraints over 1778 reachable nodes.

use serde::{Deserialize, Serialize};

/// Bumped when a change to these types is not backward compatible.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct Ir {
    pub schema: u32,
    pub pilout: PiloutMeta,
    pub air: AirMeta,
    pub columns: Columns,
    pub globals: Globals,
    pub expressions: Vec<Node>,
    pub constraints: Vec<Constraint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PiloutMeta {
    pub name: String,
    /// Basename of the `.pilout` this was extracted from.
    pub file: String,
    /// blake3 of the `.pilout` file — the same digest `pil-helpers` stamps into
    /// `pil/src/pil_helpers/traces.rs` as `PILOUT_HASH`, so an IR generated
    /// from a different compile is recognizable as such.
    pub blake3: String,
    /// Base field characteristic, in decimal (Goldilocks for ZisK).
    pub base_field: String,
    pub num_stages: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AirMeta {
    pub airgroup: String,
    pub airgroup_id: u32,
    pub air: String,
    pub air_id: u32,
    /// Row count. Despite the comment in `pilout.proto`, the field holds `n`,
    /// not `log2(n)`.
    pub num_rows: u64,
    /// Committed widths per stage, stage 1 first (stage 0 is the fixed columns).
    pub stage_widths: Vec<u32>,
    pub aggregable: bool,
}

/// Everything that varies per row.
#[derive(Debug, Serialize, Deserialize)]
pub struct Columns {
    pub witness: Vec<Slot>,
    pub fixed: Vec<Slot>,
    pub periodic: Vec<Slot>,
    pub custom: Vec<Slot>,
    /// Expressions the PIL gave a name to (`IM_COL` symbols). Names are *not*
    /// unique — the compiler can emit two intermediates called `previous_c` —
    /// so `expr` is the key. Backends that need unique identifiers disambiguate
    /// with it.
    pub intermediates: Vec<Intermediate>,
}

/// Everything that is constant down the column: air values, challenges,
/// publics, proof values, airgroup values.
#[derive(Debug, Serialize, Deserialize)]
pub struct Globals {
    pub air_values: Vec<Slot>,
    pub airgroup_values: Vec<Slot>,
    pub publics: Vec<Slot>,
    pub proof_values: Vec<Slot>,
    pub challenges: Vec<Slot>,
}

/// One scalar slot. A PIL array (`col witness a[2]`) contributes one slot per
/// element, which is how the pilout numbers them: consecutive ids, row-major.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    /// Rendered name, e.g. `a[0]` or `Main.last_reg_value[3][1]`.
    pub name: String,
    /// The symbol name without the subscripts.
    pub base: String,
    /// Empty for a scalar.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub index: Vec<u32>,
    /// 0 for fixed and periodic columns; the commit stage otherwise.
    pub stage: u32,
    /// Index within `(stage, kind)`, as operands reference it.
    pub id: u32,
    /// Set for custom-commit columns only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intermediate {
    pub expr: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Node {
    Add { lhs: Operand, rhs: Operand },
    Sub { lhs: Operand, rhs: Operand },
    Mul { lhs: Operand, rhs: Operand },
    Neg { value: Operand },
}

impl Node {
    /// The one or two operands of this node, in source order.
    pub fn operands(&self) -> Vec<&Operand> {
        match self {
            Node::Add { lhs, rhs } | Node::Sub { lhs, rhs } | Node::Mul { lhs, rhs } => {
                vec![lhs, rhs]
            }
            Node::Neg { value } => vec![value],
        }
    }
}

fn is_zero(n: &i32) -> bool {
    *n == 0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Operand {
    /// Base field element, decimal, canonical in `[0, base_field)`.
    Const {
        value: String,
    },
    Witness {
        name: String,
        stage: u32,
        col: u32,
        #[serde(default, skip_serializing_if = "is_zero")]
        offset: i32,
    },
    Fixed {
        name: String,
        col: u32,
        #[serde(default, skip_serializing_if = "is_zero")]
        offset: i32,
    },
    Periodic {
        name: String,
        col: u32,
        #[serde(default, skip_serializing_if = "is_zero")]
        offset: i32,
    },
    Custom {
        name: String,
        commit: u32,
        stage: u32,
        col: u32,
        #[serde(default, skip_serializing_if = "is_zero")]
        offset: i32,
    },
    AirValue {
        name: String,
        idx: u32,
    },
    AirGroupValue {
        name: String,
        airgroup: u32,
        idx: u32,
    },
    Public {
        name: String,
        idx: u32,
    },
    ProofValue {
        name: String,
        stage: u32,
        idx: u32,
    },
    Challenge {
        name: String,
        stage: u32,
        idx: u32,
    },
    /// A reference into the expression pool. Expression operands carry no row
    /// offset in the pilout: a shifted intermediate is a separate expression.
    Expr {
        idx: u32,
        /// The PIL name, when the expression is a named intermediate.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Position in the AIR's constraint list.
    pub idx: u32,
    pub kind: Kind,
    /// Index into [`Ir::expressions`]; the constraint asserts it is zero.
    pub expr: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "when", rename_all = "snake_case")]
pub enum Kind {
    EveryRow,
    FirstRow,
    LastRow,
    /// `offset_min == 0` means the current row is at index 0; the frame is
    /// `offset_max - offset_min + 1` rows wide.
    EveryFrame {
        offset_min: u32,
        offset_max: u32,
    },
}

/// The `debugLine` of a constraint: where it is written, and how it was written.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// The constraint as the PIL source spells it.
    pub text: String,
}

impl Source {
    /// `debugLine` is `"<file>:<line> <text>"`. Anything that does not match
    /// is kept whole as `text` rather than dropped.
    pub fn parse(debug_line: &str) -> Option<Source> {
        let debug_line = debug_line.trim();
        if debug_line.is_empty() {
            return None;
        }
        let Some((loc, text)) = debug_line.split_once(' ') else {
            return Some(Source { file: None, line: None, text: debug_line.to_string() });
        };
        match loc.rsplit_once(':').and_then(|(f, l)| l.parse::<u32>().ok().map(|l| (f, l))) {
            Some((file, line)) => Some(Source {
                file: Some(file.to_string()),
                line: Some(line),
                text: text.trim().to_string(),
            }),
            None => Some(Source { file: None, line: None, text: debug_line.to_string() }),
        }
    }

    /// `main.pil:191`, or as much of it as the pilout gave us.
    pub fn location(&self) -> Option<String> {
        let file = self.file.as_deref()?;
        Some(match self.line {
            Some(line) => format!("{file}:{line}"),
            None => file.to_string(),
        })
    }
}

impl Ir {
    /// The expression indices the constraints actually reach, in ascending
    /// order. Roughly half the pool for Main: the rest serves hints.
    pub fn reachable(&self) -> Vec<u32> {
        let mut seen = vec![false; self.expressions.len()];
        let mut stack: Vec<u32> = self.constraints.iter().map(|c| c.expr).collect();
        while let Some(idx) = stack.pop() {
            let i = idx as usize;
            if i >= seen.len() || seen[i] {
                continue;
            }
            seen[i] = true;
            for operand in self.expressions[i].operands() {
                if let Operand::Expr { idx, .. } = operand {
                    stack.push(*idx);
                }
            }
        }
        seen.iter().enumerate().filter(|(_, s)| **s).map(|(i, _)| i as u32).collect()
    }

    /// How many times each expression is referenced, counting both references
    /// from other expressions and from constraints. Backends use this to decide
    /// what to inline: a node used once can be folded into its user, a node
    /// used twice must stay shared.
    pub fn use_counts(&self) -> Vec<u32> {
        let mut counts = vec![0u32; self.expressions.len()];
        for idx in self.reachable() {
            for operand in self.expressions[idx as usize].operands() {
                if let Operand::Expr { idx, .. } = operand {
                    if let Some(c) = counts.get_mut(*idx as usize) {
                        *c += 1;
                    }
                }
            }
        }
        for c in &self.constraints {
            if let Some(count) = counts.get_mut(c.expr as usize) {
                *count += 1;
            }
        }
        counts
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A two-constraint AIR over `a - (b + 2 * a')`, with the product shared.
    pub(crate) fn sample() -> Ir {
        let witness = |name: &str, id: u32, offset: i32| Operand::Witness {
            name: name.to_string(),
            stage: 1,
            col: id,
            offset,
        };
        Ir {
            schema: SCHEMA,
            pilout: PiloutMeta {
                name: "test".into(),
                file: "test.pilout".into(),
                blake3: "00".into(),
                base_field: "18446744069414584321".into(),
                num_stages: 2,
            },
            air: AirMeta {
                airgroup: "G".into(),
                airgroup_id: 0,
                air: "A".into(),
                air_id: 0,
                num_rows: 8,
                stage_widths: vec![2],
                aggregable: false,
            },
            columns: Columns {
                witness: vec![
                    Slot {
                        name: "a".into(),
                        base: "a".into(),
                        index: vec![],
                        stage: 1,
                        id: 0,
                        commit: None,
                    },
                    Slot {
                        name: "b".into(),
                        base: "b".into(),
                        index: vec![],
                        stage: 1,
                        id: 1,
                        commit: None,
                    },
                ],
                fixed: vec![],
                periodic: vec![],
                custom: vec![],
                intermediates: vec![Intermediate { expr: 2, name: "G.named".into() }],
            },
            globals: Globals {
                air_values: vec![],
                airgroup_values: vec![],
                publics: vec![],
                proof_values: vec![],
                challenges: vec![],
            },
            expressions: vec![
                // 0: 2 * a'   — used by both constraints, so shared
                Node::Mul { lhs: Operand::Const { value: "2".into() }, rhs: witness("a", 0, 1) },
                // 1: b + (2 * a')
                Node::Add { lhs: witness("b", 1, 0), rhs: Operand::Expr { idx: 0, name: None } },
                // 2: a - (b + 2 * a')   — named by the PIL
                Node::Sub { lhs: witness("a", 0, 0), rhs: Operand::Expr { idx: 1, name: None } },
                // 3: a * (2 * a')
                Node::Mul { lhs: witness("a", 0, 0), rhs: Operand::Expr { idx: 0, name: None } },
                // 4: unreachable from any constraint
                Node::Neg { value: witness("b", 1, 0) },
            ],
            constraints: vec![
                Constraint { idx: 0, kind: Kind::EveryRow, expr: 2, source: None },
                Constraint {
                    idx: 1,
                    kind: Kind::LastRow,
                    expr: 3,
                    source: Source::parse("x.pil:7 a*(2*a')"),
                },
            ],
        }
    }

    #[test]
    fn json_round_trips() {
        let json = serde_json::to_string(&sample()).unwrap();
        let back: Ir = serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), json);

        // Skipped zero offsets must come back as zero, not as an error.
        assert!(json.contains(r#""offset":1"#));
        assert!(!json.contains(r#""offset":0"#));
    }

    #[test]
    fn reachability_stops_at_the_constraints() {
        let ir = sample();
        assert_eq!(ir.reachable(), vec![0, 1, 2, 3]); // not 4
    }

    #[test]
    fn use_counts_see_both_expressions_and_constraints() {
        let counts = sample().use_counts();
        assert_eq!(counts[0], 2); // shared by expressions 1 and 3
        assert_eq!(counts[1], 1); // used by expression 2 only
        assert_eq!(counts[2], 1); // used by constraint 0
        assert_eq!(counts[3], 1); // used by constraint 1
        assert_eq!(counts[4], 0); // unreachable
    }
}
