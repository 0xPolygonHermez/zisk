use p3_field::Field;
use proofman_util::create_buffer_fast;

use crate::{Boundary, StarkInfo};

pub const W: [u64; 33] = [
    0x1,
    18446744069414584320,
    281474976710656,
    16777216,
    4096,
    64,
    8,
    2198989700608,
    4404853092538523347,
    6434636298004421797,
    4255134452441852017,
    9113133275150391358,
    4355325209153869931,
    4308460244895131701,
    7126024226993609386,
    1873558160482552414,
    8167150655112846419,
    5718075921287398682,
    3411401055030829696,
    8982441859486529725,
    1971462654193939361,
    6553637399136210105,
    8124823329697072476,
    5936499541590631774,
    2709866199236980323,
    8877499657461974390,
    3757607247483852735,
    4969973714567017225,
    2147253751702802259,
    2530564950562219707,
    1905180297017055339,
    3524815499551269279,
    7277203076849721926,
];

#[repr(C)]
pub struct ProverHelpersC {
    pub zi: *mut u8,
    pub s: *mut u8,
    pub x: *mut u8,
    pub x_n: *mut u8,
    pub x_2ns: *mut u8,
}

impl From<&ProverHelpersC> for *mut u8 {
    fn from(prover_helpers: &ProverHelpersC) -> *mut u8 {
        prover_helpers as *const ProverHelpersC as *mut u8
    }
}

pub struct ProverHelpers<F: Field> {
    pub zi: Vec<F>,
    pub s: Vec<F>,
    pub x: Vec<F>,
    pub x_n: Vec<F>,   // For PIL1 compatibility
    pub x_2ns: Vec<F>, // For PIL1 compatibility
}

impl<F: Field> ProverHelpers<F> {
    pub fn new(stark_info: &StarkInfo, pil1: bool) -> Self {
        let n_bits = stark_info.stark_struct.n_bits;
        let n_bits_ext = stark_info.stark_struct.n_bits_ext;
        let q_deg = stark_info.q_deg;
        let boundaries = &stark_info.boundaries;

        let (x_n, x_2ns) = if pil1 {
            Self::compute_connections_x(n_bits, n_bits_ext)
        } else {
            (create_buffer_fast(1 << n_bits), create_buffer_fast(1 << n_bits_ext))
        };

        let zi = Self::compute_zerofier(n_bits, n_bits_ext, boundaries);
        let (x, s) = Self::compute_x(n_bits, n_bits_ext, q_deg);

        Self { zi, s, x, x_n, x_2ns }
    }

    fn compute_zerofier(n_bits: u64, n_bits_ext: u64, boundaries: &[Boundary]) -> Vec<F> {
        let n = 1 << n_bits;
        let n_extended = 1 << n_bits_ext;

        let mut zi = create_buffer_fast(boundaries.len() * n_extended);
        Self::build_zerofier(&mut zi, n_bits, n_bits_ext);

        for (i, boundary) in boundaries[1..].iter().enumerate() {
            if boundary.name == "firstRow" {
                Self::build_one_row_zerofier_inv(&mut zi, n_bits, n_bits_ext, i, 0);
            } else if boundary.name == "lastRow" {
                Self::build_one_row_zerofier_inv(&mut zi, n_bits, n_bits_ext, i, n);
            } else if boundary.name == "everyFrame" {
                Self::build_frame_zerofier_inv(
                    &mut zi,
                    n_bits,
                    n_bits_ext,
                    i,
                    boundary.offset_min.unwrap() as usize,
                    boundary.offset_max.unwrap() as usize,
                );
            }
        }
        zi
    }

    fn compute_connections_x(n_bits: u64, n_bits_ext: u64) -> (Vec<F>, Vec<F>) {
        let n = 1 << n_bits;
        let n_extended = 1 << n_bits_ext;

        let mut x_n = create_buffer_fast(n);
        let mut x_2ns = create_buffer_fast(n_extended);

        let mut xx = F::one();
        let w = F::from_canonical_u64(W[n_bits as usize]);
        for x in x_n.iter_mut() {
            *x = xx;
            xx *= w;
        }

        let mut xx_shift = F::generator();
        let w = F::from_canonical_u64(W[n_bits_ext as usize]);

        for x in x_2ns.iter_mut() {
            *x = xx_shift;
            xx_shift *= w;
        }

        (x_n, x_2ns)
    }

    fn compute_x(n_bits: u64, n_bits_ext: u64, q_deg: u64) -> (Vec<F>, Vec<F>) {
        let n = 1 << n_bits;
        let extend_bits = n_bits_ext - n_bits;

        let mut x = create_buffer_fast(n << extend_bits);
        let w = F::from_canonical_u64(W[n_bits_ext as usize]);
        x[0] = F::generator();
        for k in 1..x.len() {
            x[k] = x[k - 1] * w;
        }

        let mut s = create_buffer_fast(q_deg as usize);
        s[0] = F::one();
        let mut shift_inv = F::generator();
        shift_inv = shift_inv.inverse();
        shift_inv = shift_inv.exp_u64(n as u64);
        for k in 1..q_deg as usize {
            s[k] = s[k - 1] * shift_inv;
        }

        (x, s)
    }

    fn build_zerofier(zi: &mut [F], n_bits: u64, n_bits_ext: u64) {
        let n_extended = 1 << n_bits_ext;
        let extend_bits = n_bits_ext - n_bits;
        let extend = 1 << extend_bits;

        let mut w = F::one();
        let mut sn = F::generator();

        for _ in 0..n_bits {
            sn = sn.square();
        }

        let w_val = F::from_canonical_u64(W[n_bits as usize]);
        for zi_val in zi.iter_mut().take(extend) {
            *zi_val = sn * w - F::one();
            *zi_val = zi_val.inverse();
            w *= w_val;
        }

        (extend..n_extended).for_each(|i| {
            let idx = i % extend;
            zi[i] = zi[idx];
        });
    }

    fn build_one_row_zerofier_inv(zi: &mut [F], n_bits: u64, n_bits_ext: u64, offset: usize, row_index: usize) {
        let n_extended = 1 << n_bits_ext;
        let mut root = F::one();

        let w_val = F::from_canonical_u64(W[n_bits as usize]);
        for _ in 0..row_index {
            root *= w_val;
        }

        let mut w = F::one();
        let sn = F::generator();

        let w_val_ext = F::from_canonical_u64(W[n_bits_ext as usize]);
        for i in 0..n_extended {
            let x = (sn * w - root) * zi[i];
            zi[i + offset * n_extended] = x.inverse();
            w *= w_val_ext;
        }
    }

    fn build_frame_zerofier_inv(
        zi: &mut [F],
        n_bits: u64,
        n_bits_ext: u64,
        offset: usize,
        offset_min: usize,
        offset_max: usize,
    ) {
        let n_extended = 1 << n_bits_ext;
        let n = 1 << n_bits;
        let n_roots = offset_min + offset_max;

        let mut roots = vec![F::zero(); n_roots];

        let w_val = F::from_canonical_u64(W[n_bits as usize]);
        for (i, root) in roots.iter_mut().enumerate().take(offset_min) {
            *root = F::one();
            for _ in 0..i {
                *root *= w_val;
            }
        }

        for i in 0..offset_max {
            roots[i + offset_min] = F::one();
            for _ in 0..(n - i - 1) {
                roots[i + offset_min] *= w_val;
            }
        }

        let mut w = F::one();
        let sn = F::generator();

        let w_val_ext = F::from_canonical_u64(W[n_bits_ext as usize]);
        for i in 0..n_extended {
            zi[i + offset * n_extended] = F::one();
            let x = sn * w;
            for root in &roots {
                zi[i + offset * n_extended] *= x - *root;
            }
            w *= w_val_ext;
        }
    }
}
