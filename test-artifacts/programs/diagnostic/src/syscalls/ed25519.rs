use ziskos::syscalls::*;

pub fn diagnostic_ed25519() {
    // RFC 8032 base point B and its multiples, little-endian limbs.
    let b = SyscallPoint256 {
        x: [14507833142362363162, 7578651490590762930, 13881468655802702940, 2407515759118799870],
        y: [7378697629483820632, 7378697629483820646, 7378697629483820646, 7378697629483820646],
    };
    let b2 = SyscallPoint256 {
        x: [9495172746435612174, 580276354371003487, 4396704544683452332, 3939304200807777388],
        y: [1035624154251437001, 10896757615647805782, 2135373527235305302, 2477206238767819202],
    };
    let b3 = SyscallPoint256 {
        x: [12421570298656645724, 7152727868506785926, 2302657123787130938, 7471080674075774793],
        y: [199117469010736340, 11463609486776992516, 11612146322091974391, 1326224128502294955],
    };

    //////////////
    // Ed25519 Dbl (complete law doubles when p1 == p2): B + B = 2B
    //////////////

    let mut p1 = SyscallPoint256 { x: b.x, y: b.y };
    let mut params = SyscallEd25519AddParams { p1: &mut p1, p2: &b };
    syscall_ed25519_add(&mut params);
    assert_eq!(params.p1.x, b2.x);
    assert_eq!(params.p1.y, b2.y);

    //////////////
    // Ed25519 Add: 2B + B = 3B
    //////////////

    let mut p1 = SyscallPoint256 { x: b2.x, y: b2.y };
    let mut params = SyscallEd25519AddParams { p1: &mut p1, p2: &b };
    syscall_ed25519_add(&mut params);
    assert_eq!(params.p1.x, b3.x);
    assert_eq!(params.p1.y, b3.y);

    //////////////
    // Inverse: B + (-B) == O = (0, 1), with -B = (p - x, y)
    //////////////

    let neg_b = SyscallPoint256 {
        x: [3938910931347188435, 10868092583118788685, 4565275417906848675, 6815856277735975937],
        y: b.y,
    };
    let mut p1 = SyscallPoint256 { x: b.x, y: b.y };
    let mut params = SyscallEd25519AddParams { p1: &mut p1, p2: &neg_b };
    syscall_ed25519_add(&mut params);
    assert_eq!(params.p1.x, [0, 0, 0, 0]);
    assert_eq!(params.p1.y, [1, 0, 0, 0]);
}
