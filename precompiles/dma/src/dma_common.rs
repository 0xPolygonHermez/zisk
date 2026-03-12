use fields::PrimeField64;
use zisk_pil::{
    Dma64AlignedInputCpyTrace, Dma64AlignedMemCpyTrace, Dma64AlignedMemSetTrace,
    Dma64AlignedMemTrace, Dma64AlignedTrace, DmaInputCpyTrace, DmaMemCpyTrace,
    DmaPrePostInputCpyTrace, DmaPrePostMemCpyTrace, DmaPrePostTrace, DmaTrace, DmaUnalignedTrace,
};

pub fn get_dma_air_name<F: PrimeField64>(air_id: usize) -> &'static str {
    match air_id {
        DmaTrace::<F>::AIR_ID => "Dma",
        DmaMemCpyTrace::<F>::AIR_ID => "DmaMemCpy",
        DmaInputCpyTrace::<F>::AIR_ID => "DmaInputCpy",
        DmaPrePostTrace::<F>::AIR_ID => "DmaPrePost",
        DmaPrePostMemCpyTrace::<F>::AIR_ID => "DmaPrePostMemCpy",
        DmaPrePostInputCpyTrace::<F>::AIR_ID => "DmaPrePostInputCpy",
        Dma64AlignedTrace::<F>::AIR_ID => "Dma64Aligned",
        Dma64AlignedMemSetTrace::<F>::AIR_ID => "Dma64AlignedMemSet",
        Dma64AlignedMemCpyTrace::<F>::AIR_ID => "Dma64AlignedMemCpy",
        Dma64AlignedInputCpyTrace::<F>::AIR_ID => "Dma64AlignedInputCpy",
        Dma64AlignedMemTrace::<F>::AIR_ID => "Dma64AlignedMem",
        DmaUnalignedTrace::<F>::AIR_ID => "DmaUnaligned",
        _ => "Unknown",
    }
}

pub fn dma_trace(title: &str, rows: usize, num_rows: usize) {
    tracing::debug!(
        "··· Creating {title} instance [{rows} / {num_rows} rows filled {:.2}%]",
        rows as f64 / num_rows as f64 * 100.0
    );
}

/// Flattens and reorders input vectors to ensure proper sequencing.
///
/// This function reorders vectors so that:
/// - The vector whose first element has `must_be_first()` == true is placed first
/// - The vector whose last element has `must_be_last()` == true is placed last
///
/// This is necessary for DMA operations to maintain proper sequencing when
/// operations span multiple chunks or segments.
///
/// # Type Parameters
/// * `T` - The input type, must implement `DmaInputPosition`
///
/// # Arguments
/// * `inputs` - Slice of vectors containing DMA inputs
///
/// # Returns
/// A flattened vector with references to inputs, properly ordered
pub fn flatten_and_reorder_inputs<T>(inputs: &[Vec<T>]) -> Vec<&T>
where
    T: DmaInputPosition,
{
    if inputs.is_empty() {
        return Vec::new();
    }

    // Find indices of vectors that must be first/last
    let first_idx =
        inputs.iter().position(|vec| vec.first().is_some_and(|input| input.must_be_first()));

    let last_idx =
        inputs.iter().position(|vec| vec.last().is_some_and(|input| input.must_be_last()));

    match (first_idx, last_idx) {
        (None, None) => {
            // No special ordering required, simple flatten
            inputs.iter().flatten().collect()
        }
        (Some(0), None) => {
            // First is already at position 0, simple flatten
            inputs.iter().flatten().collect()
        }
        (Some(f_idx), None) => {
            // Only first needs reordering: move to beginning
            std::iter::once(&inputs[f_idx])
                .chain(inputs[..f_idx].iter())
                .chain(inputs[f_idx + 1..].iter())
                .flatten()
                .collect()
        }
        (None, Some(l_idx)) if l_idx == inputs.len() - 1 => {
            // Last is already at final position, simple flatten
            inputs.iter().flatten().collect()
        }
        (None, Some(l_idx)) => {
            // Only last needs reordering: move to end
            inputs[..l_idx]
                .iter()
                .chain(inputs[l_idx + 1..].iter())
                .chain(std::iter::once(&inputs[l_idx]))
                .flatten()
                .collect()
        }
        (Some(f_idx), Some(l_idx)) if f_idx == l_idx => {
            // Same vector is both first and last: all constrained inputs belong to one
            // large ("huge") DMA operation that spans from its first to its last element.
            // The only case in which this can happen is when there is a single collector and,
            // therefore, the length of the collector’s input list is 1. Within a collector,
            // the number of inputs does not necessarily have to be 1.
            assert!(f_idx == 0);
            assert!(inputs.len() == 1);
            inputs.iter().flatten().collect()
        }
        (Some(f_idx), Some(l_idx)) if f_idx == 0 && l_idx == inputs.len() - 1 => {
            // Already in correct order
            inputs.iter().flatten().collect()
        }
        (Some(f_idx), Some(l_idx)) => {
            // Both need reordering: first at beginning, last at end
            // Handle different cases to avoid double-including indices
            if f_idx < l_idx {
                // first comes before last in original order
                std::iter::once(&inputs[f_idx])
                    .chain(inputs[..f_idx].iter())
                    .chain(inputs[f_idx + 1..l_idx].iter())
                    .chain(inputs[l_idx + 1..].iter())
                    .chain(std::iter::once(&inputs[l_idx]))
                    .flatten()
                    .collect()
            } else {
                // last comes before first in original order
                std::iter::once(&inputs[f_idx])
                    .chain(inputs[..l_idx].iter())
                    .chain(inputs[l_idx + 1..f_idx].iter())
                    .chain(inputs[f_idx + 1..].iter())
                    .chain(std::iter::once(&inputs[l_idx]))
                    .flatten()
                    .collect()
            }
        }
    }
}

/// Trait for types that have a skip_rows field
pub trait DmaInputPosition {
    fn must_be_last(&self) -> bool;
    fn must_be_first(&self) -> bool;
}
