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

/// Reserved public slot for `is_vadcop_final_proof` (1 = raw leaf, 0 =
/// aggregator output). It is a `signal output` circom orders ahead of the input
/// publics, and is stripped in later layers so it never reaches publics.json.
pub const IS_VADCOP_FINAL_SLOT: usize = 0;

/// ZisK's fixed user-publics width (64), from the canonical `zisk-verifier`
/// constants. Re-exported so callers can size the recurser layout without a
/// second dependency.
pub use zisk_verifier::ZISK_PUBLICS;

// aggregator.circom.tera hardcodes VK_BASE = 1, assuming the flag sits in
// slot 0; a nonzero value would alias a programVK/user-publics slot. Pin the
// coupling so retuning the const fails the build.
const _: () = assert!(
    IS_VADCOP_FINAL_SLOT == 0,
    "aggregator.circom.tera hardcodes VK_BASE = 1; IS_VADCOP_FINAL_SLOT must be 0"
);

/// The optional single normalization circuit applied to every leaf.
#[derive(Debug, Clone)]
pub struct NormalizeCircuit {
    /// Circom body: `template NormalizePublics([nFreeInputs])`.
    pub body: String,
}

/// Circom bodies injected verbatim into the recurser. `AggregatePublics`
/// asserts the caller's consistency constraints and produces the merged
/// publics; the optional normalize circuit applies to every leaf proof.
#[derive(Debug, Clone)]
pub struct CircomTemplates {
    /// Optional single normalize circuit (applies to all leaves).
    pub normalize: Option<NormalizeCircuit>,
    pub aggregate_publics: String,
    /// Leaf allow-list: unique 4-limb program VKs baked into the circuit; a leaf whose programVK
    /// is absent makes it unsatisfiable. Empty accepts any program (the SDK makes that explicit).
    /// Order fixes each index, fed to a NormalizePublics that declares `programIndex`.
    pub program_vks: Vec<[String; 4]>,
    /// Unified free-value width per side: NormalizePublics consumes `n_free`
    /// free inputs and emits `n_free` free outputs, which feed AggregatePublics.
    pub n_free: usize,
    /// Publics slots the aggregation populates. `AggregatePublics` outputs a
    /// `n_publics_agg`-wide array; the generator zero-fills the
    /// `[n_publics_agg, ZISK_PUBLICS())` tail. Must be in `1..=ZISK_PUBLICS()`.
    pub n_publics_agg: usize,
}

impl CircomTemplates {
    /// The unified free-value width (0 = no free inputs/outputs).
    pub fn n_free(&self) -> usize {
        self.n_free
    }

    /// Structural checks on the user circuits and the allow-list. Run by [`gen_recurser`] and
    /// by the SDK builder, so a bad definition fails before a `recurser_id` is handed out.
    pub fn validate(&self) -> Result<()> {
        let n_publics_agg = self.n_publics_agg;
        if n_publics_agg == 0 || n_publics_agg > ZISK_PUBLICS {
            return Err(RecurserError::InvalidTemplates(format!(
                "n_publics_agg must be in 1..={ZISK_PUBLICS}, got {n_publics_agg}"
            )));
        }
        expect_template_arity(&self.aggregate_publics, "AggregatePublics", self.n_free)?;
        if let Some(norm) = &self.normalize {
            expect_template_arity(&norm.body, "NormalizePublics", self.n_free)?;
        }

        // The allow-list index is a sum over the matches: only defined when each key appears once.
        let mut keys: Vec<[u64; 4]> = Vec::with_capacity(self.program_vks.len());
        for (i, vk) in self.program_vks.iter().enumerate() {
            let key = canonical_vk(vk).ok_or_else(|| {
                RecurserError::InvalidTemplates(format!(
                    "program VK {i} is not 4 canonical limbs: {vk:?}"
                ))
            })?;
            if let Some(j) = keys.iter().position(|other| *other == key) {
                return Err(RecurserError::InvalidTemplates(format!(
                    "duplicate program VK at allow-list index {i} (already at {j})"
                )));
            }
            keys.push(key);
        }

        let declared = self.program_index();
        if declared && self.program_vks.is_empty() {
            return Err(RecurserError::InvalidTemplates(
                "`NormalizePublics` declares `programIndex` but there is no program allow-list to index".into(),
            ));
        }
        if !declared && self.program_vks.len() >= 2 && self.normalize.is_some() {
            return Err(RecurserError::InvalidTemplates(
                "with two or more allowed programs `NormalizePublics` must declare \
                 `signal input programIndex` (the leaf program's position in the allow-list)"
                    .into(),
            ));
        }
        Ok(())
    }

    /// Whether NormalizePublics takes the leaf's allow-list position (required with 2+ programs).
    fn program_index(&self) -> bool {
        self.normalize.as_ref().is_some_and(|n| declares_input(&n.body, "programIndex"))
    }
}

/// A program VK as Goldilocks field elements, so "01" and "1" are the same key.
fn canonical_vk(vk: &[String; 4]) -> Option<[u64; 4]> {
    const GOLDILOCKS_P: u64 = 0xFFFF_FFFF_0000_0001;
    let mut out = [0u64; 4];
    for (o, limb) in out.iter_mut().zip(vk) {
        *o = limb.trim().parse::<u64>().ok().filter(|v| *v < GOLDILOCKS_P)?;
    }
    Some(out)
}

/// Assert the user circom body declares `template <template>(...)` with the
/// arity the scaffolding instantiates it with:
///
/// - `NormalizePublics()` or `NormalizePublics(nFreeInputs)` (no publics-width
///   param; it sizes off `ZISK_PUBLICS()`). With `n_free > 0` it MUST emit a
///   `free_outputs` output feeding AggregatePublics.
/// - `AggregatePublics(nPublicsAgg)` or `AggregatePublics(nFreeInputs, nPublicsAgg)`.
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
    // AggregatePublics carries a leading `nPublicsAgg` param; NormalizePublics
    // sizes off ZISK_PUBLICS() and takes no publics-width param.
    let base_params = if template == "AggregatePublics" { 1 } else { 0 };
    let expected = base_params + if n_free > 0 { 1 } else { 0 };
    if params.len() != expected {
        return Err(RecurserError::InvalidTemplates(format!(
            "`{template}` declares {} params but n_free={n_free} requires {expected}",
            params.len()
        )));
    }
    // n_free > 0 requires a `free_outputs` output (it feeds AggregatePublics).
    if template == "NormalizePublics" && n_free > 0 && !declares_free_outputs(body) {
        return Err(RecurserError::InvalidTemplates(format!(
            "`{template}` with n_free={n_free} must declare `signal output free_outputs[...]`"
        )));
    }
    Ok(())
}

/// Structural check: does the body declare the scalar `signal input <name>` (comments ignored)?
fn declares_input(body: &str, name: &str) -> bool {
    strip_comments(body).split(';').any(|stmt| {
        let Some(rest) = stmt.trim().strip_prefix("signal") else { return false };
        let Some(rest) = rest.trim_start().strip_prefix("input") else { return false };
        // An optional tag list, then comma-separated names.
        let rest = rest.trim_start();
        let rest =
            if rest.starts_with('{') { rest.split_once('}').map_or("", |(_, r)| r) } else { rest };
        rest.split(',').any(|decl| decl.trim() == name)
    })
}

/// The body with `//` and `/* */` comments removed.
fn strip_comments(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(i) = rest.find('/') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if tail.starts_with("//") {
            rest = tail.find('\n').map_or("", |j| &tail[j..]);
        } else if tail.starts_with("/*") {
            rest = tail.find("*/").map_or("", |j| &tail[j + 2..]);
        } else {
            out.push('/');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

/// Structural check: does the body declare a `free_outputs` output signal?
/// Matches `signal output free_outputs` allowing arbitrary internal spacing.
fn declares_free_outputs(body: &str) -> bool {
    body.split("signal").any(|seg| {
        let seg = seg.trim_start();
        seg.starts_with("output") && seg["output".len()..].trim_start().starts_with("free_outputs")
    })
}

/// Split a circuit body's `include` lines from the rest. The body is spliced after the template's
/// own definitions and circom only accepts `include` before any, so they move to the file header:
/// a circuit names what it calls (e.g. `hash/poseidon1/poseidon.circom`) instead of relying on the
/// verifier of whichever hash family the key was built with to pull it in.
fn split_includes(body: &str) -> (Vec<String>, String) {
    let mut includes = Vec::new();
    let mut rest = String::with_capacity(body.len());
    for line in body.lines() {
        if line.trim_start().starts_with("include ") {
            includes.push(line.trim().to_string());
        } else {
            rest.push_str(line);
            rest.push('\n');
        }
    }
    (includes, rest)
}

pub fn gen_recurser(
    verifier_filename: &str,
    zisk_vk: &[String],
    stark_inputs: &StarkInputBlocks<'_>,
    templates: &CircomTemplates,
) -> Result<String> {
    templates.validate()?;
    let n_free = templates.n_free();
    let n_publics_agg = templates.n_publics_agg;
    let program_index = templates.program_index();

    let (mut includes, aggregate_body) = split_includes(&templates.aggregate_publics);
    let normalize_body = templates.normalize.as_ref().map(|norm| {
        let (inc, body) = split_includes(&norm.body);
        includes.extend(inc);
        body
    });
    includes.dedup();

    let mut ctx = Context::new();
    ctx.insert("verifier_filename", verifier_filename);
    ctx.insert("circuit_includes", &includes);
    ctx.insert("is_vadcop_final_slot", &IS_VADCOP_FINAL_SLOT);
    ctx.insert("root_c_vadcop_final_zisk", &zisk_vk);
    ctx.insert("aggregate_publics_template", &aggregate_body);
    ctx.insert("n_free", &n_free);
    ctx.insert("zisk_publics", &ZISK_PUBLICS);
    ctx.insert("n_publics_agg", &n_publics_agg);
    ctx.insert("n_programs", &templates.program_vks.len());
    ctx.insert("program_vks", &templates.program_vks);
    ctx.insert("program_index", &program_index);
    match &normalize_body {
        Some(body) => {
            ctx.insert("normalize", &true);
            ctx.insert("normalize_body", body);
        }
        None => ctx.insert("normalize", &false),
    }
    ctx.insert("define_stark_inputs_a", stark_inputs.define_a);
    ctx.insert("define_stark_inputs_b", stark_inputs.define_b);
    ctx.insert("assign_stark_inputs_a", stark_inputs.assign_a);
    ctx.insert("assign_stark_inputs_b", stark_inputs.assign_b);

    render(RECURSER_TMPL, &ctx)
}

#[cfg(test)]
mod tests {
    use super::{declares_input, split_includes};

    #[test]
    fn declares_input_matches_a_whole_scalar_signal_name() {
        assert!(declares_input("signal input programIndex;", "programIndex"));
        assert!(declares_input("signal input a, programIndex;", "programIndex"));
        assert!(declares_input("signal input {binary} programIndex;", "programIndex"));
        assert!(!declares_input("signal input programIndexes;", "programIndex"));
        assert!(!declares_input("signal input programIndex[2];", "programIndex"));
        assert!(!declares_input("// signal input programIndex;", "programIndex"));
        assert!(!declares_input("/* signal input programIndex; */", "programIndex"));
        assert!(!declares_input("signal output programIndex;", "programIndex"));
    }

    #[test]
    fn body_includes_are_split_off_for_the_header() {
        let (inc, rest) = split_includes(
            "include \"hash/poseidon1/poseidon.circom\";\n\ntemplate T() {\n    signal input a;\n}\n",
        );
        assert_eq!(inc, vec!["include \"hash/poseidon1/poseidon.circom\";".to_string()]);
        assert!(!rest.contains("include"), "{rest}");
        assert!(rest.contains("template T()"), "{rest}");
    }
}
