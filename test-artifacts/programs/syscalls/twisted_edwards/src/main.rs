#![no_main]
ziskos::entrypoint!(main);

use ziskos::syscalls::{
    syscall_babyjubjub_add, syscall_ed25519_add, SyscallBabyJubJubAddParams,
    SyscallEd25519AddParams, SyscallPoint256,
};

fn main() {
    babyjubjub_tests();
    ed25519_tests();
}

fn babyjubjub_tests() {
    // Test #0: add two distinct points (circomlib test/babyjub.js "Should add two points").
    let mut p1 = SyscallPoint256 {
        x: [7699839232871213100, 11377594457309997372, 11223204053576433119, 2832127448816122262],
        y: [5014193860014684243, 12891415696119316661, 16351138293120604662, 418439792016356479],
    };
    let p2 = SyscallPoint256 {
        x: [8787143617238384983, 14452031502512802791, 7941460534247489132, 2635075998581887399],
        y: [3786921421244027607, 17990144301093405137, 1791269356123149699, 3316665278388780408],
    };
    syscall_babyjubjub_add(&mut p1, &p2);
    let p3 = SyscallPoint256 {
        x: [10405464339497284449, 8277032624866824417, 12950120428123253884, 1261101424013096072],
        y: [11601515926616747027, 759323840248000415, 18014248375850445194, 2235942773966081583],
    };
    assert_eq!(p1.x, p3.x);
    assert_eq!(p1.y, p3.y);

    // Test #1: doubling (same point twice — the complete law doubles): 2*p1.
    let mut p1 = SyscallPoint256 {
        x: [7699839232871213100, 11377594457309997372, 11223204053576433119, 2832127448816122262],
        y: [5014193860014684243, 12891415696119316661, 16351138293120604662, 418439792016356479],
    };
    let p2 = SyscallPoint256 {
        x: [7699839232871213100, 11377594457309997372, 11223204053576433119, 2832127448816122262],
        y: [5014193860014684243, 12891415696119316661, 16351138293120604662, 418439792016356479],
    };
    syscall_babyjubjub_add(&mut p1, &p2);
    let p3 = SyscallPoint256 {
        x: [6573998244250000053, 15447716325968879609, 5178469352008426472, 1097776659210999491],
        y: [5726854610579821793, 16363202296762874374, 6162349764539256418, 691182090570128499],
    };
    assert_eq!(p1.x, p3.x);
    assert_eq!(p1.y, p3.y);

    // Test #2: identity (0, 1) is the neutral element: P + O == P.
    let mut p1 = SyscallPoint256 {
        x: [7699839232871213100, 11377594457309997372, 11223204053576433119, 2832127448816122262],
        y: [5014193860014684243, 12891415696119316661, 16351138293120604662, 418439792016356479],
    };
    let o = SyscallPoint256 { x: [0, 0, 0, 0], y: [1, 0, 0, 0] };
    syscall_babyjubjub_add(&mut p1, &o);
    assert_eq!(
        p1.x,
        [7699839232871213100, 11377594457309997372, 11223204053576433119, 2832127448816122262]
    );
    assert_eq!(
        p1.y,
        [5014193860014684243, 12891415696119316661, 16351138293120604662, 418439792016356479]
    );
}

// RFC 8032 base point B and its multiples, little-endian limbs.
const ED_B: SyscallPoint256 = SyscallPoint256 {
    x: [14507833142362363162, 7578651490590762930, 13881468655802702940, 2407515759118799870],
    y: [7378697629483820632, 7378697629483820646, 7378697629483820646, 7378697629483820646],
};
const ED_B2: SyscallPoint256 = SyscallPoint256 {
    x: [9495172746435612174, 580276354371003487, 4396704544683452332, 3939304200807777388],
    y: [1035624154251437001, 10896757615647805782, 2135373527235305302, 2477206238767819202],
};
const ED_B3: SyscallPoint256 = SyscallPoint256 {
    x: [12421570298656645724, 7152727868506785926, 2302657123787130938, 7471080674075774793],
    y: [199117469010736340, 11463609486776992516, 11612146322091974391, 1326224128502294955],
};
const ED_IDENTITY: SyscallPoint256 = SyscallPoint256 { x: [0, 0, 0, 0], y: [1, 0, 0, 0] };

fn ed25519_add(p1: &SyscallPoint256, p2: &SyscallPoint256) -> SyscallPoint256 {
    let mut p3 = SyscallPoint256 { x: p1.x, y: p1.y };
    let mut params = SyscallEd25519AddParams { p1: &mut p3, p2 };
    syscall_ed25519_add(&mut params);
    p3
}

fn assert_point_eq(got: &SyscallPoint256, expected: &SyscallPoint256) {
    assert_eq!(got.x, expected.x);
    assert_eq!(got.y, expected.y);
}

fn ed25519_tests() {
    // Test #0: doubling (the complete law doubles): B + B = 2B.
    assert_point_eq(&ed25519_add(&ED_B, &ED_B), &ED_B2);

    // Test #1: add two distinct points: 2B + B = 3B (and commutes).
    assert_point_eq(&ed25519_add(&ED_B2, &ED_B), &ED_B3);
    assert_point_eq(&ed25519_add(&ED_B, &ED_B2), &ED_B3);

    // Test #2: identity (0, 1) is the neutral element: B + O == B, O + O == O.
    assert_point_eq(&ed25519_add(&ED_B, &ED_IDENTITY), &ED_B);
    assert_point_eq(&ed25519_add(&ED_IDENTITY, &ED_IDENTITY), &ED_IDENTITY);

    // Test #3: inverse: B + (-B) == O, with -B = (p - x, y).
    let neg_b = SyscallPoint256 {
        x: [3938910931347188435, 10868092583118788685, 4565275417906848675, 6815856277735975937],
        y: ED_B.y,
    };
    assert_point_eq(&ed25519_add(&ED_B, &neg_b), &ED_IDENTITY);

    // Test #4: non-canonical input: x + p (still < 2^256) gives the same, canonical, result.
    let b_alias = SyscallPoint256 {
        x: [14507833142362363143, 7578651490590762930, 13881468655802702940, 11630887795973575678],
        y: ED_B.y,
    };
    assert_point_eq(&ed25519_add(&b_alias, &ED_B), &ED_B2);
}
