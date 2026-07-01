use tera::{Context, Tera};

use crate::error::{RecurserError, Result};

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

/// Reserved public slot for `is_vadcop_final_proof` (1 = raw ZisK vadcop_final
/// leaf, 0 = aggregator output). Slot 0, before rom_root: the vadcop_final
/// circuit emits it as a `signal output`, which circom orders ahead of the
/// input publics. It is stripped in the final_compressed / recursion_final
/// layers, so it never reaches the flag-free publics.json / on-chain hash.
pub const IS_VADCOP_FINAL_SLOT: usize = 0;

/// The optional single normalization circuit applied to every leaf.
#[derive(Debug, Clone)]
pub struct NormalizeCircuit {
    /// Circom body: `template NormalizePublics(nPublics[, nFreeInputs])`.
    pub body: String,
}

/// Circom bodies injected verbatim into the recurser. Required signatures are
/// documented in `recurser/docs/aggregator-flow.md`. `AggregatePublics` both
/// asserts the caller's consistency constraints and produces the merged
/// publics; the optional normalize circuit applies to every leaf proof.
#[derive(Debug, Clone)]
pub struct CircomTemplates {
    /// Optional single normalize circuit (applies to all leaves).
    pub normalize: Option<NormalizeCircuit>,
    pub aggregate_publics: String,
    /// Single unified free-value width per side. It is BOTH the entry template's
    /// (NormalizePublics) free-input width AND the aggregate's free-value width:
    /// NormalizePublics consumes `n_free` free inputs and emits `n_free`
    /// free outputs, which feed AggregatePublics.
    pub n_free: usize,
}

impl CircomTemplates {
    /// The unified free-value width (0 = no free inputs/outputs).
    pub fn n_free(&self) -> usize {
        self.n_free
    }
}

/// Assert the user circom body declares `template <template>(...)` with the
/// arity the recurser scaffolding instantiates it with:
///
/// - `NormalizePublics` takes a leading `nPublics` param, plus `nFreeInputs`
///   when `n_free > 0`: `NormalizePublics(nPublics)` or
///   `NormalizePublics(nPublics, nFreeInputs)`. With `n_free > 0` it MUST also
///   emit a `free_outputs` output (the free values that feed AggregatePublics).
/// - `AggregatePublics` sizes its publics arrays via `ZISK_PUBLICS()`, so it
///   has NO `nPublics` param — only `nFreeInputs` when `n_free > 0`:
///   `AggregatePublics()` or `AggregatePublics(nFreeInputs)`.
///
/// The tera instantiations (`NormalizePublics(nPublics[, n])`,
/// `AggregatePublics([n])`) are the contract these counts mirror.
pub fn expect_template_arity(body: &str, template: &str, n_free: usize) -> Result<()> {
    let needle = format!("template {template}(");
    let start = body.find(&needle).ok_or_else(|| {
        RecurserError::InvalidTemplates(format!("body must define `template {template}(...)`"))
    })?;
    let after = &body[start + needle.len()..];
    let close = after.find(')').ok_or_else(|| {
        RecurserError::InvalidTemplates(format!("malformed `template {template}(` declaration"))
    })?;
    let params: Vec<&str> =
        after[..close].split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    // NormalizePublics carries a leading `nPublics`; AggregatePublics does not.
    let base_params = if template == "NormalizePublics" { 1 } else { 0 };
    let expected = base_params + if n_free > 0 { 1 } else { 0 };
    if params.len() != expected {
        return Err(RecurserError::InvalidTemplates(format!(
            "`{template}` declares {} params but n_free={n_free} requires {expected}",
            params.len()
        )));
    }
    // New contract: NormalizePublics with free inputs MUST emit `free_outputs`.
    if template == "NormalizePublics" && n_free > 0 && !declares_free_outputs(body) {
        return Err(RecurserError::InvalidTemplates(format!(
            "`{template}` with n_free={n_free} must declare `signal output free_outputs[...]`"
        )));
    }
    Ok(())
}

/// Structural check: does the body declare a `free_outputs` output signal?
/// Matches `signal output free_outputs` allowing arbitrary internal spacing.
fn declares_free_outputs(body: &str) -> bool {
    body.split("signal").any(|seg| {
        let seg = seg.trim_start();
        seg.starts_with("output") && seg["output".len()..].trim_start().starts_with("free_outputs")
    })
}

pub fn gen_recurser(
    verifier_filename: &str,
    zisk_vk: &[String],
    stark_inputs: &StarkInputBlocks<'_>,
    templates: &CircomTemplates,
) -> Result<String> {
    let n_free = templates.n_free();

    expect_template_arity(&templates.aggregate_publics, "AggregatePublics", n_free)?;
    if let Some(norm) = &templates.normalize {
        expect_template_arity(&norm.body, "NormalizePublics", n_free)?;
    }

    let mut ctx = Context::new();
    ctx.insert("verifier_filename", verifier_filename);
    ctx.insert("is_vadcop_final_slot", &IS_VADCOP_FINAL_SLOT);
    ctx.insert("root_c_vadcop_final_zisk", &zisk_vk);
    ctx.insert("aggregate_publics_template", &templates.aggregate_publics);
    ctx.insert("n_free", &n_free);
    match &templates.normalize {
        Some(norm) => {
            ctx.insert("normalize", &true);
            ctx.insert("normalize_body", &norm.body);
        }
        None => ctx.insert("normalize", &false),
    }
    ctx.insert("define_stark_inputs_a", stark_inputs.define_a);
    ctx.insert("define_stark_inputs_b", stark_inputs.define_b);
    ctx.insert("assign_stark_inputs_a", stark_inputs.assign_a);
    ctx.insert("assign_stark_inputs_b", stark_inputs.assign_b);

    render(RECURSER_TMPL, &ctx)
}
