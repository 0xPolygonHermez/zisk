use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use cargo_metadata::camino::Utf8PathBuf;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct AggregationToml {
    /// Guest program names — the full leaf allowlist; order fixes the
    /// `programVKs[]` index of each program.
    programs: Vec<String>,
    /// `AggregatePublics` circom body, relative to this TOML.
    aggregate_publics: PathBuf,
    #[serde(default)]
    normalize: Vec<NormalizeToml>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct NormalizeToml {
    /// `NormalizePublics` circom body, relative to this TOML.
    template: PathBuf,
    #[serde(default)]
    free_inputs: usize,
    /// Subset of the top-level `programs` this group covers.
    programs: Vec<String>,
}

/// Fully-resolved definition: circuit bodies inlined, member ELFs pinned by
/// path and content hash.
#[derive(Debug)]
pub struct ResolvedAggregation {
    pub name: String,
    pub programs: Vec<ResolvedProgram>,
    pub aggregate_publics_body: String,
    pub normalize_groups: Vec<ResolvedNormalizeGroup>,
}

#[derive(Debug)]
pub struct ResolvedProgram {
    pub name: String,
    pub elf_path: String,
    pub elf_blake3: String,
}

#[derive(Debug)]
pub struct ResolvedNormalizeGroup {
    pub member_indices: Vec<usize>,
    pub body: String,
    pub n_free_inputs: usize,
}

/// Discover and process every `aggregations/*.toml` under `programs_dir`.
/// `built` is the (name, elf path) list of guests built in this pass.
pub(crate) fn process_aggregations(
    programs_dir: &Path,
    built: &[(String, Utf8PathBuf)],
) -> Result<()> {
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
        process_definition(&path, built, &out_dir)
            .with_context(|| format!("aggregation definition {}", path.display()))?;
    }
    Ok(())
}

fn process_definition(
    toml_path: &Path,
    built: &[(String, Utf8PathBuf)],
    out_dir: &Path,
) -> Result<()> {
    println!("cargo:rerun-if-changed={}", toml_path.display());
    let (resolved, paths) = resolve_aggregation(toml_path, built)?;
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
    /// One per normalize group, in group order.
    pub normalize: Vec<PathBuf>,
}

impl ResolvedCircuitPaths {
    fn circuit_paths(&self) -> impl Iterator<Item = &PathBuf> {
        std::iter::once(&self.aggregate).chain(self.normalize.iter())
    }
}

/// Name → ELF map for the guest programs under `programs_dir`, without
/// building them — the CLI-side counterpart of the map `build_program`
/// passes after a build. `release` picks the guest profile subdir.
pub fn guest_elf_map(programs_dir: &Path, release: bool) -> Result<Vec<(String, Utf8PathBuf)>> {
    let metadata_file = programs_dir.join("Cargo.toml");
    let mut cmd = cargo_metadata::MetadataCommand::new();
    let metadata = cmd
        .manifest_path(&metadata_file)
        .exec()
        .with_context(|| format!("Failed to read guest metadata at {}", metadata_file.display()))?;
    let args = crate::BuildArgs { release, ..Default::default() };
    crate::build::generate_elf_paths(&metadata, Some(&args))
}

/// Parse and resolve a definition TOML against a guest name → ELF map.
/// The single resolver behind both the build pipeline and the CLI, so the
/// schema and its validation cannot diverge.
pub fn resolve_aggregation(
    toml_path: &Path,
    built: &[(String, Utf8PathBuf)],
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
    if def.programs.is_empty() {
        bail!("`programs` must not be empty");
    }

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

    // Resolve the allowlist against the guests built in this pass.
    let programs = def
        .programs
        .iter()
        .map(|prog_name| {
            let (_, elf_path) = built.iter().find(|(n, _)| n == prog_name).with_context(|| {
                let known: Vec<&str> = built.iter().map(|(n, _)| n.as_str()).collect();
                format!("references unknown guest program {prog_name:?}; built: {known:?}")
            })?;
            let elf_bytes = fs::read(elf_path).with_context(|| {
                format!("Failed to read ELF {elf_path} (are the guest programs built for this profile?)")
            })?;
            Ok(ResolvedProgram {
                name: prog_name.clone(),
                elf_path: elf_path.to_string(),
                elf_blake3: blake3::hash(&elf_bytes).to_hex().to_string(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let mut covered = vec![false; programs.len()];
    let mut groups: Vec<(PathBuf, ResolvedNormalizeGroup)> = Vec::new();
    for (g, entry) in def.normalize.iter().enumerate() {
        if entry.programs.is_empty() {
            bail!("normalize group {g} has no member programs");
        }
        let member_indices = entry
            .programs
            .iter()
            .map(|prog_name| {
                let idx = def.programs.iter().position(|p| p == prog_name).with_context(|| {
                    format!(
                        "normalize group {g} references {prog_name:?}, which is not an \
                             entry of `programs`"
                    )
                })?;
                if covered[idx] {
                    bail!("program {prog_name:?} appears in more than one normalize group");
                }
                covered[idx] = true;
                Ok(idx)
            })
            .collect::<Result<Vec<_>>>()?;

        let (path, body) = read_circuit(&entry.template)?;
        expect_template_decl(&body, "NormalizePublics", &path)?;
        groups.push((
            path,
            ResolvedNormalizeGroup { member_indices, body, n_free_inputs: entry.free_inputs },
        ));
    }

    let (aggregate_path, aggregate_publics_body) = read_circuit(&def.aggregate_publics)?;
    expect_template_decl(&aggregate_publics_body, "AggregatePublics", &aggregate_path)?;

    let (normalize_paths, normalize_groups) = groups.into_iter().unzip();
    Ok((
        ResolvedAggregation { name, programs, aggregate_publics_body, normalize_groups },
        ResolvedCircuitPaths { aggregate: aggregate_path, normalize: normalize_paths },
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
    normalize_paths: &[PathBuf],
) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    let _ = writeln!(out, "// @generated by zisk-build from {}. Do not edit.", toml_path.display());
    let _ = writeln!(out, "{{");
    let _ = writeln!(
        out,
        "    static __ZISK_AGG_PROGRAMS: [::zisk_sdk::GuestProgram; {}] = [",
        resolved.programs.len()
    );
    for p in &resolved.programs {
        let _ = writeln!(out, "        ::zisk_sdk::GuestProgram {{");
        let _ = writeln!(
            out,
            "            program_id: ::zisk_sdk::ProgramId::new_static({:?}, {:?}),",
            p.name, p.elf_blake3
        );
        let _ = writeln!(
            out,
            "            elf: ::zisk_sdk::Elf::from_embedded(include_bytes!({:?})),",
            p.elf_path
        );
        let _ = writeln!(out, "        }},");
    }
    let _ = writeln!(out, "    ];");

    let all_refs: Vec<String> =
        (0..resolved.programs.len()).map(|i| format!("&__ZISK_AGG_PROGRAMS[{i}]")).collect();
    let _ = writeln!(out, "    ::zisk_sdk::AggregationProgramBuilder::new(");
    let _ = writeln!(out, "        &[{}],", all_refs.join(", "));
    let _ = writeln!(
        out,
        "        ::zisk_sdk::CircomCircuit::new_static({:?}, include_str!({:?})),",
        format!("{}/aggregate_publics", resolved.name),
        aggregate_path.display().to_string(),
    );
    let _ = writeln!(out, "    )");
    for (g, (path, group)) in normalize_paths.iter().zip(&resolved.normalize_groups).enumerate() {
        let member_refs: Vec<String> =
            group.member_indices.iter().map(|i| format!("&__ZISK_AGG_PROGRAMS[{i}]")).collect();
        let _ = writeln!(out, "    .normalize_with(");
        let _ = writeln!(out, "        &[{}],", member_refs.join(", "));
        let _ = writeln!(
            out,
            "        ::zisk_sdk::CircomCircuit::new_static({:?}, include_str!({:?})),",
            format!("{}/normalize_{g}", resolved.name),
            path.display().to_string(),
        );
        let _ = writeln!(out, "        {}usize,", group.n_free_inputs);
        let _ = writeln!(out, "    )");
    }
    let _ = writeln!(out, "}}");
    out
}
