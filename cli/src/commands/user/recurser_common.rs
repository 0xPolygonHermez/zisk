use std::path::Path;

use anyhow::{anyhow, Context, Result};
use zisk_build::resolve_aggregation;
use zisk_sdk::{AggregationProgramBuilder, CircomCircuit, Recurser};

/// Resolve a `programs/aggregations/<name>.toml` into a [`Recurser`] at runtime —
/// the CLI sibling of the compile-time `load_aggregation_program!` path. Both
/// derive the same content-addressed `recurser_id` for the same definition.
///
/// Resolution reads only the definition TOML and its circom bodies; no guest
/// build is required.
pub(crate) fn resolve_recurser(aggregation: &Path) -> Result<Recurser> {
    let definition_path = aggregation
        .canonicalize()
        .with_context(|| format!("definition not found: {}", aggregation.display()))?;

    let (definition, _circuit_paths) = resolve_aggregation(&definition_path)
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
