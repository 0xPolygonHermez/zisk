#[allow(dead_code)]
fn format_u64_hex(value: u64) -> String {
    let hex_str = format!("{value:016x}");
    hex_str
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("_")
}

#[derive(Debug, Clone)]
pub struct MemAlignInput {
    pub addr: u64, // byte address (33 bits)
    pub is_write: bool,
    pub width: u8,
    pub step: u64,
    pub value: u64,           // value to write
    pub mem_values: [u64; 2], // values read from memory
}

/// One memory access as the fill reads it. Collectors do not store this: they store the packed
/// [`MemInputs`], and this is what its iterator decodes, one access at a time.
#[derive(Debug, Clone, Copy)]
pub struct MemInput {
    pub addr: u32,      // address in word native format means byte_address / MEM_BYTES
    pub is_write: bool, // it's a write operation
    pub step: u64,      // mem_step = f(main_step, main_step_offset)
    /// The value read or written. A read of a [`MemInputs`] that does not keep read values decodes
    /// with `0` here: the fill derives the real value (see [`MemInputs`]).
    pub value: u64,
}

// A packed access: `addr_w` in bits 0-31, the write flag in bit 32, and the step relative to the
// collection's base step in bits 33-63.
const HDR_WR_SHIFT: u32 = 32;
const HDR_STEP_SHIFT: u32 = 33;
const HDR_STEP_MAX: u64 = (1u64 << (64 - HDR_STEP_SHIFT)) - 1;
const HDR_ADDR_MASK: u64 = (1u64 << HDR_WR_SHIFT) - 1;

/// How far below its first step a collection sets its base. A chunk's accesses arrive in step
/// order except for the segment's initial data, written at `MEMORY_INIT_STEP` by
/// `init_with_mem_sections` before the chunk's first access, so the base must leave room below.
/// A chunk spans a few million memory steps; this margin and the 31-bit field are far beyond it.
const STEP_BASE_MARGIN: u64 = 1 << 24;

/// No access pushed yet: the base is set by the first one.
const NO_BASE: u64 = u64::MAX;

/// One chunk's memory accesses for one instance, in collection order, packed.
///
/// - `hdr`: one word per access, `addr_w | wr << 32 | (step - base_step) << 33`.
/// - `values`: the value of every access that carries one, in the same order. A write always
///   carries its value. A read carries it only when `keep_read_values` is set.
///
/// A read of the mutable memory (`Mem`, the RAM) carries no value: `mem.pil` forces it to equal the
/// value of the previous lane of the same address, or `0` when the address changes, or the
/// previous segment's value on the segment's first lane. The fill already walks each address's
/// accesses in step order, so it derives the value instead of storing it. A read then takes 8
/// bytes instead of the 24 of an unpacked [`MemInput`], and a write 16.
///
/// `RomData` and `InputData` keep their read values: there the value is the data itself.
#[derive(Debug)]
pub struct MemInputs {
    base_step: u64,
    hdr: Vec<u64>,
    values: Vec<u64>,
    keep_read_values: bool,
}

impl MemInputs {
    /// An empty collection with room for `ops` accesses, `values` of which carry a value. Large
    /// buffers go to lent memory when there is some ([`zisk_common::lent_memory::lent_vec`]).
    pub fn with_capacity(ops: usize, values: usize, keep_read_values: bool) -> Self {
        Self {
            base_step: NO_BASE,
            hdr: zisk_common::lent_memory::lent_vec(ops),
            values: zisk_common::lent_memory::lent_vec(values),
            keep_read_values,
        }
    }

    /// A collection holding `inputs`, in their order. For the paths that build the accesses
    /// themselves (the legacy sort, tests).
    pub fn from_inputs(inputs: impl IntoIterator<Item = MemInput>, keep_read_values: bool) -> Self {
        let inputs = inputs.into_iter();
        let mut out = Self::with_capacity(inputs.size_hint().0, 0, keep_read_values);
        for input in inputs {
            out.push(input.addr, input.is_write, input.step, input.value);
        }
        out
    }

    /// Appends one access. `value` is dropped for a read unless the collection keeps read values.
    ///
    /// # Panics
    /// If `step` is out of the 31-bit range around the collection's base. A chunk's accesses are
    /// orders of magnitude inside it, so this means a caller mixed steps of different chunks.
    #[inline(always)]
    pub fn push(&mut self, addr_w: u32, is_write: bool, step: u64, value: u64) {
        if self.base_step == NO_BASE {
            self.base_step = step.saturating_sub(STEP_BASE_MARGIN);
        }
        let rel = step.wrapping_sub(self.base_step);
        assert!(
            rel <= HDR_STEP_MAX,
            "MemInputs: step {step} does not fit around the base step {}",
            self.base_step
        );
        self.hdr.push(addr_w as u64 | (is_write as u64) << HDR_WR_SHIFT | rel << HDR_STEP_SHIFT);
        if is_write || self.keep_read_values {
            self.values.push(value);
        }
    }

    /// Accesses in the collection.
    #[inline]
    pub fn len(&self) -> usize {
        self.hdr.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.hdr.is_empty()
    }

    /// Whether reads carry their value. When `false`, reads decode with value `0` and the fill
    /// derives the real one.
    #[inline]
    pub fn keeps_read_values(&self) -> bool {
        self.keep_read_values
    }

    /// Heap bytes held: `(used, reserved)`, where `used` counts the stored words and `reserved`
    /// the capacity behind them.
    pub fn heap_bytes(&self) -> (usize, usize) {
        let word = std::mem::size_of::<u64>();
        (
            (self.hdr.len() + self.values.len()) * word,
            (self.hdr.capacity() + self.values.capacity()) * word,
        )
    }

    /// The accesses in collection order, decoded.
    #[inline]
    pub fn iter(&self) -> MemInputsIter<'_> {
        MemInputsIter {
            base_step: self.base_step,
            hdr: self.hdr.iter(),
            values: self.values.iter(),
            keep_read_values: self.keep_read_values,
        }
    }
}

/// Decoding iterator over a [`MemInputs`].
#[derive(Clone)]
pub struct MemInputsIter<'a> {
    base_step: u64,
    hdr: std::slice::Iter<'a, u64>,
    values: std::slice::Iter<'a, u64>,
    keep_read_values: bool,
}

impl Iterator for MemInputsIter<'_> {
    type Item = MemInput;

    #[inline(always)]
    fn next(&mut self) -> Option<MemInput> {
        let hdr = *self.hdr.next()?;
        let is_write = (hdr >> HDR_WR_SHIFT) & 1 == 1;
        let value = if is_write || self.keep_read_values {
            *self.values.next().expect("MemInputs: an access that carries a value has none")
        } else {
            0
        };
        Some(MemInput {
            addr: (hdr & HDR_ADDR_MASK) as u32,
            is_write,
            step: self.base_step + (hdr >> HDR_STEP_SHIFT),
            value,
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.hdr.size_hint()
    }
}

impl ExactSizeIterator for MemInputsIter<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(inputs: &MemInputs) -> Vec<(u32, bool, u64, u64)> {
        inputs.iter().map(|i| (i.addr, i.is_write, i.step, i.value)).collect()
    }

    #[test]
    fn round_trips_every_field_when_read_values_are_kept() {
        let ops = [
            (0x1400_0010u32, false, 1_000_000u64, 7u64),
            (0x1400_0010, true, 1_000_004, u64::MAX),
            (0x1FFF_FFFF, false, 1_000_009, 0x8000_0000_0000_0001),
            (u32::MAX, true, 1_000_010, 0),
        ];
        let mut inputs = MemInputs::with_capacity(0, 0, true);
        for (a, w, s, v) in ops {
            inputs.push(a, w, s, v);
        }
        assert_eq!(collect(&inputs), ops.to_vec());
        assert_eq!(inputs.len(), 4);
    }

    #[test]
    fn drops_read_values_and_keeps_write_values() {
        let mut inputs = MemInputs::with_capacity(4, 2, false);
        inputs.push(10, false, 100, 55);
        inputs.push(10, true, 104, 66);
        inputs.push(11, false, 105, 77);
        inputs.push(11, true, 108, 88);
        assert_eq!(
            collect(&inputs),
            vec![
                (10, false, 100, 0),
                (10, true, 104, 66),
                (11, false, 105, 0),
                (11, true, 108, 88)
            ]
        );
        assert_eq!(inputs.values.len(), 2, "only the writes store a value");
        assert_eq!(inputs.heap_bytes(), (6 * 8, 6 * 8), "4 headers and 2 values, all reserved");
    }

    #[test]
    fn init_step_before_the_first_access_still_encodes() {
        // `init_with_mem_sections` writes at MEMORY_INIT_STEP, then the chunk's own accesses
        // follow; a later chunk starts far from step 0, and its accesses only go up.
        let mut chunk0 = MemInputs::with_capacity(0, 0, false);
        chunk0.push(1, true, 3, 9);
        chunk0.push(1, false, 1, 0);
        assert_eq!(collect(&chunk0), vec![(1, true, 3, 9), (1, false, 1, 0)]);

        let first = 4 * (1u64 << 40);
        let mut later = MemInputs::with_capacity(0, 0, false);
        later.push(2, false, first, 0);
        later.push(2, true, first + (1 << 22), 5);
        assert_eq!(collect(&later), vec![(2, false, first, 0), (2, true, first + (1 << 22), 5)]);
    }

    #[test]
    #[should_panic(expected = "does not fit around the base step")]
    fn a_step_far_below_the_base_is_rejected() {
        let mut inputs = MemInputs::with_capacity(0, 0, false);
        inputs.push(1, false, 1 << 40, 0);
        inputs.push(1, false, (1 << 40) - STEP_BASE_MARGIN - 1, 0);
    }

    #[test]
    fn from_inputs_matches_push() {
        let ops = [
            MemInput { addr: 3, is_write: false, step: 10, value: 1 },
            MemInput { addr: 3, is_write: true, step: 12, value: 2 },
        ];
        let kept = MemInputs::from_inputs(ops, true);
        assert_eq!(collect(&kept), vec![(3, false, 10, 1), (3, true, 12, 2)]);
        let derived = MemInputs::from_inputs(ops, false);
        assert_eq!(collect(&derived), vec![(3, false, 10, 0), (3, true, 12, 2)]);
    }
}
