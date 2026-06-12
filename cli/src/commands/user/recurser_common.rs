use std::path::Path;

use anyhow::{anyhow, Context, Result};
use zisk_build::{guest_elf_map, resolve_aggregation};
use zisk_sdk::{AggregationProgramBuilder, CircomCircuit, GuestProgram, Recurser};

/// Resolve a `programs/aggregations/<name>.toml` into a [`Recurser`] at runtime —
/// the CLI sibling of the compile-time `load_aggregation_program!` path. Both
/// derive the same content-addressed `recurser_id` for the same definition.
///
/// The referenced guest programs must already be built (`cargo build` of the
/// host crate); `release` selects which profile's ELFs to resolve.
pub(crate) fn resolve_recurser(aggregation: &Path, release: bool) -> Result<Recurser> {
    // The definition lives at `<programs>/aggregations/<name>.toml`, so
    // the guest workspace is two levels up.
    let definition_path = aggregation
        .canonicalize()
        .with_context(|| format!("definition not found: {}", aggregation.display()))?;
    let programs_dir = definition_path
        .parent()
        .and_then(|aggregations| aggregations.parent())
        .context("definition must live under <programs>/aggregations/")?
        .to_path_buf();

    let elf_map = guest_elf_map(&programs_dir, release)?;
    let (definition, _circuit_paths) = resolve_aggregation(&definition_path, &elf_map)
        .with_context(|| format!("aggregation definition {}", aggregation.display()))?;

    let guests: Vec<GuestProgram> = definition
        .programs
        .iter()
        .map(|p| GuestProgram::from_uri(&p.elf_path))
        .collect::<Result<_>>()?;
    let guest_refs: Vec<&GuestProgram> = guests.iter().collect();

    let mut builder = AggregationProgramBuilder::new(
        &guest_refs,
        CircomCircuit::from_source(
            format!("{}-aggregate_publics", definition.name),
            definition.aggregate_publics_body.clone(),
        ),
    );
    for (i, group) in definition.normalize_groups.iter().enumerate() {
        let members: Vec<&GuestProgram> = group
            .member_indices
            .iter()
            .map(|&idx| {
                guest_refs.get(idx).copied().ok_or_else(|| {
                    anyhow!("normalize group {i} references program index {idx} out of range")
                })
            })
            .collect::<Result<_>>()?;
        builder = builder.normalize_with(
            &members,
            CircomCircuit::from_source(
                format!("{}-normalize-{i}", definition.name),
                group.body.clone(),
            ),
            group.n_free_inputs,
        );
    }
    builder.build()
}

/// Parse comma-separated decimal u64s ("" / absent → empty).
pub(crate) fn parse_free_inputs(s: Option<&str>) -> Result<Vec<u64>> {
    match s {
        Some(s) if !s.trim().is_empty() => s
            .split(',')
            .map(|x| {
                x.trim().parse::<u64>().map_err(|e| anyhow!("invalid free input '{x}': {e}"))
            })
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
