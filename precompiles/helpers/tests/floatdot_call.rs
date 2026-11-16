use zisk_precomp_helpers::{
    FloatDotCall, FloatDotCallHeader, FloatDotDescriptor, FloatDotFormat, FloatDotMemoryRead,
    Fp32Result, FLOATDOT_MAX_LEN,
};

fn odd_payload() -> Vec<u64> {
    // Ordered [2^24, 1, -2^24] dot [1, 1, 1], with initial DZ.
    vec![
        0x1000,
        0x2000,
        3,
        0,
        8,
        9 << 32,
        0x3f800000_4b800000,
        0x7f800001_cb800000,
        0x3f800000_3f800000,
        0x7f800001_3f800000,
    ]
}

#[test]
fn odd_payload_roundtrips_with_typed_fields_and_ignored_padding() {
    let payload = odd_payload();
    let call = FloatDotCall::decode(0x3000, 77, &payload).unwrap();
    assert_eq!(call.descriptor_addr(), 0x3000);
    assert_eq!(call.step(), 77);
    assert_eq!(
        call.descriptor(),
        FloatDotDescriptor {
            x: 0x1000,
            y: 0x2000,
            n: 3,
            initial: Fp32Result { bits: 0, flags: 8 },
            format: FloatDotFormat::Fp32,
            stride_x: 0,
            stride_y: 0,
        }
    );
    assert_eq!(call.result(), Fp32Result { bits: 0, flags: 9 });
    assert_eq!(call.x_words(), &[0x3f800000_4b800000, 0x7f800001_cb800000]);
    assert_eq!(call.y_words(), &[0x3f800000_3f800000, 0x7f800001_3f800000]);
    assert_eq!(call.reference_result(), Fp32Result { bits: 0, flags: 9 });
    assert_eq!(call.encode(), payload);
}

#[test]
fn memory_reads_are_five_descriptor_words_then_x_then_y() {
    let payload = odd_payload();
    let call = FloatDotCall::decode(0x3000, 77, &payload).unwrap();
    let expected = [
        (0x3000, 0x1000),
        (0x3008, 0x2000),
        (0x3010, 3),
        (0x3018, 0),
        (0x3020, 8),
        (0x1000, 0x3f800000_4b800000),
        (0x1008, 0x7f800001_cb800000),
        (0x2000, 0x3f800000_3f800000),
        (0x2008, 0x7f800001_3f800000),
    ];
    assert_eq!(
        call.memory_reads(),
        expected.map(|(address, value)| FloatDotMemoryRead { address, step: 77, value })
    );
}

#[test]
fn overlapping_reads_are_retained_in_order() {
    // Both vectors alias the descriptor; its first two words are the operands.
    let payload = [0x1000, 0x1000, 3, 0, 0, 0, 0x1000, 0x1000, 0x1000, 0x1000];
    let call = FloatDotCall::decode(0x1000, 4, &payload).unwrap();
    assert_eq!(
        call.memory_reads().iter().map(|read| read.address).collect::<Vec<_>>(),
        [0x1000, 0x1008, 0x1010, 0x1018, 0x1020, 0x1000, 0x1008, 0x1000, 0x1008]
    );
    assert_eq!(call.encode(), payload);
}

#[test]
fn zero_length_has_only_descriptor_reads_and_preserves_initial_state() {
    let payload = [0xfffffff8, 0, 0, 0xff800001, 31, 0x1f_ff800001];
    let call = FloatDotCall::decode(0xffffffd8, (1 << 36) - 1, &payload).unwrap();
    assert_eq!(call.x_words(), &[]);
    assert_eq!(call.y_words(), &[]);
    assert_eq!(call.reference_result(), Fp32Result { bits: 0xff800001, flags: 31 });
    assert_eq!(call.result(), Fp32Result { bits: 0xff800001, flags: 31 });
    assert_eq!(call.memory_reads().len(), 5);
    assert_eq!(
        call.memory_reads().last(),
        Some(&FloatDotMemoryRead { address: 0xfffffff8, step: (1 << 36) - 1, value: 31 })
    );
    assert_eq!(call.encode(), payload);
}

#[test]
fn one_and_two_elements_read_one_complete_word_per_vector() {
    for (n, result) in [(1, 0x3f800000), (2, 0x40400000)] {
        let payload =
            [0xfffffff8, 0x2000, n, 0, 0, result, 0x40000000_3f800000, 0x3f800000_3f800000];
        let call = FloatDotCall::decode(0x3000, 0, &payload).unwrap();
        assert_eq!(call.memory_reads().len(), 7);
        assert_eq!(call.memory_reads()[5].address, 0xfffffff8);
        assert_eq!(u64::from(call.reference_result().bits), result);
    }
}

#[test]
fn maximum_length_fits_the_padded_address_boundary() {
    let mut payload = vec![0xffffc000, 0xffffc000, 4096, 0, 0, 0x45800000];
    payload.resize(6 + FLOATDOT_MAX_LEN, 0x3f800000_3f800000);
    let call = FloatDotCall::decode(0xffffffd8, 0, &payload).unwrap();
    assert_eq!(call.x_words().len(), 2048);
    assert_eq!(call.y_words().len(), 2048);
    assert_eq!(call.memory_reads().len(), 4101);
    assert_eq!(call.memory_reads()[2052].address, 0xfffffff8);
    assert_eq!(call.memory_reads()[4100].address, 0xfffffff8);
    assert_eq!(call.reference_result(), Fp32Result { bits: 0x45800000, flags: 0 });
    assert_eq!(call.encode(), payload);
}

#[test]
fn decoder_rejects_every_truncated_or_extra_payload() {
    let payload = odd_payload();
    for end in 0..payload.len() {
        assert!(FloatDotCall::decode(0x3000, 77, &payload[..end]).is_err(), "length {end}");
    }
    let mut extra = payload;
    extra.push(0);
    assert!(FloatDotCall::decode(0x3000, 77, &extra).is_err());
    assert!(FloatDotCall::decode(0x3000, 0, &[0, 0, 0, 0, 0, 0, 0]).is_err());
}

#[test]
fn decoder_rejects_unbounded_length_before_sizing_or_allocation() {
    for n in [4097, 1 << 32, u64::MAX] {
        assert!(FloatDotCall::decode(0x3000, 0, &[0, 0, n, 0, 0, 0]).is_err());
    }
}

#[test]
fn decoder_rejects_noncanonical_initial_and_result_encodings() {
    for (index, value) in
        [(3, 1 << 32), (3, u64::MAX), (4, 32), (4, u64::MAX), (5, 1 << 37), (5, u64::MAX)]
    {
        let mut payload = odd_payload();
        payload[index] = value;
        assert!(FloatDotCall::decode(0x3000, 0, &payload).is_err(), "index {index} value {value}");
    }
}

#[test]
fn decoder_rejects_bad_descriptor_addresses_and_steps() {
    let payload = odd_payload();
    for address in [1, 0x3004, 1 << 32, u64::MAX, 0xffffffe0, 0xfffffff8] {
        assert!(FloatDotCall::decode(address, 0, &payload).is_err(), "address {address:x}");
    }
    for step in [1 << 36, u64::MAX] {
        assert!(FloatDotCall::decode(0x3000, step, &payload).is_err());
    }
}

#[test]
fn decoder_rejects_bad_vector_addresses_even_when_empty() {
    for n in [0, 3] {
        let mut payload = odd_payload();
        payload[2] = n;
        if n == 0 {
            payload.truncate(6);
        }
        for index in [0, 1] {
            for address in [1, 4, 1 << 32, u64::MAX] {
                let mut malformed = payload.clone();
                malformed[index] = address;
                assert!(FloatDotCall::decode(0x3000, 0, &malformed).is_err());
            }
        }
    }
    for index in [0, 1] {
        let mut payload = odd_payload();
        payload[index] = 0xfffffff8;
        assert!(FloatDotCall::decode(0x3000, 0, &payload).is_err(), "padded range wraps");
    }
}

#[test]
fn structural_decode_does_not_claim_arithmetic_or_memory_authentication() {
    let mut payload = odd_payload();
    payload[5] = 0x1f_7fc01234;
    let call = FloatDotCall::decode(0x3000, 0, &payload).unwrap();
    assert_ne!(call.result(), call.reference_result());
    assert_eq!(call.result(), Fp32Result { bits: 0x7fc01234, flags: 31 });
    assert_eq!(call.encode(), payload);

    // Repeated addresses are retained even when values conflict; the memory bus must reject it.
    payload[1] = payload[0];
    let call = FloatDotCall::decode(0x3000, 0, &payload).unwrap();
    assert_eq!(call.memory_reads()[5].address, call.memory_reads()[7].address);
    assert_ne!(call.memory_reads()[5].value, call.memory_reads()[7].value);
}

#[test]
fn fixed_header_validates_without_loading_or_faking_extended_words() {
    let payload = odd_payload();
    let header = FloatDotCallHeader::decode(0x3000, 77, &payload[..6]).unwrap();
    assert_eq!(header.data_ext_len(), 4);
    assert_eq!(header.memory_read_count(), 9);
    assert_eq!(header.encode(), payload[..6]);
    let call = FloatDotCall::decode_parts(0x3000, 77, &payload[..6], &payload[6..]).unwrap();
    assert_eq!(call.encode(), payload);
    assert_eq!(call.reference_result(), Fp32Result { bits: 0, flags: 9 });
    for end in 0..6 {
        assert!(FloatDotCallHeader::decode(0x3000, 77, &payload[..end]).is_err());
    }
    assert!(FloatDotCallHeader::decode(0x3000, 77, &payload[..7]).is_err());
    for end in 6..payload.len() {
        assert!(FloatDotCall::decode_parts(0x3000, 77, &payload[..6], &payload[6..end]).is_err());
    }
    assert!(FloatDotCall::decode_parts(0x3000, 77, &payload[..6], &[0; 5]).is_err());
}
