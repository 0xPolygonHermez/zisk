//! `.pilout` (protobuf) -> [`Ir`].
//!
//! The protobuf refers to everything by index: a witness operand is
//! `(stage, colIdx, rowOffset)`, a constraint is an index into the AIR's
//! expression pool. The names live apart, in `PilOut.symbols`, keyed by the
//! same indices and scoped by `(airGroupId, airId)`. This module joins the two
//! so that no consumer of the IR has to.

use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};
use pil2_pilout::pilout::{self, SymbolType};
use pil2_pilout::pilout_proxy::PilOutProxy;

use crate::ir::*;

/// A decoded `.pilout`, plus the identity of the file it came from.
pub struct Pilout {
    pub pilout: PilOutProxy,
    file: String,
    blake3: String,
}

/// One AIR's coordinates and names.
pub struct AirRef {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub airgroup: String,
    pub air: String,
}

impl Pilout {
    pub fn load(path: &str) -> Result<Pilout> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {path}"))?;
        let blake3 = blake3::hash(&bytes).to_hex().to_string();
        drop(bytes);

        // PilOutProxy reads the file itself. Reading it twice costs a few ms on
        // a 3.5 MB pilout and saves this crate a direct prost dependency, which
        // would have to be kept in version lockstep with pil2-pilout's.
        let pilout = PilOutProxy::new(path).map_err(|e| anyhow!("decoding {path}: {e}"))?;
        let file = std::path::Path::new(path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());

        Ok(Pilout { pilout, file, blake3 })
    }

    /// Every AIR in the pilout, in declaration order.
    pub fn airs(&self) -> Vec<AirRef> {
        let mut out = Vec::new();
        for (airgroup_id, airgroup) in self.pilout.air_groups.iter().enumerate() {
            for (air_id, air) in airgroup.airs.iter().enumerate() {
                out.push(AirRef {
                    airgroup_id,
                    air_id,
                    airgroup: name_of(&airgroup.name, airgroup_id, "airgroup"),
                    air: name_of(&air.name, air_id, "air"),
                });
            }
        }
        out
    }

    /// The AIRs matching a name filter. `air` is matched case-insensitively
    /// against the AIR name; `None` for both means every AIR.
    pub fn select(&self, airgroup: Option<&str>, air: Option<&str>) -> Result<Vec<AirRef>> {
        let matches = |want: Option<&str>, have: &str| match want {
            None => true,
            Some(w) => w.eq_ignore_ascii_case(have),
        };
        let selected: Vec<AirRef> = self
            .airs()
            .into_iter()
            .filter(|a| matches(airgroup, &a.airgroup) && matches(air, &a.air))
            .collect();

        if selected.is_empty() {
            let known: Vec<String> =
                self.airs().iter().map(|a| format!("{}/{}", a.airgroup, a.air)).collect();
            bail!(
                "no AIR matches {}/{} in {}. Known AIRs: {}",
                airgroup.unwrap_or("*"),
                air.unwrap_or("*"),
                self.file,
                known.join(", ")
            );
        }
        Ok(selected)
    }

    pub fn extract(&self, air_ref: &AirRef) -> Result<Ir> {
        let air = &self.pilout.air_groups[air_ref.airgroup_id].airs[air_ref.air_id];
        let symbols = Symbols::collect(&self.pilout, air_ref.airgroup_id, air_ref.air_id);

        let expressions = air
            .expressions
            .iter()
            .enumerate()
            .map(|(i, e)| symbols.node(e, i))
            .collect::<Result<Vec<_>>>()?;
        let constraints = air
            .constraints
            .iter()
            .enumerate()
            .map(|(i, c)| constraint(c, i))
            .collect::<Result<Vec<_>>>()?;

        Ok(Ir {
            schema: SCHEMA,
            pilout: PiloutMeta {
                name: self.pilout.name.clone().unwrap_or_default(),
                file: self.file.clone(),
                blake3: self.blake3.clone(),
                base_field: be_bytes_to_decimal(&self.pilout.base_field),
                num_stages: self.pilout.num_stages(),
            },
            air: AirMeta {
                airgroup: air_ref.airgroup.clone(),
                airgroup_id: air_ref.airgroup_id as u32,
                air: air_ref.air.clone(),
                air_id: air_ref.air_id as u32,
                num_rows: air.num_rows.unwrap_or(0) as u64,
                stage_widths: air.stage_widths.clone(),
                aggregable: air.aggregable,
            },
            columns: Columns {
                witness: symbols.slots(SymbolType::WitnessCol),
                fixed: symbols.slots(SymbolType::FixedCol),
                periodic: symbols.slots(SymbolType::PeriodicCol),
                custom: symbols.slots(SymbolType::CustomCol),
                intermediates: symbols.intermediates(),
            },
            globals: Globals {
                air_values: symbols.slots(SymbolType::AirValue),
                airgroup_values: symbols.slots(SymbolType::AirGroupValue),
                publics: symbols.slots(SymbolType::PublicValue),
                proof_values: symbols.slots(SymbolType::ProofValue),
                challenges: symbols.slots(SymbolType::Challenge),
            },
            expressions,
            constraints,
        })
    }
}

fn name_of(name: &Option<String>, idx: usize, what: &str) -> String {
    name.clone().unwrap_or_else(|| format!("<unnamed {what} {idx}>"))
}

/// Base field elements are variable-length big-endian, and an empty value is
/// zero. Goldilocks fits in a u64, but nothing in the format promises that, so
/// this does the decimal conversion by hand rather than through an integer.
fn be_bytes_to_decimal(bytes: &[u8]) -> String {
    let mut digits: Vec<u8> = vec![0]; // little-endian decimal digits
    for byte in bytes {
        let mut carry = 0u32;
        for d in digits.iter_mut() {
            let v = *d as u32 * 256 + carry;
            *d = (v % 10) as u8;
            carry = v / 10;
        }
        while carry > 0 {
            digits.push((carry % 10) as u8);
            carry /= 10;
        }
        let mut carry = *byte as u32;
        let mut i = 0;
        while carry > 0 {
            if i == digits.len() {
                digits.push(0);
            }
            let v = digits[i] as u32 + carry;
            digits[i] = (v % 10) as u8;
            carry = v / 10;
            i += 1;
        }
    }
    digits.iter().rev().map(|d| char::from(b'0' + d)).collect()
}

/// The symbol table of one AIR, indexed the way operands reference it.
struct Symbols {
    /// `(type, stage, commit, id)` -> rendered name. See [`key_stage`]: only
    /// some symbol types are numbered per stage, and only custom columns are
    /// numbered per commit.
    names: HashMap<(i32, u32, u32, u32), String>,
    /// Flattened slots per symbol type, in id order.
    slots: HashMap<i32, Vec<Slot>>,
    /// Named intermediates, by the expression index they name.
    im_names: HashMap<u32, String>,
    /// The airgroup this AIR belongs to, for airgroup-value operands, which
    /// carry only an index.
    airgroup_id: u32,
}

/// Which symbol types are numbered per stage. A witness column's index is
/// relative to its stage, so `(stage, id)` identifies it; an air value or a
/// public is numbered once for the whole AIR and its operand carries no stage
/// at all, so keying those by stage would never match.
fn key_stage(kind: SymbolType, stage: u32) -> u32 {
    match kind {
        SymbolType::WitnessCol
        | SymbolType::CustomCol
        | SymbolType::Challenge
        | SymbolType::ProofValue => stage,
        _ => 0,
    }
}

impl Symbols {
    fn collect(pilout: &PilOutProxy, airgroup_id: usize, air_id: usize) -> Symbols {
        let mut this = Symbols {
            names: HashMap::new(),
            slots: HashMap::new(),
            im_names: HashMap::new(),
            airgroup_id: airgroup_id as u32,
        };

        for symbol in &pilout.symbols {
            // A symbol applies here if it is scoped to this AIR, to this
            // airgroup with no AIR, or global (challenges, publics, proof
            // values). Anything scoped to another AIR is skipped.
            let in_scope = match (symbol.air_group_id, symbol.air_id) {
                (Some(g), Some(a)) => g as usize == airgroup_id && a as usize == air_id,
                (Some(g), None) => g as usize == airgroup_id,
                (None, _) => true,
            };
            if !in_scope {
                continue;
            }
            let Ok(kind) = SymbolType::try_from(symbol.r#type) else {
                continue;
            };
            let stage = symbol.stage.unwrap_or(0);

            if kind == SymbolType::ImCol {
                // An intermediate's id is its index in the expression pool.
                // Names are not unique (the compiler emits two `previous_c`
                // intermediates in Main), but expression indices are.
                this.im_names.insert(symbol.id, symbol.name.clone());
                continue;
            }

            for (offset, index) in flatten(&symbol.lengths).into_iter().enumerate() {
                let id = symbol.id + offset as u32;
                let name = render_name(&symbol.name, &index);
                let commit = symbol.commit_id.unwrap_or(0);
                this.names
                    .insert((symbol.r#type, key_stage(kind, stage), commit, id), name.clone());
                this.slots.entry(symbol.r#type).or_default().push(Slot {
                    name,
                    base: symbol.name.clone(),
                    index,
                    stage,
                    id,
                    commit: symbol.commit_id,
                });
            }
        }

        for slots in this.slots.values_mut() {
            slots.sort_by_key(|s| (s.stage, s.commit, s.id));
        }
        this
    }

    fn slots(&self, kind: SymbolType) -> Vec<Slot> {
        self.slots.get(&(kind as i32)).cloned().unwrap_or_default()
    }

    /// Named intermediates, ordered by expression index.
    fn intermediates(&self) -> Vec<Intermediate> {
        let mut out: Vec<Intermediate> = self
            .im_names
            .iter()
            .map(|(expr, name)| Intermediate { expr: *expr, name: name.clone() })
            .collect();
        out.sort_by_key(|i| i.expr);
        out
    }

    /// The name for an operand, or a synthetic one. A missing name is normal:
    /// the std library commits columns the PIL never names.
    fn name(&self, kind: SymbolType, stage: u32, id: u32, fallback: &str) -> String {
        self.name_in(kind, stage, 0, id, fallback)
    }

    fn name_in(
        &self,
        kind: SymbolType,
        stage: u32,
        commit: u32,
        id: u32,
        fallback: &str,
    ) -> String {
        self.names
            .get(&(kind as i32, key_stage(kind, stage), commit, id))
            .cloned()
            .unwrap_or_else(|| format!("{fallback}{id}"))
    }

    fn node(&self, e: &pilout::Expression, idx: usize) -> Result<Node> {
        use pilout::expression::Operation;
        let operation =
            e.operation.as_ref().ok_or_else(|| anyhow!("expression {idx} has no operation"))?;
        let binary = |lhs: &Option<pilout::Operand>, rhs: &Option<pilout::Operand>| {
            let lhs = lhs.as_ref().ok_or_else(|| anyhow!("expression {idx} has no lhs"))?;
            let rhs = rhs.as_ref().ok_or_else(|| anyhow!("expression {idx} has no rhs"))?;
            Ok::<_, anyhow::Error>((self.operand(lhs, idx)?, self.operand(rhs, idx)?))
        };
        Ok(match operation {
            Operation::Add(op) => {
                let (lhs, rhs) = binary(&op.lhs, &op.rhs)?;
                Node::Add { lhs, rhs }
            }
            Operation::Sub(op) => {
                let (lhs, rhs) = binary(&op.lhs, &op.rhs)?;
                Node::Sub { lhs, rhs }
            }
            Operation::Mul(op) => {
                let (lhs, rhs) = binary(&op.lhs, &op.rhs)?;
                Node::Mul { lhs, rhs }
            }
            Operation::Neg(op) => {
                let value =
                    op.value.as_ref().ok_or_else(|| anyhow!("expression {idx} negates nothing"))?;
                Node::Neg { value: self.operand(value, idx)? }
            }
        })
    }

    fn operand(&self, o: &pilout::Operand, idx: usize) -> Result<Operand> {
        use pilout::operand::Operand as O;
        let operand =
            o.operand.as_ref().ok_or_else(|| anyhow!("expression {idx} has an empty operand"))?;
        Ok(match operand {
            O::Constant(c) => Operand::Const { value: be_bytes_to_decimal(&c.value) },
            O::WitnessCol(c) => Operand::Witness {
                name: self.name(SymbolType::WitnessCol, c.stage, c.col_idx, "w"),
                stage: c.stage,
                col: c.col_idx,
                offset: c.row_offset,
            },
            O::FixedCol(c) => Operand::Fixed {
                name: self.name(SymbolType::FixedCol, 0, c.idx, "fixed"),
                col: c.idx,
                offset: c.row_offset,
            },
            O::PeriodicCol(c) => Operand::Periodic {
                name: self.name(SymbolType::PeriodicCol, 0, c.idx, "periodic"),
                col: c.idx,
                offset: c.row_offset,
            },
            O::CustomCol(c) => Operand::Custom {
                name: self.name_in(
                    SymbolType::CustomCol,
                    c.stage,
                    c.commit_id,
                    c.col_idx,
                    "custom",
                ),
                commit: c.commit_id,
                stage: c.stage,
                col: c.col_idx,
                offset: c.row_offset,
            },
            O::AirValue(v) => Operand::AirValue {
                name: self.name(SymbolType::AirValue, 0, v.idx, "airvalue"),
                idx: v.idx,
            },
            O::AirGroupValue(v) => Operand::AirGroupValue {
                name: self.name(SymbolType::AirGroupValue, 0, v.idx, "airgroupvalue"),
                airgroup: self.airgroup_id,
                idx: v.idx,
            },
            O::PublicValue(v) => Operand::Public {
                name: self.name(SymbolType::PublicValue, 0, v.idx, "public"),
                idx: v.idx,
            },
            O::ProofValue(v) => Operand::ProofValue {
                name: self.name(SymbolType::ProofValue, v.stage, v.idx, "proofvalue"),
                stage: v.stage,
                idx: v.idx,
            },
            O::Challenge(c) => Operand::Challenge {
                name: self.name(SymbolType::Challenge, c.stage, c.idx, "challenge"),
                stage: c.stage,
                idx: c.idx,
            },
            O::Expression(e) => {
                Operand::Expr { idx: e.idx, name: self.im_names.get(&e.idx).cloned() }
            }
        })
    }
}

/// Row-major index vectors for a symbol of the given shape. A scalar yields one
/// empty vector, so callers can treat both cases the same way.
fn flatten(lengths: &[u32]) -> Vec<Vec<u32>> {
    let mut out = vec![vec![]];
    for len in lengths {
        let mut next = Vec::with_capacity(out.len() * *len as usize);
        for prefix in &out {
            for i in 0..*len {
                let mut index = prefix.clone();
                index.push(i);
                next.push(index);
            }
        }
        out = next;
    }
    out
}

fn render_name(base: &str, index: &[u32]) -> String {
    let mut name = base.to_string();
    for i in index {
        name.push_str(&format!("[{i}]"));
    }
    name
}

fn constraint(c: &pilout::Constraint, idx: usize) -> Result<Constraint> {
    use pilout::constraint::Constraint as C;
    let inner = c.constraint.as_ref().ok_or_else(|| anyhow!("constraint {idx} is empty"))?;
    let (kind, expr, debug_line) = match inner {
        C::EveryRow(c) => (Kind::EveryRow, c.expression_idx.as_ref(), c.debug_line.as_deref()),
        C::FirstRow(c) => (Kind::FirstRow, c.expression_idx.as_ref(), c.debug_line.as_deref()),
        C::LastRow(c) => (Kind::LastRow, c.expression_idx.as_ref(), c.debug_line.as_deref()),
        C::EveryFrame(c) => (
            Kind::EveryFrame { offset_min: c.offset_min, offset_max: c.offset_max },
            c.expression_idx.as_ref(),
            c.debug_line.as_deref(),
        ),
    };
    let expr = expr.ok_or_else(|| anyhow!("constraint {idx} references no expression"))?.idx;
    Ok(Constraint { idx: idx as u32, kind, expr, source: debug_line.and_then(Source::parse) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_conversion_handles_goldilocks_and_the_empty_value() {
        assert_eq!(be_bytes_to_decimal(&[]), "0");
        assert_eq!(be_bytes_to_decimal(&[0]), "0");
        assert_eq!(be_bytes_to_decimal(&[1]), "1");
        assert_eq!(
            be_bytes_to_decimal(&[0xff, 0xff, 0xff, 0xff, 0, 0, 0, 1]),
            "18446744069414584321"
        );
    }

    #[test]
    fn flatten_is_row_major_and_keeps_scalars_scalar() {
        assert_eq!(flatten(&[]), vec![Vec::<u32>::new()]);
        assert_eq!(flatten(&[2]), vec![vec![0], vec![1]]);
        // Main.last_reg_value is [31][2] and the next symbol sits 62 ids later.
        let flat = flatten(&[31, 2]);
        assert_eq!(flat.len(), 62);
        assert_eq!(flat[0], vec![0, 0]);
        assert_eq!(flat[1], vec![0, 1]);
        assert_eq!(flat[2], vec![1, 0]);
    }

    #[test]
    fn source_parses_a_debug_line_and_survives_a_malformed_one() {
        let s =
            Source::parse("main/pil/main.pil:191 addr1-(b_offset_imm0+(b_src_ind*a[0]))").unwrap();
        assert_eq!(s.file.as_deref(), Some("main/pil/main.pil"));
        assert_eq!(s.line, Some(191));
        assert_eq!(s.text, "addr1-(b_offset_imm0+(b_src_ind*a[0]))");
        assert_eq!(s.location().as_deref(), Some("main/pil/main.pil:191"));

        assert!(Source::parse("").is_none());
        let s = Source::parse("no location here").unwrap();
        assert_eq!(s.file, None);
        assert_eq!(s.text, "no location here");
    }
}
