use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct AggregationToml {
    /// `AggregatePublics` circom body, relative to this TOML.
    aggregate_publics: PathBuf,
    /// `NormalizePublics` circom body, relative to this TOML (optional).
    #[serde(default)]
    normalize_publics: Option<PathBuf>,
    /// Free inputs supplied per side at the fold — a single count shared by
    /// the normalize + aggregate circuits.
    #[serde(default)]
    free_inputs: usize,
}

/// Fully-resolved definition: circuit bodies inlined.
#[derive(Debug)]
pub struct ResolvedAggregation {
    pub name: String,
    pub aggregate_publics_body: String,
    pub n_free: usize,
    pub normalize: Option<ResolvedNormalize>,
}

#[derive(Debug)]
pub struct ResolvedNormalize {
    pub body: String,
}

/// Discover and process every `aggregations/*.toml` under `programs_dir`.
pub(crate) fn process_aggregations(programs_dir: &Path) -> Result<()> {
    let agg_dir = programs_dir.join("aggregations");
    if !agg_dir.is_dir() {
        // No rerun-if-changed here: a missing path would force the script
        // (and the guest cargo build) to rerun on every host build. The cost:
        // creating this dir for the first time needs one manual rebuild
        // trigger (e.g. touch build.rs); afterwards it's tracked.
        return Ok(());
    }
    println!("cargo:rerun-if-changed={}", agg_dir.display());

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").context("OUT_DIR is not set")?)
        .join("zisk_aggregations");
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("Failed to create {}", out_dir.display()))?;

    for entry in fs::read_dir(&agg_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        process_definition(&path, &out_dir)
            .with_context(|| format!("aggregation definition {}", path.display()))?;
    }
    Ok(())
}

fn process_definition(toml_path: &Path, out_dir: &Path) -> Result<()> {
    println!("cargo:rerun-if-changed={}", toml_path.display());
    let (resolved, paths) = resolve_aggregation(toml_path)?;
    for path in paths.circuit_paths() {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    // Generated builder expression — `load_aggregation_program!`'s input.
    let rs_path = out_dir.join(format!("{}.rs", resolved.name));
    fs::write(&rs_path, codegen(&resolved, toml_path, &paths.aggregate, &paths.normalize))
        .with_context(|| format!("Failed to write {}", rs_path.display()))?;
    println!("cargo:rustc-env=ZISK_AGG_{}={}", resolved.name, rs_path.display());
    Ok(())
}

/// Source paths of the circuits a definition resolved to (for cargo
/// rerun-if-changed and codegen `include_str!`).
pub struct ResolvedCircuitPaths {
    pub aggregate: PathBuf,
    pub normalize: Option<PathBuf>,
}

impl ResolvedCircuitPaths {
    fn circuit_paths(&self) -> impl Iterator<Item = &PathBuf> {
        std::iter::once(&self.aggregate).chain(self.normalize.iter())
    }
}

/// Parse and resolve a definition TOML into inlined circuit bodies.
/// The single resolver behind both the build pipeline and the CLI, so the
/// schema and its validation cannot diverge.
pub fn resolve_aggregation(
    toml_path: &Path,
) -> Result<(ResolvedAggregation, ResolvedCircuitPaths)> {
    let name =
        toml_path.file_stem().and_then(|s| s.to_str()).context("non-UTF-8 file name")?.to_string();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        bail!("name {name:?} must be a valid identifier ([A-Za-z0-9_]+; it names env vars)");
    }

    let def: AggregationToml = toml::from_str(
        &fs::read_to_string(toml_path)
            .with_context(|| format!("Failed to read {}", toml_path.display()))?,
    )?;

    let base = toml_path.parent().unwrap_or_else(|| Path::new("."));
    let read_circuit = |rel: &Path| -> Result<(PathBuf, String)> {
        let path = if rel.is_absolute() { rel.to_path_buf() } else { base.join(rel) };
        let path = path
            .canonicalize()
            .with_context(|| format!("circuit not found: {}", path.display()))?;
        let body = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        Ok((path, body))
    };

    let normalize = def
        .normalize_publics
        .map(|rel| -> Result<(PathBuf, ResolvedNormalize)> {
            let (path, body) = read_circuit(&rel)?;
            expect_template_decl(&body, "NormalizePublics", &path)?;
            Ok((path, ResolvedNormalize { body }))
        })
        .transpose()?;

    let (aggregate_path, aggregate_publics_body) = read_circuit(&def.aggregate_publics)?;
    expect_template_decl(&aggregate_publics_body, "AggregatePublics", &aggregate_path)?;

    let (normalize_path, normalize_resolved) =
        normalize.map(|(p, r)| (Some(p), Some(r))).unwrap_or((None, None));
    Ok((
        ResolvedAggregation {
            name,
            aggregate_publics_body,
            n_free: def.free_inputs,
            normalize: normalize_resolved,
        },
        ResolvedCircuitPaths { aggregate: aggregate_path, normalize: normalize_path },
    ))
}

fn expect_template_decl(body: &str, template: &str, path: &Path) -> Result<()> {
    let needle = format!("template {template}(");
    match body.matches(&needle).count() {
        1 => Ok(()),
        n => bail!(
            "{} must define `template {template}(...)` exactly once, found {n}",
            path.display()
        ),
    }
}

fn codegen(
    resolved: &ResolvedAggregation,
    toml_path: &Path,
    aggregate_path: &Path,
    normalize_path: &Option<PathBuf>,
) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    let _ = writeln!(out, "// @generated by zisk-build from {}. Do not edit.", toml_path.display());
    let _ = writeln!(out, "{{");
    let _ = writeln!(out, "    ::zisk_sdk::AggregationProgramBuilder::new(");
    let _ = writeln!(
        out,
        "        ::zisk_sdk::CircomCircuit::new_static({:?}, include_str!({:?})),",
        format!("{}/aggregate_publics", resolved.name),
        aggregate_path.display().to_string(),
    );
    let _ = writeln!(out, "    )");
    if resolved.n_free > 0 {
        let _ = writeln!(out, "    .free_inputs({}usize)", resolved.n_free);
    }
    if let (Some(path), Some(_norm)) = (normalize_path, &resolved.normalize) {
        let _ = writeln!(out, "    .normalize(");
        let _ = writeln!(
            out,
            "        ::zisk_sdk::CircomCircuit::new_static({:?}, include_str!({:?})),",
            format!("{}/normalize", resolved.name),
            path.display().to_string(),
        );
        let _ = writeln!(out, "    )");
    }
    let _ = writeln!(out, "}}");
    out
}
