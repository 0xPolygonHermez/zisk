//! [`Ir`] -> a constraint listing meant to be read and diffed.
//!
//! Same expression printer as the Lean backend, different leaves: PIL notation
//! rather than Lean, so the output can be compared against the `.pil` sources
//! the `debugLine` points at.

use std::fmt::Write as _;

use crate::ir::{Ir, Kind};
use crate::print::{self, Leaves, Plan, MAX_INLINE};

pub fn render(ir: &Ir) -> String {
    let plan = Plan::build(ir, MAX_INLINE);
    let leaves = PilLeaves { plan: &plan };
    let mut out = String::new();

    let air = &ir.air;
    let _ = writeln!(out, "{}/{} — {} constraints", air.airgroup, air.air, ir.constraints.len());
    let _ = writeln!(
        out,
        "  pilout      {} (blake3 {})",
        ir.pilout.file,
        &ir.pilout.blake3[..ir.pilout.blake3.len().min(16)]
    );
    let _ = writeln!(out, "  base field  {}", ir.pilout.base_field);
    let _ = writeln!(out, "  rows        {}", air.num_rows);
    let _ = writeln!(out, "  stages      {:?}", air.stage_widths);
    let _ = writeln!(
        out,
        "  columns     {} witness, {} fixed, {} periodic, {} custom",
        ir.columns.witness.len(),
        ir.columns.fixed.len(),
        ir.columns.periodic.len(),
        ir.columns.custom.len()
    );
    let _ = writeln!(
        out,
        "  values      {} air, {} airgroup, {} public, {} proof, {} challenge",
        ir.globals.air_values.len(),
        ir.globals.airgroup_values.len(),
        ir.globals.publics.len(),
        ir.globals.proof_values.len(),
        ir.globals.challenges.len()
    );
    let _ = writeln!(
        out,
        "  expressions {} reachable of {} in the pool",
        ir.reachable().len(),
        ir.expressions.len()
    );

    let shared: Vec<u32> = plan.definitions().collect();
    if !shared.is_empty() {
        let _ = writeln!(out, "\nshared subexpressions ({})", shared.len());
        for idx in shared {
            let name = leaves.name(idx);
            let _ = writeln!(out, "\n  {name} =");
            let _ = writeln!(out, "    {}", print::render(ir, &plan, &leaves, idx));
        }
    }

    let _ = writeln!(out, "\nconstraints ({})", ir.constraints.len());
    for c in &ir.constraints {
        let kind = match c.kind {
            Kind::EveryRow => "every row".to_string(),
            Kind::FirstRow => "first row".to_string(),
            Kind::LastRow => "last row".to_string(),
            Kind::EveryFrame { offset_min, offset_max } => {
                format!("every frame [{offset_min}, {offset_max}]")
            }
        };
        let location = c.source.as_ref().and_then(|s| s.location()).unwrap_or_default();
        let _ = writeln!(out, "\n  [{}] {kind}  {location}", c.idx);
        if let Some(source) = &c.source {
            let _ = writeln!(out, "       pil: {}", source.text);
        }
        let body = if plan.is_defined(c.expr) {
            leaves.name(c.expr)
        } else {
            print::render(ir, &plan, &leaves, c.expr)
        };
        let _ = writeln!(out, "       0 = {body}");
    }
    out
}

struct PilLeaves<'a> {
    plan: &'a Plan,
}

impl PilLeaves<'_> {
    /// A shared expression's name: the PIL's own if it has one, else the pilout
    /// index, which is how to find it in the protobuf.
    fn name(&self, idx: u32) -> String {
        match self.plan.named.get(&idx) {
            Some(name) => name.clone(),
            None => format!("e{idx}"),
        }
    }
}

impl Leaves for PilLeaves<'_> {
    fn constant(&self, value: &str) -> String {
        value.to_string()
    }

    /// PIL writes the next row as `x'`. There is no notation for the general
    /// case, so anything else is spelled out.
    fn column(&self, name: &str, offset: i32) -> String {
        match offset {
            0 => name.to_string(),
            1 => format!("{name}'"),
            o => format!("{name}@{o:+}"),
        }
    }

    fn global(&self, name: &str) -> String {
        name.to_string()
    }

    fn reference(&self, idx: u32) -> String {
        self.name(idx)
    }
}
