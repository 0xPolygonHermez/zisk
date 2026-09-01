//! Exercises the `mt` DMA family: memcpy/memcmp variants that read their source as it was at a
//! temporal reference rather than as it is now, plus the `execute_advice` hint and the temporal
//! reference request that feed them.
//!
//! The guest asserts every result itself, so a wrong lowering or a wrong emulation surfaces as a
//! failed execution.

#![no_main]
ziskos::entrypoint!(main);

use ziskos::{
    ziskos_execute_advice, ziskos_mtcmp, ziskos_mtcpy, ziskos_temporal_ref,
    ziskos_temporal_snapshot,
};

/// Aligned scratch buffer, so the tests can slice unaligned ranges out of it deliberately
#[repr(align(8))]
struct Aligned<const N: usize>([u8; N]);

fn main() {
    xmtcpy_reads_the_source_as_it_was();
    mtcpy_with_a_register_count();
    mtcpy_on_unaligned_ranges();
    xmtcmp_compares_against_the_snapshot();
    mtcmp_reports_the_first_difference();
    a_reference_survives_several_advised_regions();
    two_references_stay_independent();
    the_reference_can_be_requested_on_its_own();
}

/// The source is overwritten after being advised; xmtcpy (immediate count) must still see the old
/// contents.
fn xmtcpy_reads_the_source_as_it_was() {
    let mut src = Aligned([0u8; 32]);
    let mut dst = Aligned([0u8; 32]);
    for (i, b) in src.0.iter_mut().enumerate() {
        *b = i as u8;
    }

    let tref = ziskos_temporal_snapshot!(&src.0, 32);
    src.0 = [0xFF; 32];

    ziskos_mtcpy!(&mut dst.0, &src.0, 32, tref);

    for (i, b) in dst.0.iter().enumerate() {
        assert_eq!(*b, i as u8);
    }
    // the live source really was clobbered
    assert_eq!(src.0[0], 0xFF);
}

/// Same, with the count in a register, which selects the non-extended `dma_mtcpy`
fn mtcpy_with_a_register_count() {
    let mut src = Aligned([0u8; 32]);
    let mut dst = Aligned([0u8; 32]);
    for (i, b) in src.0.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(3).wrapping_add(1);
    }

    let count = read_count(24);
    let tref = ziskos_temporal_snapshot!(&src.0, count);
    src.0 = [0; 32];

    ziskos_mtcpy!(&mut dst.0, &src.0, count, tref);

    for i in 0..24 {
        assert_eq!(dst.0[i], (i as u8).wrapping_mul(3).wrapping_add(1));
    }
    // past the copied range the destination is untouched
    assert_eq!(dst.0[24], 0);
}

/// Neither end of the range is 64-bit aligned, so the pre/post paths of the DMA are exercised and
/// the bytes around the destination range must survive.
fn mtcpy_on_unaligned_ranges() {
    let mut src = Aligned([0u8; 32]);
    let mut dst = Aligned([0xAAu8; 32]);
    for i in 0..13 {
        src.0[3 + i] = 0x40 + i as u8;
    }

    let count = read_count(13);
    let tref = ziskos_temporal_snapshot!(&src.0[3..], count);
    src.0 = [0; 32];

    ziskos_mtcpy!(&mut dst.0[5..], &src.0[3..], count, tref);

    for i in 0..5 {
        assert_eq!(dst.0[i], 0xAA);
    }
    for i in 0..13 {
        assert_eq!(dst.0[5 + i], 0x40 + i as u8);
    }
    for i in 18..32 {
        assert_eq!(dst.0[i], 0xAA);
    }
}

/// The destination still holds what the source held at the reference, so the comparison against
/// the snapshot matches even though the live source no longer does.
fn xmtcmp_compares_against_the_snapshot() {
    let mut src = Aligned([0u8; 16]);
    let mut dst = Aligned([0u8; 16]);
    for i in 0..16 {
        src.0[i] = 0x10 + i as u8;
        dst.0[i] = 0x10 + i as u8;
    }

    let tref = ziskos_temporal_snapshot!(&src.0, 16);
    src.0 = [0xFF; 16];

    assert_eq!(ziskos_mtcmp!(&dst.0, &src.0, 16, tref), 0);
}

fn mtcmp_reports_the_first_difference() {
    let mut src = Aligned([0u8; 16]);
    let mut dst = Aligned([0u8; 16]);
    for i in 0..16 {
        src.0[i] = 0x10 + i as u8;
        dst.0[i] = 0x10 + i as u8;
    }
    // dst[4] is one larger than the snapshot's byte
    dst.0[4] = 0x15;

    let count = read_count(16);
    let tref = ziskos_temporal_snapshot!(&src.0, count);
    src.0 = [0xFF; 16];

    assert_eq!(ziskos_mtcmp!(&dst.0, &src.0, count, tref), 1);
}

/// One reference, two advised regions: an extra advice right after the snapshot binds to the same
/// reference.
fn a_reference_survives_several_advised_regions() {
    let mut first = Aligned([1u8; 16]);
    let mut second = Aligned([2u8; 16]);
    let mut dst = Aligned([0u8; 16]);

    let tref = ziskos_temporal_snapshot!(&first.0, 16);
    ziskos_execute_advice!(&second.0, 16);

    first.0 = [9; 16];
    second.0 = [9; 16];

    ziskos_mtcpy!(&mut dst.0, &first.0, 16, tref);
    assert_eq!(dst.0, [1u8; 16]);

    ziskos_mtcpy!(&mut dst.0, &second.0, 16, tref);
    assert_eq!(dst.0, [2u8; 16]);
}

fn two_references_stay_independent() {
    let mut src = Aligned([1u8; 16]);
    let mut dst = Aligned([0u8; 16]);

    let first = ziskos_temporal_snapshot!(&src.0, 16);
    src.0 = [2; 16];
    let second = ziskos_temporal_snapshot!(&src.0, 16);
    src.0 = [3; 16];

    assert_ne!(first, second);

    ziskos_mtcpy!(&mut dst.0, &src.0, 16, first);
    assert_eq!(dst.0, [1u8; 16]);

    ziskos_mtcpy!(&mut dst.0, &src.0, 16, second);
    assert_eq!(dst.0, [2u8; 16]);
}

/// The request and the advice can also be issued separately, as long as no call comes in between.
fn the_reference_can_be_requested_on_its_own() {
    let mut src = Aligned([7u8; 16]);
    let mut dst = Aligned([0u8; 16]);

    let tref = ziskos_temporal_ref!();
    ziskos_execute_advice!(&src.0, 16);
    src.0 = [0; 16];

    ziskos_mtcpy!(&mut dst.0, &src.0, 16, tref);
    assert_eq!(dst.0, [7u8; 16]);

    // references grow with the step count, so a later one is always larger
    let later = ziskos_temporal_ref!();
    assert!(later > tref);
}

/// Keeps a count out of the reach of constant folding, so the register-count patterns are the ones
/// that actually get emitted.
#[inline(never)]
fn read_count(count: usize) -> usize {
    unsafe { core::ptr::read_volatile(&count) }
}
