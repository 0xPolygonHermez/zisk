use serde::Serialize;
use tera::{Context, Tera};

use crate::error::{RecurserError, Result};

const RECURSER_TMPL: &str = include_str!("../templates/aggregator.circom.tera");
/// `GetPublic{LE,BE}` for reading typed values from the 64-slot publics array.
pub const PUBLICS_HELPERS_CIRCOM: &str = include_str!("../circom_helpers/publics_helpers.circom");
pub const PUBLICS_HELPERS_FILENAME: &str = "publics_helpers.circom";

/// The template name every normalize body must define. The generator renames
/// it to `NormalizePublics_<g>` at injection so multiple groups coexist.
pub const NORMALIZE_TEMPLATE_NAME: &str = "NormalizePublics";

fn render(template_src: &str, ctx: &Context) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template("t", template_src)?;
    Ok(tera.render("t", ctx)?)
}

#[derive(Debug, Clone)]
pub struct StarkInputBlocks<'a> {
    pub define_a: &'a str,
    pub define_b: &'a str,
    pub assign_a: &'a str,
    pub assign_b: &'a str,
}

/// One normalization group: a `NormalizePublics` Circom body applied to the
/// leaf proofs of the member programs the first time they enter the
/// recursion. Programs not in any group pass their publics through raw.
#[derive(Debug, Clone)]
pub struct NormalizeGroup {
    /// Indices into the registered `program_vks[]` allowlist.
    pub member_indices: Vec<usize>,
    /// Circom body defining `template NormalizePublics(nPublics, nFreeInputs)`.
    pub body: String,
    /// Side inputs this group's circuit consumes; the circuit's shared
    /// per-side array is sized to the max across groups.
    pub n_free_inputs: usize,
}

/// Circom bodies injected verbatim into the recurser. Required signatures are
/// documented in `recurser/docs/aggregator-flow.md`. `AggregatePublics` both
/// asserts the caller's consistency constraints and produces the merged
/// publics; normalization groups are optional.
#[derive(Debug, Clone)]
pub struct CircomTemplates {
    pub normalize_groups: Vec<NormalizeGroup>,
    pub aggregate_publics: String,
}

impl CircomTemplates {
    /// Size of the per-side `freeInputs` arrays: worst case across groups.
    pub fn max_free_inputs(&self) -> usize {
        self.normalize_groups.iter().map(|g| g.n_free_inputs).max().unwrap_or(0)
    }
}

/// Validate the group structure against the registered program count.
/// Soundness of the in-circuit membership muxing rests on this: indices in
/// range, no program in two groups, no empty groups.
pub fn validate_normalize_groups(groups: &[NormalizeGroup], n_programs: usize) -> Result<()> {
    let mut seen = vec![false; n_programs];
    for (g, group) in groups.iter().enumerate() {
        if group.member_indices.is_empty() {
            return Err(RecurserError::InvalidTemplates(format!(
                "normalize group {g} has no member programs"
            )));
        }
        for &idx in &group.member_indices {
            if idx >= n_programs {
                return Err(RecurserError::InvalidTemplates(format!(
                    "normalize group {g} references program index {idx}, but only {n_programs} \
                     programs are registered"
                )));
            }
            if seen[idx] {
                return Err(RecurserError::InvalidTemplates(format!(
                    "program index {idx} appears more than once across normalize groups"
                )));
            }
            seen[idx] = true;
        }
    }
    Ok(())
}

/// Rename a group body's `NormalizePublics` template to `NormalizePublics_<g>`
/// so the bodies of all groups can coexist in one generated file.
fn rename_normalize_template(body: &str, group_idx: usize) -> Result<String> {
    let needle = format!("template {NORMALIZE_TEMPLATE_NAME}(");
    let occurrences = body.matches(&needle).count();
    if occurrences != 1 {
        return Err(RecurserError::InvalidTemplates(format!(
            "normalize group {group_idx} body must define `template \
             {NORMALIZE_TEMPLATE_NAME}(...)` exactly once, found {occurrences} occurrences"
        )));
    }
    Ok(body.replace(&needle, &format!("template {NORMALIZE_TEMPLATE_NAME}_{group_idx}(")))
}

/// Per-group values handed to the tera template. Everything tricky
/// (renaming, last-position arithmetic) is precomputed here so the template
/// stays dumb.
#[derive(Debug, Serialize)]
struct TeraNormalizeGroup {
    idx: usize,
    n: usize,
    members: Vec<usize>,
    last_member_pos: usize,
    body: String,
}

pub fn gen_recurser(
    verifier_filename: &str,
    zisk_vk: &[String],
    program_vks: &[[String; 4]],
    stark_inputs: &StarkInputBlocks<'_>,
    templates: &CircomTemplates,
) -> Result<String> {
    let n_programs = program_vks.len();
    validate_normalize_groups(&templates.normalize_groups, n_programs)?;

    let groups: Vec<TeraNormalizeGroup> = templates
        .normalize_groups
        .iter()
        .enumerate()
        .map(|(idx, g)| {
            Ok(TeraNormalizeGroup {
                idx,
                n: g.n_free_inputs,
                members: g.member_indices.clone(),
                last_member_pos: g.member_indices.len() - 1,
                body: rename_normalize_template(&g.body, idx)?,
            })
        })
        .collect::<Result<_>>()?;

    let mut ctx = Context::new();
    ctx.insert("verifier_filename", verifier_filename);
    ctx.insert("n_free_inputs", &templates.max_free_inputs());
    ctx.insert("n_programs", &n_programs);
    ctx.insert("program_vks", program_vks);
    ctx.insert("root_c_vadcop_final_zisk", &zisk_vk);
    ctx.insert("aggregate_publics_template", &templates.aggregate_publics);
    ctx.insert("normalize_groups", &groups);
    ctx.insert("define_stark_inputs_a", stark_inputs.define_a);
    ctx.insert("define_stark_inputs_b", stark_inputs.define_b);
    ctx.insert("assign_stark_inputs_a", stark_inputs.assign_a);
    ctx.insert("assign_stark_inputs_b", stark_inputs.assign_b);

    render(RECURSER_TMPL, &ctx)
}
