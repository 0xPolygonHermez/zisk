use tera::{Context, Tera};

use crate::error::Result;

const RECURSER_TMPL: &str = include_str!("../templates/aggregator.circom.tera");
/// `GetPublic{LE,BE}` for reading typed values from the 64-slot publics array.
pub const PUBLICS_HELPERS_CIRCOM: &str = include_str!("../circom_helpers/publics_helpers.circom");
pub const PUBLICS_HELPERS_FILENAME: &str = "publics_helpers.circom";

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

/// Circom bodies injected verbatim into the recurser. Required signatures are
/// documented in `recurser/docs/aggregator-flow.md`. `AggregatePublics` both
/// asserts the caller's consistency constraints and produces the merged
/// publics.
///
/// All accepted guest programs must emit the same publics layout — the
/// recurser folds publics through raw, with no per-program canonicalization.
/// Aggregating programs with different publics layouts is intentionally
/// unsupported.
#[derive(Debug, Clone)]
pub struct CircomTemplates {
    pub aggregate_publics: String,
    /// Free-input slots the `AggregatePublics` template reads; sizes the
    /// circuit's per-side `freeInputs` arrays. Lets a recurser feed preimages
    /// to `AggregatePublics` (e.g. hash-style publics checked and re-hashed at
    /// the fold).
    pub aggregate_n_free_inputs: usize,
}

impl CircomTemplates {
    /// Size of the per-side `freeInputs` arrays.
    pub fn max_free_inputs(&self) -> usize {
        self.aggregate_n_free_inputs
    }
}

pub fn gen_recurser(
    verifier_filename: &str,
    zisk_vk: &[String],
    program_vks: &[[String; 4]],
    stark_inputs: &StarkInputBlocks<'_>,
    templates: &CircomTemplates,
) -> Result<String> {
    let n_programs = program_vks.len();

    let mut ctx = Context::new();
    ctx.insert("verifier_filename", verifier_filename);
    ctx.insert("n_free_inputs", &templates.max_free_inputs());
    ctx.insert("n_programs", &n_programs);
    ctx.insert("program_vks", program_vks);
    ctx.insert("root_c_vadcop_final_zisk", &zisk_vk);
    ctx.insert("aggregate_publics_template", &templates.aggregate_publics);
    ctx.insert("define_stark_inputs_a", stark_inputs.define_a);
    ctx.insert("define_stark_inputs_b", stark_inputs.define_b);
    ctx.insert("assign_stark_inputs_a", stark_inputs.assign_a);
    ctx.insert("assign_stark_inputs_b", stark_inputs.assign_b);

    render(RECURSER_TMPL, &ctx)
}
