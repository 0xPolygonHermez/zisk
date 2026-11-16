use std::{marker::PhantomData, sync::Arc};

use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use zisk_common::{
    OperationBusData, OperationFloatDotData, B, OPERATION_PRECOMPILED_BUS_DATA_SIZE, STEP,
};
use zisk_pil::{FloatDotTraceRowOps, ZISK_AIRGROUP_ID};
use zisk_precomp_helpers::{
    fp32_add, fp32_mul, FloatDotCall, FloatDotCallHeader, FloatDotDescriptor, FloatDotFormat,
    Fp32Result, Fp32Witness,
};

use crate::floatdot_rows;

/// Validated operation header and owned x words followed by y words.
#[derive(Clone, Debug)]
pub struct FloatDotInput {
    header: FloatDotCallHeader,
    data_ext: Vec<u64>,
}

impl FloatDotInput {
    pub fn from_extended(values: &OperationFloatDotData<u64>, data_ext: &[u64]) -> Self {
        OperationBusData::validate_floatdot_payload(values, data_ext)
            .expect("invalid FloatDot payload");
        let header = FloatDotCallHeader::decode(
            values[B],
            values[STEP],
            &values[OPERATION_PRECOMPILED_BUS_DATA_SIZE..],
        )
        .expect("validated FloatDot header");
        Self { header, data_ext: data_ext.to_vec() }
    }
}

pub struct FloatDotSM<F: PrimeField64> {
    _phantom: PhantomData<F>,
}

impl<F: PrimeField64> FloatDotSM<F> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { _phantom: PhantomData })
    }

    pub fn process_input<R: FloatDotTraceRowOps<F>>(&self, input: &FloatDotInput, trace: &mut [R]) {
        self.fill_call(Some(input), trace);
    }

    fn fill_call<R: FloatDotTraceRowOps<F>>(&self, input: Option<&FloatDotInput>, trace: &mut [R]) {
        let empty = FloatDotCallHeader::decode(0, 0, &[0; 6]).expect("empty FloatDot header");
        let header = input.map_or(empty, |input| input.header);
        let data_ext = input.map_or(&[][..], |input| input.data_ext.as_slice());
        let call = FloatDotCall::decode_parts(
            u64::from(header.descriptor_addr()),
            header.step(),
            &header.encode(),
            data_ext,
        )
        .expect("validated FloatDot call");
        let descriptor = header.descriptor();
        assert_eq!(
            trace.len(),
            floatdot_rows(descriptor.n),
            "FloatDot call requires its exact rows"
        );
        let count = bits::<13>(descriptor.n as u64);
        let mut length_zero = [1; 14];
        for i in 0..13 {
            length_zero[i + 1] = length_zero[i] * (1 - count[i]);
        }
        let mut common = R::default();
        common.set_in_use(u64::from(input.is_some()));
        common.set_all_descriptor(&bits(u64::from(header.descriptor_addr()) >> 3));
        common.set_all_x_half(&bits(u64::from(descriptor.x) >> 1));
        common.set_all_y_half(&bits(u64::from(descriptor.y) >> 1));
        common.set_all_x_stride(&bits(u64::from(descriptor.stride_x)));
        common.set_all_y_stride(&bits(u64::from(descriptor.stride_y)));
        common.set_bf16(u64::from(descriptor.format == FloatDotFormat::Bf16));
        common.set_fp16(u64::from(descriptor.format == FloatDotFormat::Fp16));
        common.set_all_step(&bits(header.step()));
        common.set_all_count(&count);
        common.set_all_initial(&bits(u64::from(descriptor.initial.bits)));
        common.set_all_initial_flags(&bits(u64::from(descriptor.initial.flags)));
        common.set_all_returned(&bits(u64::from(header.result().bits)));
        common.set_all_returned_flags(&bits(u64::from(header.result().flags)));
        common.set_all_descriptor_end(&bits((u64::from(header.descriptor_addr()) >> 3) + 4));
        common.set_all_length_bound(&bits(4096 - descriptor.n as u64));
        common.set_all_length_zero(&length_zero);
        let mut multiply_padding = common;
        let mut add_padding = common;
        write_scalar::<F, R>(&mut multiply_padding, &Fp32Witness::new(0, 0, true));
        write_scalar::<F, R>(&mut add_padding, &Fp32Witness::new(0, 0, false));
        let mut acc = descriptor.initial.bits;
        let mut sticky = descriptor.initial.flags;
        let mut product = Fp32Result { bits: 0, flags: 0 };
        for (index, row) in trace.iter_mut().enumerate() {
            let term = index / 2;
            let is_mul = index % 2 == 0;
            *row = if is_mul { multiply_padding } else { add_padding };
            row.set_start(u64::from(index == 0));
            row.set_end(u64::from(index + 1 == floatdot_rows(descriptor.n)));
            row.set_all_index(&bits(term as u64));
            let remaining = bits::<14>((descriptor.n + 8191 - term) as u64);
            row.set_all_remaining(&remaining);
            let mut last_zero = [1; 14];
            for i in 0..13 {
                last_zero[i + 1] = last_zero[i] * (1 - remaining[i]);
            }
            row.set_all_last_zero(&last_zero);
            row.set_all_acc(&bits(u64::from(acc)));
            row.set_all_sticky(&bits(u64::from(sticky)));
            // FP32 caches a packed word across both elements; 16-bit formats load every term.
            let pair = descriptor.format == FloatDotFormat::Fp32 && term % 2 == 1;
            row.set_load(u64::from(is_mul && !pair));
            let x =
                Operand::new(&descriptor, call.x_words(), descriptor.x, descriptor.stride_x, term);
            let y =
                Operand::new(&descriptor, call.y_words(), descriptor.y, descriptor.stride_y, term);
            row.set_all_x_word(&[bits(x.word & u64::from(u32::MAX)), bits(x.word >> 32)]);
            row.set_all_y_word(&[bits(y.word & u64::from(u32::MAX)), bits(y.word >> 32)]);
            row.set_all_x_waddr(&bits(x.waddr));
            row.set_all_x_lane(&bits(x.lane));
            row.set_all_x_sel(&bits(x.sel));
            row.set_all_x_op(&bits(u64::from(x.op)));
            row.set_all_x_lead(&x.lead);
            row.set_x_ez(x.ez);
            row.set_x_eo(x.eo);
            row.set_all_y_waddr(&bits(y.waddr));
            row.set_all_y_lane(&bits(y.lane));
            row.set_all_y_sel(&bits(y.sel));
            row.set_all_y_op(&bits(u64::from(y.op)));
            row.set_all_y_lead(&y.lead);
            row.set_y_ez(y.ez);
            row.set_y_eo(y.eo);
            if term < descriptor.n {
                let (a, b) = if is_mul { (x.op, y.op) } else { (acc, product.bits) };
                write_scalar::<F, R>(row, &Fp32Witness::new(a, b, is_mul));
                let result = if is_mul { fp32_mul(a, b) } else { fp32_add(a, b) };
                sticky |= result.flags;
                if is_mul {
                    product = result;
                } else {
                    acc = result.bits;
                }
            }
        }
    }

    fn fill_trace<R: FloatDotTraceRowOps<F>>(&self, inputs: &[Vec<FloatDotInput>], rows: &mut [R]) {
        let required = inputs
            .iter()
            .flatten()
            .map(|input| floatdot_rows(input.header.descriptor().n))
            .sum::<usize>();
        assert!(required <= rows.len(), "FloatDot inputs exceed trace capacity");
        assert_eq!(rows.len() % 2, 0, "FloatDot trace must end on an add row");
        let mut offset = 0;
        for input in inputs.iter().flatten() {
            let count = floatdot_rows(input.header.descriptor().n);
            self.process_input(input, &mut rows[offset..offset + count]);
            offset += count;
        }
        if offset < rows.len() {
            let mut padding = [R::default(); 2];
            self.fill_call(None, &mut padding);
            for pair in rows[offset..].chunks_exact_mut(2) {
                pair.copy_from_slice(&padding);
            }
        }
    }

    pub fn compute_witness<
        R: FloatDotTraceRowOps<F>,
        const NUM_ROWS: usize,
        const AIR_ID: usize,
    >(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<FloatDotInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = GenericTrace::<R, NUM_ROWS, ZISK_AIRGROUP_ID, AIR_ID>::new_from_vec_zeroes(
            trace_buffer,
        )?;
        self.fill_trace(inputs, &mut trace.buffer);
        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}

/// Per-row columns of one operand vector: enclosing word, its address and lane, the
/// selected 16-bit lane, the binary32 operand, and FP16 normalization selectors.
struct Operand {
    word: u64,
    waddr: u64,
    lane: u64,
    sel: u64,
    op: u32,
    lead: [u64; 10],
    ez: u64,
    eo: u64,
}

impl Operand {
    fn new(d: &FloatDotDescriptor, words: &[u64], base: u32, stride: u16, term: usize) -> Self {
        // Inactive rows (an empty call) keep the base address and a zero word.
        let (waddr, lane, word) = match d.format {
            FloatDotFormat::Fp32 => (
                u64::from(base >> 3) + (term / 2) as u64,
                2 * (term % 2) as u64,
                words.get(term / 2).copied().unwrap_or(0),
            ),
            _ => {
                let address = u64::from(base) + 2 * u64::from(stride) * term as u64;
                (address >> 3, (address >> 1) & 3, words.get(term).copied().unwrap_or(0))
            }
        };
        let half = (word >> (32 * (lane >> 1))) & u64::from(u32::MAX);
        let sel = (half >> (16 * (lane & 1))) & 0xffff;
        let op = match d.format {
            FloatDotFormat::Fp32 => half as u32,
            format => format.widen(sel as u16),
        };
        let (exponent, mantissa) = ((sel >> 10) & 31, sel & 0x3ff);
        let mut lead = [0; 10];
        if d.format == FloatDotFormat::Fp16 && exponent == 0 && mantissa != 0 {
            lead[63 - mantissa.leading_zeros() as usize] = 1;
        }
        Self {
            word,
            waddr,
            lane,
            sel,
            op,
            lead,
            ez: u64::from(exponent == 0),
            eo: u64::from(exponent == 31),
        }
    }
}

fn bits<const N: usize>(value: u64) -> [u64; N] {
    debug_assert!(N < 64 && value < 1 << N);
    std::array::from_fn(|i| (value >> i) & 1)
}

fn write_scalar<F: PrimeField64, R: FloatDotTraceRowOps<F>>(row: &mut R, w: &Fp32Witness) {
    row.set_all_a(&w.a);
    row.set_all_b(&w.b);
    row.set_is_mul(w.is_mul);
    row.set_all_result(&w.result);
    row.set_all_flags(&w.flags);
    row.set_all_fp_a_sig(&w.fp_a_sig);
    row.set_fp_a_sign(w.fp_a_sign);
    row.set_fp_a_exp(w.fp_a_exp);
    row.set_all_fp_a_cls(&w.fp_a_cls);
    row.set_all_fp_a_ez(&w.fp_a_ez);
    row.set_all_fp_a_eo(&w.fp_a_eo);
    row.set_all_fp_a_fz(&w.fp_a_fz);
    row.set_all_fp_b_sig(&w.fp_b_sig);
    row.set_fp_b_sign(w.fp_b_sign);
    row.set_fp_b_exp(w.fp_b_exp);
    row.set_all_fp_b_cls(&w.fp_b_cls);
    row.set_all_fp_b_ez(&w.fp_b_ez);
    row.set_all_fp_b_eo(&w.fp_b_eo);
    row.set_all_fp_b_fz(&w.fp_b_fz);
    row.set_all_fp_a_finite_sig(&w.fp_a_finite_sig);
    row.set_all_fp_b_finite_sig(&w.fp_b_finite_sig);
    row.set_fp_a_eff_exp(w.fp_a_eff_exp);
    row.set_fp_b_eff_exp(w.fp_b_eff_exp);
    row.set_all_fp_compare(&w.fp_compare);
    row.set_all_fp_delta(&w.fp_delta);
    row.set_all_fp_align_bits(&w.fp_align_bits);
    row.set_all_fp_align_stages(&w.fp_align_stages);
    row.set_all_fp_align_sticky(&w.fp_align_sticky);
    row.set_fp_sign_xor(w.fp_sign_xor);
    row.set_all_fp_sum(&w.fp_sum);
    row.set_all_fp_product(&w.fp_product);
    row.set_all_fp_norm_raw(&w.fp_norm_raw);
    row.set_all_fp_norm_norm(&w.fp_norm_norm);
    row.set_all_fp_norm_high_index(&w.fp_norm_high_index);
    row.set_all_fp_norm_prefix(&w.fp_norm_prefix);
    row.set_all_fp_norm_leading(&w.fp_norm_leading);
    row.set_all_fp_norm_tail(&w.fp_norm_tail);
    row.set_all_fp_norm_sig(&w.fp_norm_sig);
    row.set_all_fp_exponent(&w.fp_exponent);
    row.set_fp_sign(w.fp_sign);
    row.set_all_fp_round_exp_lower(&w.fp_round_exp_lower);
    row.set_all_fp_round_exp_upper(&w.fp_round_exp_upper);
    row.set_all_fp_round_under_diff(&w.fp_round_under_diff);
    row.set_all_fp_round_tiny_diff(&w.fp_round_tiny_diff);
    row.set_all_fp_round_shift_amount(&w.fp_round_shift_amount);
    row.set_all_fp_round_shift_bits(&w.fp_round_shift_bits);
    row.set_all_fp_round_stages(&w.fp_round_stages);
    row.set_all_fp_round_sticky(&w.fp_round_sticky);
    row.set_all_fp_round_rounded(&w.fp_round_rounded);
    row.set_fp_round_inexact(w.fp_round_inexact);
    row.set_all_fp_round_top_ones(&w.fp_round_top_ones);
    row.set_all_fp_round_overflow_diff(&w.fp_round_overflow_diff);
    row.set_all_fp_round_result(&w.fp_round_result);
    row.set_all_fp_round_flags(&w.fp_round_flags);
    row.set_fp_invalid(w.fp_invalid);
    row.set_fp_nan(w.fp_nan);
    row.set_fp_inf(w.fp_inf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
    use zisk_pil::{FloatDotTraceRow, FloatDotTraceRowPacked};

    fn pack(bits: &[u64]) -> u64 {
        bits.iter().enumerate().map(|(i, bit)| bit << i).sum()
    }

    fn input(result: u64) -> FloatDotInput {
        let header = [
            u64::from(ZiskOp::FLOATDOT),
            ZiskOperationType::FloatDot as u64,
            0,
            0xa0001000,
            17,
            0xa0002000,
            0xa0003000,
            3,
            0,
            8,
            result,
        ];
        FloatDotInput::from_extended(
            &header,
            &[0x3f800000_4b800000, 0x7f800001_cb800000, 0x3f800000_3f800000, 0x7f800001_3f800000],
        )
    }

    /// A 16-bit call over sparse memory whose untouched lanes hold a NaN pattern.
    fn input16(
        code: u64,
        x: (&[u16], u32, u16),
        y: (&[u16], u32, u16),
        flags: u64,
    ) -> FloatDotInput {
        let mut memory = std::collections::HashMap::<u32, u64>::new();
        for (values, base, stride) in [x, y] {
            for (i, &h) in values.iter().enumerate() {
                let address = base + 2 * u32::from(stride) * i as u32;
                let word = memory.entry(address & !7).or_insert(0x7fa1_7fa1_7fa1_7fa1);
                let shift = 16 * ((address >> 1) & 3);
                *word = (*word & !(0xffff << shift)) | (u64::from(h) << shift);
            }
        }
        let n = x.0.len() as u64;
        let mut descriptor = vec![
            u64::from(x.1) | u64::from(x.2) << 32,
            u64::from(y.1) | u64::from(y.2) << 32,
            n,
            0x3f80_0000,
            flags | code << 8,
            0,
        ];
        let d = FloatDotCallHeader::decode(0xa0001000, 17, &descriptor).unwrap().descriptor();
        let data_ext: Vec<u64> = d
            .word_addresses(d.x, d.stride_x)
            .into_iter()
            .chain(d.word_addresses(d.y, d.stride_y))
            .map(|address| memory[&address])
            .collect();
        let mut payload = descriptor.clone();
        payload.extend(&data_ext);
        let result = FloatDotCall::decode(0xa0001000, 17, &payload).unwrap().reference_result();
        descriptor[5] = u64::from(result.bits) | u64::from(result.flags) << 32;
        let mut header = vec![
            u64::from(ZiskOp::FLOATDOT),
            ZiskOperationType::FloatDot as u64,
            0,
            0xa0001000,
            17,
        ];
        header.extend(descriptor);
        FloatDotInput::from_extended(&header.try_into().unwrap(), &data_ext)
    }

    fn bf16_input() -> FloatDotInput {
        // Unaligned contiguous x crossing words; V-like stride 1024 y; odd length.
        input16(
            1,
            (&[0x3f80, 0x4000, 0xc040, 0x0001, 0x7f81], 0xa0002002, 1),
            (&[0x4040, 0x3f80, 0x3f80, 0x7f80, 0x3f80], 0xa0004006, 1024),
            0,
        )
    }

    fn fp16_input() -> FloatDotInput {
        // Subnormals, signed zero, sNaN; broadcast y with stride 0.
        input16(
            2,
            (&[0x0001, 0x03ff, 0x8000, 0x7d00], 0xa0005000, 3),
            (&[0x3c00; 4], 0xa0005ffe, 0),
            8,
        )
    }

    #[test]
    fn sixteen_bit_rows_select_lanes_and_widen_exactly() {
        let sm = FloatDotSM::<Goldilocks>::new();
        let mut rows = vec![FloatDotTraceRow::<Goldilocks>::default(); 10];
        sm.process_input(&bf16_input(), &mut rows);
        assert_eq!(rows[0].get_bf16(), 1);
        assert_eq!(pack(&rows[0].get_all_x_half()), 0xa0002002 >> 1);
        assert_eq!(pack(&rows[0].get_all_y_stride()), 1024);
        // x: 0xa0002002 + 2*i; y: 0xa0004006 + 2048*i.
        for (term, (x_waddr, x_lane, y_waddr)) in [
            (0xa0002000u64 >> 3, 1, 0xa0004000u64 >> 3),
            (0xa0002000 >> 3, 2, 0xa0004800 >> 3),
            (0xa0002000 >> 3, 3, 0xa0005000 >> 3),
            (0xa0002008 >> 3, 0, 0xa0005800 >> 3),
        ]
        .into_iter()
        .enumerate()
        {
            for row in &rows[2 * term..2 * term + 2] {
                assert_eq!(pack(&row.get_all_x_waddr()), x_waddr);
                assert_eq!(pack(&row.get_all_x_lane()), x_lane);
                assert_eq!(pack(&row.get_all_y_waddr()), y_waddr);
                assert_eq!(pack(&row.get_all_y_lane()), 3);
            }
            assert_eq!(rows[2 * term].get_load(), 1);
            assert_eq!(rows[2 * term + 1].get_load(), 0);
        }
        assert_eq!(pack(&rows[6].get_all_a()), 0x0001_0000);
        assert_eq!(pack(&rows[8].get_all_x_sel()), 0x7f81);
        assert_eq!(pack(&rows[8].get_all_x_op()), 0x7f81_0000);
        assert_eq!(pack(&rows[9].get_all_returned()), 0x7fc0_0000);
        assert_eq!(pack(&rows[9].get_all_returned_flags()), 16);

        let mut rows = vec![FloatDotTraceRow::<Goldilocks>::default(); 8];
        sm.process_input(&fp16_input(), &mut rows);
        assert_eq!((rows[0].get_bf16(), rows[0].get_fp16()), (0, 1));
        assert_eq!(pack(&rows[0].get_all_x_op()), 0x3380_0000);
        assert_eq!(rows[0].get_all_x_lead(), [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(pack(&rows[2].get_all_x_op()), 0x387f_c000);
        assert_eq!(rows[2].get_all_x_lead()[9], 1);
        assert_eq!(pack(&rows[4].get_all_x_op()), 0x8000_0000);
        assert_eq!(pack(&rows[6].get_all_x_op()), 0x7fa0_0000);
        assert_eq!((rows[6].get_x_ez(), rows[6].get_x_eo()), (0, 1));
        for row in &rows {
            assert_eq!(pack(&row.get_all_y_waddr()), 0xa0005ff8 >> 3);
            assert_eq!(pack(&row.get_all_y_op()), 0x3f80_0000);
        }
        assert_eq!(pack(&rows[7].get_all_returned_flags()), 8 | 16 | 1);
    }

    fn check_calls<R: FloatDotTraceRowOps<Goldilocks>>() {
        let sm = FloatDotSM::<Goldilocks>::new();
        let mut rows = vec![R::default(); 16];
        sm.fill_trace(&[vec![input(9u64 << 32), input(123)]], &mut rows);
        assert_eq!(pack(&rows[0].get_all_a()), 0x4b800000);
        assert_eq!(pack(&rows[1].get_all_result()), 0x4b800000);
        assert_eq!(pack(&rows[3].get_all_flags()), 1);
        assert_eq!(pack(&rows[4].get_all_acc()), 0x4b800000);
        assert_eq!(pack(&rows[4].get_all_sticky()), 9);
        assert_eq!(pack(&rows[5].get_all_result()), 0);
        assert_eq!(pack(&rows[5].get_all_x_word()[1]), 0x7f800001);
        assert_eq!(pack(&rows[5].get_all_returned_flags()), 9);
        assert_eq!(rows[5].get_end(), 1);
        assert_eq!(rows[6].get_start(), 1);
        assert_eq!(pack(&rows[6].get_all_index()), 0);
        assert_eq!(pack(&rows[6].get_all_acc()), 0);
        assert_eq!(pack(&rows[6].get_all_sticky()), 8);
        assert_eq!(pack(&rows[6].get_all_x_word()[0]), 0x4b800000);
        assert_eq!(pack(&rows[0].get_all_step()), 17);
        assert_eq!(pack(&rows[0].get_all_descriptor()), 0xa0001000 >> 3);
        // Claimed output stays separate from recomputed arithmetic for AIR validation.
        assert_eq!(pack(&rows[11].get_all_result()), 0);
        assert_eq!(pack(&rows[11].get_all_returned()), 123);
        for (i, row) in rows.iter().enumerate().take(16).skip(12) {
            assert_eq!(row.get_in_use(), 0);
            assert_eq!(row.get_start(), (1 - i % 2) as u64);
            assert_eq!(row.get_end(), (i % 2) as u64);
            assert_eq!(pack(&row.get_all_descriptor_end()), 4);
            assert_eq!(pack(&row.get_all_count()), 0);
            assert_eq!(pack(&row.get_all_length_bound()), 4096);
            assert_eq!(pack(&row.get_all_remaining()), 8191);
            assert_eq!(row.get_is_mul(), (1 - i % 2) as u64);
            assert_eq!(pack(&row.get_all_result()), 0);
        }
    }

    #[test]
    fn short_call_uses_only_its_arithmetic_rows() {
        let sm = FloatDotSM::<Goldilocks>::new();
        let mut rows = vec![FloatDotTraceRow::<Goldilocks>::default(); 6];
        sm.process_input(&input(9u64 << 32), &mut rows);
        assert_eq!(pack(&rows[5].get_all_result()), 0);
        assert_eq!(pack(&rows[5].get_all_returned_flags()), 9);
    }

    #[test]
    fn ordered_calls_and_padding_fill_both_trace_layouts() {
        check_calls::<FloatDotTraceRow<Goldilocks>>();
        check_calls::<FloatDotTraceRowPacked<Goldilocks>>();
    }

    #[test]
    #[should_panic(expected = "FloatDot inputs exceed trace capacity")]
    fn complete_calls_cannot_overflow_the_trace() {
        FloatDotSM::<Goldilocks>::new().fill_trace(
            &[vec![input(9u64 << 32), input(9u64 << 32)]],
            &mut vec![FloatDotTraceRow::<Goldilocks>::default(); 10],
        );
    }

    #[test]
    fn input_rejects_missing_words_before_owning_them() {
        let mut header = [0; 11];
        header[0] = u64::from(ZiskOp::FLOATDOT);
        header[1] = ZiskOperationType::FloatDot as u64;
        header[7] = 1;
        assert!(std::panic::catch_unwind(|| FloatDotInput::from_extended(&header, &[])).is_err());
        header[7] = 0;
        header[2] = 1;
        assert!(std::panic::catch_unwind(|| FloatDotInput::from_extended(&header, &[])).is_err());
    }

    #[test]
    fn scalar_setters_preserve_every_air_column() {
        type Rows = GenericTrace<
            FloatDotTraceRow<Goldilocks>,
            2,
            ZISK_AIRGROUP_ID,
            { zisk_pil::FloatDotTrace::<()>::AIR_ID },
        >;
        let mut trace = Rows::new_zeroes();
        let witnesses = [
            Fp32Witness::new(0x00800000, 0x3f7fffff, true),
            Fp32Witness::new(0xff800001, 0x3f800001, false),
        ];
        for (row, witness) in trace.buffer.iter_mut().zip(&witnesses) {
            write_scalar::<Goldilocks, _>(row, witness);
        }
        let buffer = trace.get_buffer::<Goldilocks>();
        for (row, witness) in buffer.chunks_exact(Rows::ROW_SIZE).zip(witnesses) {
            let actual: Vec<_> = row[..zisk_precomp_helpers::FP32_WITNESS_COLUMNS]
                .iter()
                .map(PrimeField64::as_canonical_u64)
                .collect();
            assert_eq!(actual, witness.to_row());
        }
    }

    #[test]
    #[ignore = "writes selected production dot rows and predecessors for compiled PIL evaluation"]
    fn dump_dot_witness_rows() {
        use std::{collections::BTreeSet, io::Write};
        use zisk_pil::FloatDotTrace;
        let path = std::env::var_os("FP32_DOT_WITNESS_DUMP").expect("set FP32_DOT_WITNESS_DUMP");
        let mut output = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
        let empty_header = [
            u64::from(ZiskOp::FLOATDOT),
            ZiskOperationType::FloatDot as u64,
            0,
            0xa0001000,
            17,
            0,
            0,
            0,
            0xff800001,
            31,
            (31u64 << 32) | 0xff800001,
        ];
        let max_header = [
            u64::from(ZiskOp::FLOATDOT),
            ZiskOperationType::FloatDot as u64,
            0,
            0xa0001000,
            17,
            0xa0002000,
            0xa000a000,
            4096,
            0,
            0,
            0x45800000,
        ];
        let cases = [
            input(9u64 << 32),
            FloatDotInput::from_extended(&empty_header, &[]),
            FloatDotInput::from_extended(&max_header, &vec![0x3f800000_3f800000; 4096]),
            bf16_input(),
            fp16_input(),
        ];
        let sm = FloatDotSM::<Goldilocks>::new();
        for (case, input) in cases.iter().enumerate() {
            let mut trace = FloatDotTrace::<FloatDotTraceRow<Goldilocks>>::new_zeroes();
            let width = trace.num_cols();
            let num_rows = trace.num_rows();
            // Include adjacent odd, empty, and maximum-length calls in each layout.
            let calls = vec![
                input.clone(),
                cases[1].clone(),
                cases[0].clone(),
                cases[3].clone(),
                cases[4].clone(),
            ];
            sm.fill_trace(std::slice::from_ref(&calls), &mut trace.buffer);
            let mut indexes = BTreeSet::new();
            for i in [0, 1, 2, 3, 4, 5, 6, 7, 8, 4095, 4096, 8190, 8191, 8192, 8193, num_rows - 1] {
                indexes.insert(i);
                indexes.insert((i + num_rows - 1) % num_rows);
            }
            let mut offset = 0;
            for call in &calls {
                for i in
                    [offset, offset + 1, offset + floatdot_rows(call.header.descriptor().n) - 1]
                {
                    indexes.insert(i);
                    indexes.insert((i + num_rows - 1) % num_rows);
                }
                offset += floatdot_rows(call.header.descriptor().n);
            }
            indexes.insert(offset);
            indexes.insert(offset + 1);
            let buffer = trace.get_buffer::<Goldilocks>();
            for i in indexes {
                write!(output, "{case},{i}").unwrap();
                for value in &buffer[i * width..(i + 1) * width] {
                    write!(output, ",{}", value.as_canonical_u64()).unwrap();
                }
                writeln!(output).unwrap();
            }
        }
        output.flush().unwrap();
    }
}
