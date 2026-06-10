use tera::{Context, Tera};

use crate::error::Result;

const AGGREGATOR_TMPL: &str = include_str!("../templates/aggregator.circom.tera");
/// Identity passthrough — used when the caller passes `prepare_publics: None`.
pub const DEFAULT_PREPARE_PUBLICS: &str = include_str!("../templates/prepare_publics.circom");
/// No-op (no stitching constraints) — used when the caller passes `check_publics: None`.
pub const DEFAULT_CHECK_PUBLICS: &str = include_str!("../templates/check_publics.circom");
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

/// Circom bodies for the three publics-handling sub-templates, injected
/// verbatim into the aggregator. Required signatures are documented in
/// `recurser/docs/aggregator-flow.md`.
#[derive(Debug, Clone)]
pub struct CircomTemplates {
    /// `None` uses the built-in identity passthrough.
    pub prepare_publics: Option<String>,
    /// `None` uses the built-in no-op body.
    pub check_publics: Option<String>,
    pub aggregate_publics: String,
}

pub fn gen_aggregator(
    n_private_inputs: usize,
    verifier_filename: &str,
    zisk_vk: &[String],
    program_vks: &[[String; 4]],
    stark_inputs: &StarkInputBlocks<'_>,
    templates: &CircomTemplates,
) -> Result<String> {
    let n_programs = program_vks.len();

    let mut ctx = Context::new();
    ctx.insert("verifier_filename", verifier_filename);
    ctx.insert("n_private_inputs", &n_private_inputs);
    ctx.insert("n_programs", &n_programs);
    ctx.insert("program_vks", program_vks);
    ctx.insert("root_c_vadcop_final_zisk", &zisk_vk);
    ctx.insert("aggregate_publics_template", &templates.aggregate_publics);
    ctx.insert(
        "prepare_publics_template",
        templates.prepare_publics.as_deref().unwrap_or(DEFAULT_PREPARE_PUBLICS),
    );
    ctx.insert(
        "check_publics_template",
        templates.check_publics.as_deref().unwrap_or(DEFAULT_CHECK_PUBLICS),
    );
    ctx.insert("define_stark_inputs_a", stark_inputs.define_a);
    ctx.insert("define_stark_inputs_b", stark_inputs.define_b);
    ctx.insert("assign_stark_inputs_a", stark_inputs.assign_a);
    ctx.insert("assign_stark_inputs_b", stark_inputs.assign_b);

    render(AGGREGATOR_TMPL, &ctx)
}
