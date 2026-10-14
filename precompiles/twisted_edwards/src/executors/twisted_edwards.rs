use super::TwistedEdwardsData;
use crate::babyjubjub_constants::{
    BABYJUBJUB_A, BABYJUBJUB_D, BABYJUBJUB_PRIME, BABYJUBJUB_PRIME_CHUNKS,
};
use crate::ed25519_constants::{ED25519_D, ED25519_PRIME, ED25519_PRIME_CHUNKS};
use crate::equations;
use crate::twisted_edwards_constants::{SEL_OP_BABYJUBJUB_ADD, SEL_OP_ED25519_ADD};
use ark_bn254::Fr as BabyJubJubField;
use ark_ff::{BigInteger, PrimeField};
use lazy_static::lazy_static;
use num_bigint::{BigInt, Sign};
use num_traits::Zero;
use zisk_precomp_helpers::{bigint2_to_8_u64, bigint_from_u64s, bigint_to_16_chunks, Ed25519Field};

const COLS: u8 = 32;

lazy_static! {
    pub static ref BABYJUBJUB_PRIME_BI: BigInt =
        BigInt::parse_bytes(BABYJUBJUB_PRIME.as_bytes(), 16).unwrap();
    pub static ref ED25519_PRIME_BI: BigInt =
        BigInt::parse_bytes(ED25519_PRIME.as_bytes(), 16).unwrap();
    pub static ref BABYJUBJUB_D_BI: BigInt = BigInt::from(BABYJUBJUB_D);
    pub static ref ED25519_D_BI: BigInt = BigInt::parse_bytes(ED25519_D.as_bytes(), 16).unwrap();
}

/// Offset added to keep every plus-convention quotient non-negative.
const OFFSET_SMALL: i64 = 0x10;

/// A curve proved by the TwistedEdwards air. Each one owns an add operation (its own `sel_op`
/// slot) and the generated equations of `equations/<curve>_*.rs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwistedEdwardsCurve {
    /// circomlib BabyJubJub over the BN254 scalar field: a = 168700, d = 168696.
    BabyJubJub,
    /// RFC 8032 edwards25519 over 2^255 - 19: a = -1, d = -121665/121666.
    Ed25519,
}

impl TwistedEdwardsCurve {
    /// `sel_op` slot of this curve's add operation.
    pub fn sel_op(self) -> usize {
        match self {
            Self::BabyJubJub => SEL_OP_BABYJUBJUB_ADD,
            Self::Ed25519 => SEL_OP_ED25519_ADD,
        }
    }

    /// The prime, as 16 little-endian 16-bit chunks (alias-free output check).
    pub fn prime_chunks(self) -> &'static [i64; 16] {
        match self {
            Self::BabyJubJub => &BABYJUBJUB_PRIME_CHUNKS,
            Self::Ed25519 => &ED25519_PRIME_CHUNKS,
        }
    }

    fn prime(self) -> &'static BigInt {
        match self {
            Self::BabyJubJub => &BABYJUBJUB_PRIME_BI,
            Self::Ed25519 => &ED25519_PRIME_BI,
        }
    }

    /// `d` as the integer the generated EQ_DT uses.
    fn d(self) -> &'static BigInt {
        match self {
            Self::BabyJubJub => &BABYJUBJUB_D_BI,
            Self::Ed25519 => &ED25519_D_BI,
        }
    }

    /// `a` as it appears in EQ_Y3 (`... + a*A ...`), i.e. signed and not reduced mod p.
    fn a(self) -> i64 {
        match self {
            Self::BabyJubJub => BABYJUBJUB_A as i64,
            Self::Ed25519 => -1,
        }
    }

    /// Offset of the minus-convention EQ_Y3 quotient (must match the generator).
    fn y3_offset(self) -> i64 {
        match self {
            Self::BabyJubJub => 0x40000,
            Self::Ed25519 => OFFSET_SMALL,
        }
    }

    /// Reduced scratch values and result `[A, B, N, T, DT, x3, y3]` of `p1 + p2`.
    fn field_values(self, p1: &[u64; 8], p2: &[u64; 8]) -> [BigInt; 7] {
        match self {
            Self::BabyJubJub => field_values::<BabyJubJubField>(
                p1,
                p2,
                BabyJubJubField::from(BABYJUBJUB_A),
                BabyJubJubField::from(BABYJUBJUB_D),
            ),
            Self::Ed25519 => field_values::<Ed25519Field>(
                p1,
                p2,
                -Ed25519Field::from(1u64),
                zisk_precomp_helpers::ed25519_d(),
            ),
        }
    }

    fn equations(self, icol: u8, data: &TwistedEdwardsData) -> [i64; 7] {
        macro_rules! eqs {
            ($a:ident, $b:ident, $n:ident, $t:ident, $dt:ident, $x3:ident, $y3:ident) => {
                [
                    equations::$a::calculate(icol, &data.x1, &data.x2, &data.a, &data.qa),
                    equations::$b::calculate(icol, &data.y1, &data.y2, &data.b, &data.qb),
                    equations::$n::calculate(
                        icol, &data.x1, &data.y2, &data.y1, &data.x2, &data.n, &data.qn,
                    ),
                    equations::$t::calculate(icol, &data.a, &data.b, &data.t, &data.qt),
                    equations::$dt::calculate(icol, &data.t, &data.dt, &data.qdt),
                    equations::$x3::calculate(icol, &data.x3, &data.dt, &data.n, &data.qx),
                    equations::$y3::calculate(
                        icol, &data.y3, &data.dt, &data.b, &data.a, &data.qy,
                    ),
                ]
            };
        }
        match self {
            Self::BabyJubJub => eqs!(
                BabyJubJubA,
                BabyJubJubB,
                BabyJubJubN,
                BabyJubJubT,
                BabyJubJubDt,
                BabyJubJubX3,
                BabyJubJubY3
            ),
            Self::Ed25519 => {
                eqs!(Ed25519A, Ed25519B, Ed25519N, Ed25519T, Ed25519Dt, Ed25519X3, Ed25519Y3)
            }
        }
    }
}

/// Converts 4 little-endian limbs to a field element, reducing mod p.
fn limbs_to_field<Fp: PrimeField>(limbs: &[u64]) -> Fp {
    let mut bytes = [0u8; 32];
    for (chunk, limb) in bytes.chunks_mut(8).zip(limbs) {
        chunk.copy_from_slice(&limb.to_le_bytes());
    }
    Fp::from_le_bytes_mod_order(&bytes)
}

/// Converts a field element to a `num_bigint::BigInt` in `[0, p)`.
fn field_to_bigint<Fp: PrimeField>(f: &Fp) -> BigInt {
    let bytes = f.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(Sign::Plus, &bytes)
}

fn field_values<Fp: PrimeField>(p1: &[u64; 8], p2: &[u64; 8], a: Fp, d: Fp) -> [BigInt; 7] {
    let (x1, y1) = (limbs_to_field::<Fp>(&p1[0..4]), limbs_to_field::<Fp>(&p1[4..8]));
    let (x2, y2) = (limbs_to_field::<Fp>(&p2[0..4]), limbs_to_field::<Fp>(&p2[4..8]));
    let one = Fp::one();

    let af = x1 * x2; // A  = x1*x2
    let bf = y1 * y2; // B  = y1*y2
    let nf = x1 * y2 + y1 * x2; // N  = x1*y2 + y1*x2
    let tf = af * bf; // T  = A*B = tau
    let dtf = d * tf; // DT = d*T
    let x3f = nf / (one + dtf);
    let y3f = (bf - a * af) / (one - dtf);

    [af, bf, nf, tf, dtf, x3f, y3f].map(|f| field_to_bigint(&f))
}

pub struct TwistedEdwards {}

impl TwistedEdwards {
    #[allow(dead_code)]
    pub fn calculate_add(
        curve: TwistedEdwardsCurve,
        p1: &[u64; 8],
        p2: &[u64; 8],
        p3: &mut [u64; 8],
    ) {
        let [.., x3, y3] = curve.field_values(p1, p2);
        bigint2_to_8_u64(&x3, &y3, p3);
    }

    /// Builds the witness of `p1 + p2`. Inputs are taken exactly as read from memory (the air
    /// does not require them reduced), while scratch values and the result are reduced mod p.
    fn prepare(curve: TwistedEdwardsCurve, p1: &[u64; 8], p2: &[u64; 8]) -> TwistedEdwardsData {
        let [av, bv, nv, tv, dtv, x3, y3] = curve.field_values(p1, p2);

        let x1 = bigint_from_u64s(&p1[0..4]);
        let y1 = bigint_from_u64s(&p1[4..8]);
        let x2 = bigint_from_u64s(&p2[0..4]);
        let y2 = bigint_from_u64s(&p2[4..8]);

        let p = curve.prime();
        let a = BigInt::from(curve.a());

        let plus = |lhs: BigInt| -> BigInt {
            debug_assert!((&lhs % p).is_zero());
            lhs / p + OFFSET_SMALL
        };

        let qa = plus(&x1 * &x2 - &av);
        let qb = plus(&y1 * &y2 - &bv);
        let qn = plus(&x1 * &y2 + &y1 * &x2 - &nv);
        let qt = plus(&av * &bv - &tv);
        let qdt = plus(curve.d() * &tv - &dtv);
        let qx = plus(&x3 + &x3 * &dtv - &nv);

        let _qy: BigInt = &y3 - &y3 * &dtv - &bv + &a * &av;
        debug_assert!((&_qy % p).is_zero());
        let qy = BigInt::from(curve.y3_offset()) - (&_qy / p);

        let mut data = TwistedEdwardsData::default();
        bigint_to_16_chunks(&x1, &mut data.x1);
        bigint_to_16_chunks(&y1, &mut data.y1);
        bigint_to_16_chunks(&x2, &mut data.x2);
        bigint_to_16_chunks(&y2, &mut data.y2);
        bigint_to_16_chunks(&x3, &mut data.x3);
        bigint_to_16_chunks(&y3, &mut data.y3);
        bigint_to_16_chunks(&av, &mut data.a);
        bigint_to_16_chunks(&bv, &mut data.b);
        bigint_to_16_chunks(&nv, &mut data.n);
        bigint_to_16_chunks(&tv, &mut data.t);
        bigint_to_16_chunks(&dtv, &mut data.dt);
        bigint_to_16_chunks(&qa, &mut data.qa);
        bigint_to_16_chunks(&qb, &mut data.qb);
        bigint_to_16_chunks(&qn, &mut data.qn);
        bigint_to_16_chunks(&qt, &mut data.qt);
        bigint_to_16_chunks(&qdt, &mut data.qdt);
        bigint_to_16_chunks(&qx, &mut data.qx);
        bigint_to_16_chunks(&qy, &mut data.qy);
        data
    }

    pub fn execute_add(
        curve: TwistedEdwardsCurve,
        p1: &[u64; 8],
        p2: &[u64; 8],
    ) -> TwistedEdwardsData {
        let mut data = Self::prepare(curve, p1, p2);
        for icol in 0..COLS {
            let index = icol as usize;
            data.eq[index] = curve.equations(icol, &data);
            for ieq in 0..7 {
                let cin = if index > 0 { data.cout[index - 1][ieq] } else { 0 };
                let value = data.eq[index][ieq] + cin;
                if icol != COLS - 1 {
                    data.cout[index][ieq] = value / 0x10000;
                }
                debug_assert!(
                    0 == if icol == COLS - 1 { value } else { value % 0x10000 },
                    "TwistedEdwards {curve:?} residue eq{ieq} ({index}) #:{value} cin:{cin}"
                );
            }
        }
        data
    }

    #[cfg(any(test, feature = "test_data"))]
    #[allow(dead_code)]
    pub fn verify_add(curve: TwistedEdwardsCurve, p1: &[u64; 8], p2: &[u64; 8], p: &[u64; 8]) {
        let data = Self::execute_add(curve, p1, p2);
        data.check_ranges();
        for i in 0..2 {
            let offset = (i + 1) * 4 - 1;
            let mut x3 = data.x3[offset] as u64;
            let mut y3 = data.y3[offset] as u64;
            for j in 1..4 {
                x3 <<= 16;
                y3 <<= 16;
                x3 += data.x3[offset - j] as u64;
                y3 += data.y3[offset - j] as u64;
            }
            assert!(p[i] == x3, "{curve:?} p[{}]:{} not match with x3:{}", i, p[i], x3);
            assert!(
                p[i + 4] == y3,
                "{curve:?} p[{}]:{} not match with y3:{}",
                i + 4,
                p[i + 4],
                y3
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    fn fr_limbs(decimal: &str) -> [u64; 4] {
        BabyJubJubField::from_str(decimal).expect("invalid field element").into_bigint().0
    }

    fn point(x: &str, y: &str) -> [u64; 8] {
        let mut p = [0u64; 8];
        p[0..4].copy_from_slice(&fr_limbs(x));
        p[4..8].copy_from_slice(&fr_limbs(y));
        p
    }

    // Ground-truth vector from circomlib test/babyjub.js ("Should add two points").
    #[test]
    fn add_matches_circomlib_vector() {
        let p1 = point(
            "17777552123799933955779906779655732241715742912184938656739573121738514868268",
            "2626589144620713026669568689430873010625803728049924121243784502389097019475",
        );
        let p2 = point(
            "16540640123574156134436876038791482806971768689494387082833631921987005038935",
            "20819045374670962167435360035096875258406992893633759881276124905556507972311",
        );
        let expected = point(
            "7916061937171219682591368294088513039687205273691143098332585753343424131937",
            "14035240266687799601661095864649209771790948434046947201833777492504781204499",
        );
        TwistedEdwards::verify_add(TwistedEdwardsCurve::BabyJubJub, &p1, &p2, &expected);
    }

    // Random field inputs near the modulus stress every chunk/carry/quotient range; the
    // executor must reconstruct the host-helper result and stay within check_ranges bounds.
    #[test]
    fn add_ranges_hold_for_random_inputs() {
        // splitmix64 PRNG (no external dep).
        let mut state: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            state = state.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        };
        let rand_fr = |next: &mut dyn FnMut() -> u64| -> [u64; 4] {
            let mut bytes = [0u8; 32];
            for chunk in bytes.chunks_mut(8) {
                chunk.copy_from_slice(&next().to_le_bytes());
            }
            BabyJubJubField::from_le_bytes_mod_order(&bytes).into_bigint().0
        };
        for _ in 0..5000 {
            let mut p1 = [0u64; 8];
            let mut p2 = [0u64; 8];
            p1[0..4].copy_from_slice(&rand_fr(&mut next));
            p1[4..8].copy_from_slice(&rand_fr(&mut next));
            p2[0..4].copy_from_slice(&rand_fr(&mut next));
            p2[4..8].copy_from_slice(&rand_fr(&mut next));
            let mut expected = [0u64; 8];
            zisk_precomp_helpers::babyjubjub_add(&p1, &p2, &mut expected);
            TwistedEdwards::verify_add(TwistedEdwardsCurve::BabyJubJub, &p1, &p2, &expected);
        }
    }

    // Executor result must match the host helper bit-for-bit, including doubling.
    #[test]
    fn add_matches_host_helper() {
        let base = point(
            "17777552123799933955779906779655732241715742912184938656739573121738514868268",
            "2626589144620713026669568689430873010625803728049924121243784502389097019475",
        );
        let other = point(
            "16540640123574156134436876038791482806971768689494387082833631921987005038935",
            "20819045374670962167435360035096875258406992893633759881276124905556507972311",
        );
        let identity = point("0", "1");
        for (p1, p2) in [(base, other), (base, base), (base, identity), (identity, identity)] {
            let mut expected = [0u64; 8];
            zisk_precomp_helpers::babyjubjub_add(&p1, &p2, &mut expected);
            TwistedEdwards::verify_add(TwistedEdwardsCurve::BabyJubJub, &p1, &p2, &expected);

            let mut got = [0u64; 8];
            TwistedEdwards::calculate_add(TwistedEdwardsCurve::BabyJubJub, &p1, &p2, &mut got);
            assert_eq!(got, expected, "calculate_add mismatch");
        }
    }

    // Independent-oracle cross-check: vectors generated by arnaucube/babyjubjub-rs (a separate
    // implementation, projective add-2008-bbjlp) rather than our own ark_bn254::Fr formula.
    #[test]
    fn add_matches_babyjubjub_rs_vectors() {
        // (p1, p2, p3 = p1 + p2), each [u64;8] = x‖y LE limbs.
        // Includes identity laws (P+O, O+O) and deliberate doublings.
        #[rustfmt::skip]
        const VECTORS: &[([u64; 8], [u64; 8], [u64; 8])] = &[
            ([13776628885078076759, 17192168939367597241, 4038109072970774824, 1547695284297813601, 1978261447218200958, 17475828874145971098, 3665104879553117438, 2515885876984883184], [0, 0, 0, 0, 1, 0, 0, 0], [13776628885078076759, 17192168939367597241, 4038109072970774824, 1547695284297813601, 1978261447218200958, 17475828874145971098, 3665104879553117438, 2515885876984883184]),
            ([0, 0, 0, 0, 1, 0, 0, 0], [0, 0, 0, 0, 1, 0, 0, 0], [0, 0, 0, 0, 1, 0, 0, 0]),
            ([3763798864748236520, 8298471196003298756, 12460235914325922890, 3047173900851596384, 4571748410885429973, 4523588032312272190, 1676630629911186218, 3348673288667043639], [3763798864748236520, 8298471196003298756, 12460235914325922890, 3047173900851596384, 4571748410885429973, 4523588032312272190, 1676630629911186218, 3348673288667043639], [15843428223110705897, 14460864151858456428, 2735263167764658524, 567635962720228519, 4106642745955857954, 9639697921861639182, 1020253447237243719, 2717280723716366687]),
            ([3138412068497202964, 13295987350842066516, 7738376944659771487, 3477304294808703512, 5576527852531342281, 10102414601246412192, 11080797389575925815, 3347671929691310059], [18257455887924078597, 10229854351947227084, 5147940355206531389, 1026280865242139873, 14676970959555134237, 3806702479927790302, 2469148876842939747, 718160486167769141], [8353714181168952036, 15678332900248681089, 18262775729154510250, 243095648229401407, 5260166238596176248, 6270740529200156640, 15855628129498666643, 2706581858914131063]),
            ([17289381552119486960, 1614313106257101085, 5830004928182058725, 190459855242282922, 10849873782287451908, 1377628978541083275, 165176543883917728, 1202630159922370600], [12244030850766363258, 16657188202755553729, 9000549355818177902, 2562698284811184817, 2189053619423568510, 5355038650674883808, 9894424561527418248, 102520744252319101], [8735016053614912272, 3589924415486039548, 17197896874433925557, 3135433166851907839, 1641777647956347676, 18111964401502939340, 9157794103676311138, 866302692096195503]),
            ([7408808972208651317, 4865600854897251732, 13965361999313604171, 508240106007268148, 4352920645506828486, 6139192289096410932, 6644245237968233117, 2202916717906448489], [13608936526906166831, 14537583256500979220, 17275170856153345216, 727555996614453669, 13785558421567079763, 11602786520219409485, 2681512225109201419, 1365308957311124583], [1135134007624377183, 8491063875105323643, 5902638270866924636, 3319023507254690419, 866596360702813242, 3078865944602480329, 4260022410430743035, 1822659728870059277]),
            ([6724893965447600980, 3630809342293396956, 10359759267776098534, 2860853035186204565, 17169500184504627703, 1682861306054058831, 9958386737566918828, 2930621109695708659], [181882737397840026, 1930340037308490809, 6753747651528125558, 981467940562847670, 13758150931107373242, 5935132595240481018, 6487872239840213472, 238668919424903743], [6363120704973999548, 10411035347700343583, 1511142615465380196, 452036902687481428, 1556754553697424699, 7946598089353837411, 3817626657682259745, 792644935166007594]),
            ([7717853342671795537, 832318872236909209, 11992062685015549044, 207204283331114504, 11332699784974025156, 4404417066273776394, 4364735401332134825, 2433829578488234881], [7717853342671795537, 832318872236909209, 11992062685015549044, 207204283331114504, 11332699784974025156, 4404417066273776394, 4364735401332134825, 2433829578488234881], [11679292330523052515, 4109044322978739223, 15316461245080771990, 1048397260613198299, 3289539170023693906, 12674339788027481260, 9753441379042660900, 1534689967230263265]),
            ([10887394506490229672, 10528522077968798698, 1970807208569479938, 654887625947646942, 16194654311601487715, 5044132449839392615, 12021979514546481336, 1702198588501134174], [3873921448114893496, 2354053819357155418, 17592494767293783340, 1649768213662080668, 5825484817993283685, 357946160912365088, 13129997050934697218, 1783347964641535613], [10857595941557012765, 10693900133465904464, 3603730038542339845, 2926843404146286382, 10354542628283063655, 10836768565180695991, 9402347586295848726, 1573554000783807805]),
            ([16700122835100529740, 9545845909250206045, 5159180843002989203, 1609203118086652381, 14147253975379158378, 14689065034502954001, 14078674824069017872, 1572827094468767509], [16563393161387561441, 15565173209351487830, 9337591075467619068, 2174586183778754466, 4170912717036309757, 12961503774258027825, 6923465849890219958, 1260781540270124525], [2033902920859644299, 3373684186912066619, 16745515686057141890, 2959452174140063034, 9342036714148719771, 13107410427838135043, 11735752278113093403, 1343200511809331666]),
            ([9407769729717388630, 16779117493498455047, 6748341349876361122, 1100810906607626012, 17933428346185129339, 3883081399026598729, 6248330285459251681, 457034578547576194], [16155401989967027233, 427840360997355553, 10918862356900808504, 2152031207686375750, 1840060620254620682, 15301125212933021833, 12699675876809884196, 1149073512707399130], [6039196601118511826, 3989267422179561785, 17741315847594252888, 2552782340834391105, 16661105591817791595, 7590142869623277705, 17153563165858603547, 2950077094218924456]),
            ([542418156539017694, 16678590104406768406, 17550255546726733679, 642715847834755418, 10498868303263411321, 3173862636670355271, 17397973905678115983, 820818483340225220], [13490598836310613813, 11993669486148072576, 4026101483886590621, 1828356459858154973, 10274487130703891605, 13013846053934138949, 118671176241598778, 3035107453162956532], [11473198328632928530, 3979268525996687257, 5426077509137204356, 2388856939387967319, 4097638403029916280, 14437191374138624895, 2804263056831838823, 3395095533606279395]),
            ([18090042523804283537, 12410061439422700013, 13207134570018127107, 1355373205405171231, 15828132088733251186, 6365140687275920796, 3869002959649804861, 2263171431837945742], [18090042523804283537, 12410061439422700013, 13207134570018127107, 1355373205405171231, 15828132088733251186, 6365140687275920796, 3869002959649804861, 2263171431837945742], [13554674439590648512, 2083096358645558291, 17754247534914319479, 432223031161478270, 17401269634857310715, 7477689782645851666, 10803677576163093982, 2458806159328764019]),
            ([8668481466387627479, 3112960813316344260, 10478103650228932480, 3074971106419027708, 15377452048814839579, 8780841645652275938, 17001295480304504951, 678818191144494572], [2239337243501589935, 7239286093183483051, 15388375295991097168, 663723208591779533, 7950328104399576442, 17531164255655549745, 7158815373484336084, 590294032219082699], [16788727223528411502, 3495755295590506902, 9144662346117776248, 1644634358107658444, 3549829802579523525, 3684337910218592587, 16429483205108240653, 129697928847592792]),
            ([11203228655141405878, 1779630155384702225, 12347737031636885847, 24715713145348603, 13773170550531940739, 15474423799319678903, 14788668618095534258, 1218123895987048788], [16938541676703582892, 17598523928323883719, 16792548889948455165, 1503477595822868568, 11732661338720934400, 13768438183540763619, 9055912814387967696, 3083471700715433675], [6940830269197298414, 16264616417080989773, 16383092603829474408, 3117708422381316205, 4310978833736155793, 6964508318756228752, 5609594626689136835, 309686845146999126]),
            ([981649612642583257, 1363804445322307249, 13377379138239074627, 2088629340176759911, 14473713816165492381, 16133196492840157948, 7985308916267260245, 892355901842517936], [14357467241272816386, 7168559659380140501, 11153581501665812437, 1941389076805214734, 13942073963269962382, 5077894045891731787, 7207583841298560225, 1530131789845686725], [4964623296573644060, 10903873669212376761, 7597020412703139236, 2168693809802814272, 12770424957570954510, 3134527182256305163, 12640785763447775202, 3456310965218976820]),
            ([1786406649956924246, 2664621724628607163, 14342678801114087654, 168186411167210612, 16614182149505643742, 5786582884079784885, 11335703205651180095, 2850772241255716811], [13276944250984596608, 1378637888165805077, 2873157034463094309, 1210020785942965877, 8512771076763724446, 14718143181908547967, 4149069734522841391, 2821961903097410214], [13078149561302423704, 14933732821461190904, 12836948772708456706, 2894370741024400516, 3974503178830415392, 15372097428421020252, 7483713112720711445, 2828998250052557538]),
            ([4589662461355414621, 13139347841596464218, 962152761008126689, 710075215004952915, 16788534949442826519, 17683502669417353353, 5180777027377979083, 1135421800557266124], [4589662461355414621, 13139347841596464218, 962152761008126689, 710075215004952915, 16788534949442826519, 17683502669417353353, 5180777027377979083, 1135421800557266124], [10624804931769401360, 551319935632329526, 7317452444132248876, 2110137559037270516, 2034083139055508537, 2116540210735389357, 2827104053417142624, 439328336019907653]),
            ([4551190329934191115, 5290883087563985703, 2061029004947750652, 1334375469288976188, 13667728463137109732, 5077631535515934568, 15739039896056715557, 551195402005233247], [7898793943630705267, 7681944294170946983, 15959304187594925464, 2429043669677514534, 17492437480425935105, 4967575794797916110, 4792274987124465467, 576493471378070145], [15819254773394621861, 14099806438963072149, 1190662743003100347, 786975732675562272, 15280881440723407082, 5383863454786648652, 11182915632410109549, 3263630006552595985]),
            ([10684940457868621871, 3036276956815396163, 14205351223240002816, 2854414008099239884, 10188295851521720936, 10169501466838980956, 11495958091224756113, 1471856971456496756], [679337833971266369, 11667868982536972269, 14653972506809634165, 1496547500255247271, 8035562844108012488, 17650276092294811711, 6844352797576697991, 2554149026800610621], [3105333048173026763, 17547459503129843718, 323671755551434093, 2701314678460285803, 17054689186078373634, 2207815281730260316, 12415485287184072554, 230287063354574008]),
        ];
        for (p1, p2, p3) in VECTORS {
            let mut got = [0u64; 8];
            zisk_precomp_helpers::babyjubjub_add(p1, p2, &mut got);
            assert_eq!(&got, p3, "host helper diverged from babyjubjub-rs oracle");
            TwistedEdwards::verify_add(TwistedEdwardsCurve::BabyJubJub, p1, p2, p3);
        }
    }

    // RFC 8032 base point B, little-endian limbs (x || y).
    const ED25519_B: [u64; 8] = [
        14507833142362363162,
        7578651490590762930,
        13881468655802702940,
        2407515759118799870,
        7378697629483820632,
        7378697629483820646,
        7378697629483820646,
        7378697629483820646,
    ];
    const ED25519_IDENTITY: [u64; 8] = [0, 0, 0, 0, 1, 0, 0, 0];

    fn ed25519_add(p1: &[u64; 8], p2: &[u64; 8]) -> [u64; 8] {
        let mut out = [0u64; 8];
        zisk_precomp_helpers::ed25519_add(p1, p2, &mut out);
        out
    }

    /// k*B for k = 0..n (0*B is the identity).
    fn ed25519_multiples(n: usize) -> Vec<[u64; 8]> {
        let mut pts = vec![ED25519_IDENTITY];
        for i in 0..n {
            pts.push(ed25519_add(&pts[i], &ED25519_B));
        }
        pts
    }

    /// Adds p (< 2^256) to the 4-limb coordinate starting at `offset`.
    fn ed25519_alias(point: &[u64; 8], offset: usize) -> [u64; 8] {
        let p = bigint_from_u64s(point) + (&*ED25519_PRIME_BI << (64 * offset));
        let (_, digits) = p.to_u64_digits();
        let mut out = [0u64; 8];
        out[..digits.len()].copy_from_slice(&digits);
        out
    }

    // Executor result must match the host helper, including identity, doubling and P + (-P).
    #[test]
    fn ed25519_add_matches_host_helper() {
        let pts = ed25519_multiples(24);
        let mut neg_b = ED25519_B;
        let neg_x = &*ED25519_PRIME_BI - bigint_from_u64s(&ED25519_B[0..4]);
        let (_, digits) = neg_x.to_u64_digits();
        neg_b[0..4].copy_from_slice(&digits);
        assert_eq!(ed25519_add(&ED25519_B, &neg_b), ED25519_IDENTITY);

        let mut cases: Vec<([u64; 8], [u64; 8])> = vec![(ED25519_B, neg_b)];
        for p1 in &pts {
            for p2 in pts.iter().step_by(5) {
                cases.push((*p1, *p2));
            }
        }
        for (p1, p2) in cases {
            let expected = ed25519_add(&p1, &p2);
            TwistedEdwards::verify_add(TwistedEdwardsCurve::Ed25519, &p1, &p2, &expected);

            let mut got = [0u64; 8];
            TwistedEdwards::calculate_add(TwistedEdwardsCurve::Ed25519, &p1, &p2, &mut got);
            assert_eq!(got, expected, "calculate_add mismatch");
        }
    }

    // The air reads inputs from memory as given: x + p (still < 2^256) must give the same
    // (canonical) result, with every chunk, quotient and carry in range.
    #[test]
    fn ed25519_add_accepts_non_canonical_inputs() {
        let pts = ed25519_multiples(6);
        for p1 in &pts {
            for p2 in &pts {
                let expected = ed25519_add(p1, p2);
                let (a1, a2) = (ed25519_alias(p1, 0), ed25519_alias(&ed25519_alias(p2, 0), 4));
                assert_eq!(ed25519_add(&a1, &a2), expected);
                TwistedEdwards::verify_add(TwistedEdwardsCurve::Ed25519, &a1, &a2, &expected);
            }
        }
    }

    // Slots must match the airtemplate's sel_op order (BabyJubJub first) for curves: TE_ALL.
    #[test]
    fn curves_select_their_own_slot_and_prime() {
        assert_eq!(TwistedEdwardsCurve::BabyJubJub.sel_op(), 0);
        assert_eq!(TwistedEdwardsCurve::Ed25519.sel_op(), 1);
        assert_eq!(TwistedEdwardsCurve::Ed25519.prime_chunks()[15], 0x7FFF);
        assert_eq!(TwistedEdwardsCurve::BabyJubJub.prime_chunks()[15], 0x3064);
    }
}
