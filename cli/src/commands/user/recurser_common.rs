use std::path::Path;

use anyhow::{anyhow, Context, Result};
use zisk_build::{guest_elf_map, resolve_aggregation};
use zisk_sdk::{AggregationProgramBuilder, CircomCircuit, GuestProgram, Recurser};

/// Resolve a `programs/aggregations/<name>.toml` into a [`Recurser`] at runtime —
/// the CLI sibling of the compile-time `load_aggregation_program!` path. Both
/// derive the same content-addressed `recurser_id` for the same definition.
///
/// Resolution reads the definition TOML and its circom bodies. When the
/// definition declares a `programs` allow-list, it also resolves those guests'
/// ELFs (name → ELF via the guest workspace) and derives their VKs — the guest
/// programs must be built for the active profile. Allow-list-free definitions
/// need no guest build.
///
pub(crate) fn resolve_recurser(aggregation: &Path) -> Result<Recurser> {
    let definition_path = aggregation
        .canonicalize()
        .with_context(|| format!("definition not found: {}", aggregation.display()))?;

    // The definition lives at `<programs>/aggregations/<name>.toml`, so the
    // guest workspace is two levels up. Only needed to resolve an allow-list;
    // try the release profile first, then debug, so either build satisfies it.
    let programs_dir = definition_path
        .parent()
        .and_then(|aggregations| aggregations.parent())
        .context("definition must live under <programs>/aggregations/")?;
    let elf_map =
        guest_elf_map(programs_dir, true).or_else(|_| guest_elf_map(programs_dir, false)).ok();

    let (definition, _circuit_paths) = resolve_aggregation(&definition_path, elf_map.as_deref())
        .with_context(|| format!("aggregation definition {}", aggregation.display()))?;

    let mut builder = AggregationProgramBuilder::new(CircomCircuit::from_source(
        format!("{}-aggregate_publics", definition.name),
        definition.aggregate_publics_body.clone(),
    ))
    .free_inputs(definition.n_free);
    if let Some(norm) = &definition.normalize {
        builder = builder.normalize(CircomCircuit::from_source(
            format!("{}-normalize", definition.name),
            norm.body.clone(),
        ));
    }

    // Materialize allow-list guests from their resolved ELF paths so the builder
    // can derive their VKs (same VKs the codegen path bakes at host-build time).
    let guests: Vec<GuestProgram> = definition
        .programs
        .iter()
        .map(|p| {
            GuestProgram::from_uri(&p.elf_path)
                .with_context(|| format!("failed to load allow-list guest '{}'", p.name))
        })
        .collect::<Result<_>>()?;
    if !guests.is_empty() {
        let refs: Vec<&GuestProgram> = guests.iter().collect();
        builder = builder.programs(&refs);
    }

    Ok(builder.build()?)
}

/// Parse comma-separated decimal u64s ("" / absent → empty).
pub(crate) fn parse_free_inputs(s: Option<&str>) -> Result<Vec<u64>> {
    match s {
        Some(s) if !s.trim().is_empty() => s
            .split(',')
            .map(|x| x.trim().parse::<u64>().map_err(|e| anyhow!("invalid free input '{x}': {e}")))
            .collect(),
        _ => Ok(Vec::new()),
    }
}

/// Parse `rootCRecurserAgg` as exactly 4 comma-separated decimal limbs.
pub(crate) fn parse_root_c(s: &str) -> Result<[u64; 4]> {
    let limbs: Vec<u64> = s
        .split(',')
        .map(|x| x.trim().parse::<u64>().map_err(|e| anyhow!("invalid limb '{x}': {e}")))
        .collect::<Result<_>>()?;
    <[u64; 4]>::try_from(limbs).map_err(|v| anyhow!("expected 4 limbs, got {}", v.len()))
}
