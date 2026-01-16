use crate::hints::macros::define_hint;

// === is_on_curve_bn254 (p) ===

define_hint! {
    variant IS_ON_CURVE_BN254{ p: [u64;8] }
    hint_id 14
}

// === to_affine_bn254 (p) ===

define_hint! {
    variant TO_AFFINE_BN254 { p: [u64;12] }
    hint_id 15
}

// === add_bn254 (p1, p2) ===

define_hint! {
    variant ADD_BN254 { p1: [u64;8], p2: [u64;8] }
    hint_id 16
}

// === mul_bn254 (p, k) ===

define_hint! {
    variant MUL_BN254 { p: [u64;8], k: [u64;4] }
    hint_id 17
}

// === to_affine_twist_bn254 (p) ===

define_hint! {
    variant TO_AFFINE_TWIST_BN254 { p: [u64;24] }
    hint_id 18
}

// === is_on_curve_twist_bn254 (p) ===

define_hint! {
    variant IS_ON_CURVE_TWIST_BN254 { p: [u64;16] }
    hint_id 19
}

// === is_on_subgroup_twist_bn254 (p) ===

define_hint! {
    variant IS_ON_SUBGROUP_TWIST_BN254 { p: [u64;16] }
    hint_id 20
}

// === pairing_batch_bn254 (g1_points, g2_points, num_points) ===

// pub const HINTS_TYPE_PAIRING_BATCH_BN254: u32 = 21;

// #[repr(C, align(8))]
// #[derive(Clone, Debug, Eq, PartialEq)]
// pub struct PairingBatchBn254 {
//     pub payload: Vec<u64>, // format: [num_points:u64][g1_points:&[u64;8]][g2_points:&[u64;16]]
// }

// impl PairingBatchBn254 {
//     pub fn new(g1_points: Vec<[u64;8]>, g2_points: Vec<[u64; 16]>) -> Self {
//         assert_eq!(g1_points.len(), g2_points.len(), "g1 and g2 batches must have the same length");
//         let num_points = g1_points.len();
//         let mut payload = Vec::with_capacity(1 + num_points * 8 + num_points * 16);

//         // Store the batch size first so the consumer knows how many points to read
//         payload.push(num_points as u64);

//         // Flatten the G1 points (each 8 limbs) into the payload
//         for point in &g1_points {
//             payload.extend_from_slice(point);
//         }

//         // Flatten the G2 points (each 16 limbs) into the payload
//         for point in &g2_points {
//             payload.extend_from_slice(point);
//         }

//         Self { payload }
//     }
// }

// impl Default for PairingBatchBn254 {
//     fn default() -> Self {
//         Self {
//             payload: Vec::new(),
//         }
//     }
// }

// impl HintData for PairingBatchBn254 {
//     #[inline(always)]
//     fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
//         let header: [u8; 8] =
//             (((HINTS_TYPE_PAIRING_BATCH_BN254 as u64) << 32) | self.payload.len() as u64).to_le_bytes();

//         // Convert payload to bytes
//         let bytes = unsafe {
//             core::slice::from_raw_parts(
//                 self.payload.as_ptr() as *const u8,
//                 self.payload.len() * core::mem::size_of::<u64>(),
//             )
//         };

//         (header, bytes)
//     }

//     fn hint_id(&self) -> u32 {
//         HINTS_TYPE_PAIRING_BATCH_BN254
//     }
// }

// #[inline(always)]
// pub fn hint_pairing_batch_bn254(g1_points: Vec<[u64; 8]>, g2_points: Vec<[u64; 16]>) {
//     check_main_thread();

//     let hint = Hint::PairingBatchBN254(PairingBatchBn254::new(g1_points, g2_points));
//     HINT_QUEUE.push(hint);
// }
