#[cfg(not(feature = "ziskasm"))]
use ziskos::zisklib::ecdsa_verify_secp256k1;
// ziskasm: the flat binding has identical signature — a plain rename.
#[cfg(feature = "ziskasm")]
use zisklib::secp256k1_ecdsa_verify as ecdsa_verify_secp256k1;

pub fn ecdsa_tests() {
    // Verify (valids)
    let pk = [
        0x3bcfdc2aca47e0f2,
        0xa739d5cc6b89e9b5,
        0x35b73cc431afc6bc,
        0xe1ea4273f638d4ae,
        0xc6402318ee33448e,
        0x9f18c242b8df8bb6,
        0x934a8dfdd797e1c4,
        0x3840aa9c4d86557e,
    ];
    let z = [0x1bf86a1816a52f52, 0xd31e26c3da73dda8, 0xa3b71997594da038, 0x17560495f6944673];
    let r = [0x68df7d8d7e0fb36b, 0xc2189fe681cd6e78, 0xc85ba1fd6238ecb5, 0x3e125456c8338994];
    let s = [0xd4e89d1ae75aeea2, 0xb8e33178783bd1a3, 0x866acebc9e141ec, 0x3a816b1c33739e41];
    let res = ecdsa_verify_secp256k1(&pk, &z, &r, &s);
    assert!(res);

    let pk = [
        0x563cc8bd265493e8,
        0xe26ca51d73493bb1,
        0xbaf413f523859dc3,
        0x8977dd29be12980c,
        0x576f2adf398ee11b,
        0xe5cf41decfb89b8b,
        0x520997ce18f48f65,
        0x5ca08a9cbf10b944,
    ];
    let z = [0xab313c87e8099e4d, 0x98e191bbf16e2538, 0x5fad2b6370bed1a0, 0xbda0ec6db2b2f290];
    let r = [0x320ca78102f69537, 0x548ce6ea0e9f500c, 0xac0d2ea7c5b22534, 0x560ef25130dd8a78];
    let s = [0xdfd826f31fb981d0, 0x48fed994c3e102e5, 0xc2ffab95e65eef0f, 0x18b94747713e154f];
    let res = ecdsa_verify_secp256k1(&pk, &z, &r, &s);
    assert!(res);

    // 58 generated secp256k1 ECDSA vectors (pure-python reference).
    // Exercises the GLV 4-way double-scalar ladder across diverse u1/u2, plus
    // z==0 (u1==0) edges and reject paths.
    // valid #0 z=0
    let pk = [0xa2f8fce8909d0e49, 0x0b91f39a7be7960c, 0x6abce7e99d4436db, 0x346ee4ceb5e260f3, 0x95c226b3ca09ff40, 0x5fd5f3aaf898969c, 0xf3e71f27a32fd808, 0xee66db47656bb160];
    let z = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r = [0x9a7cad1ee06d50cf, 0xb759026b1f3c527d, 0xbfd6b956e9af4437, 0x4f637a518868258c];
    let s = [0x79e6b2f741419e9c, 0xe46bb012731fb993, 0x0c86f1763ba9dca7, 0x7baed8be557e09dc];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-s #0
    let pk = [0xa2f8fce8909d0e49, 0x0b91f39a7be7960c, 0x6abce7e99d4436db, 0x346ee4ceb5e260f3, 0x95c226b3ca09ff40, 0x5fd5f3aaf898969c, 0xf3e71f27a32fd808, 0xee66db47656bb160];
    let z = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r = [0x9a7cad1ee06d50cf, 0xb759026b1f3c527d, 0xbfd6b956e9af4437, 0x4f637a518868258c];
    let s = [0x79e6b2f741419e9d, 0xe46bb012731fb993, 0x0c86f1763ba9dca7, 0x7baed8be557e09dc];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-z #0
    let pk = [0xa2f8fce8909d0e49, 0x0b91f39a7be7960c, 0x6abce7e99d4436db, 0x346ee4ceb5e260f3, 0x95c226b3ca09ff40, 0x5fd5f3aaf898969c, 0xf3e71f27a32fd808, 0xee66db47656bb160];
    let z = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r = [0x9a7cad1ee06d50cf, 0xb759026b1f3c527d, 0xbfd6b956e9af4437, 0x4f637a518868258c];
    let s = [0x79e6b2f741419e9c, 0xe46bb012731fb993, 0x0c86f1763ba9dca7, 0x7baed8be557e09dc];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #1
    let pk = [0xb578c22ced929a7d, 0x28f2bfe9845b54e9, 0x3cb96356730a4360, 0xea8f1c7a690f86c1, 0x6be42e197f1bba0a, 0xa3a4396d29ad6353, 0x5bc3462b06ec2565, 0x9a1d9c8b03f2c9ab];
    let z = [0x684e43f682e59bb2, 0x2a44e10eaa631b21, 0xc6536928ec508e84, 0x00dc09dda1fc617e];
    let r = [0x2e1a462dc82c4044, 0x3950724ab56e3c25, 0xc057c31809192b97, 0xd1bcd09c5978c981];
    let s = [0x424dc5852b920b02, 0x9381a1ce1b8aafec, 0x9687a3872122a729, 0x2a8d1d2573a69015];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #2
    let pk = [0x0fadf51ea7d17388, 0xbab6051773e13b7f, 0x16fd6209aaf5e8a4, 0xa7685185af3e4e70, 0x224b32c7a4112cb0, 0x1eb8dc629328f7e1, 0x2aa2cd90b3f487e9, 0x7858aeb387f11806];
    let z = [0x0787c6e8f6a3aab6, 0xe73ea63a6a97ba4a, 0x50a31fa285132d22, 0x1db75a93b71cb7a5];
    let r = [0x887ced453eb2a868, 0x9582212e56580a46, 0xd349430adfbf1a77, 0x8f45e8d955f854f0];
    let s = [0xe9aa057251a1d783, 0x0a8162056b07b6c3, 0xa4e6d15c9bfe9139, 0x693f32c5b58f3d68];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #3
    let pk = [0xec50cf8983be8e6a, 0xab2a4176b7af28b6, 0x49a7ecc3610e03a5, 0xf0cccac61a92a7d3, 0xdc42694bd82e4836, 0x7ae058fa375e0935, 0x63cfc51a9e30f0b9, 0xc76fe7762e155725];
    let z = [0x3a1fd73a8027b1a3, 0xad75e989adcc4a7f, 0xe9c4979e82e1915c, 0x5d589723fa1405be];
    let r = [0xfb83cab6dbcbae0c, 0xadb0e8e2f4f1381b, 0xda9db2fc592a38e5, 0x5756f6755741c308];
    let s = [0xe5ede7af2cb3cfef, 0x8a67567985c417db, 0xfcd96c339562d299, 0x2c6eb585008058d0];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #4
    let pk = [0x9a0bc3394cbb042c, 0x88540e755af9271d, 0x89445aa9687ef106, 0x5d3f1ba4a05cd7d8, 0x8062101f3f4760d6, 0x7c78177949790523, 0x41fd12f07bf85513, 0xca9f6f3e9b4ed047];
    let z = [0x27aa3ede2525e0ac, 0xd50477e0af797fe5, 0xc79865ed8768a694, 0xe4f8b9b2fda7d682];
    let r = [0x953f2f2c8ff214dc, 0x737cea0572260b94, 0x8654b2e6598a1e75, 0x7b7e2d70e4f68d4c];
    let s = [0x4586df444763614c, 0x0e9f3c2958cdfcbb, 0x1f8cd8919d1aaa54, 0x568eadae58e4c4d5];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-s #4
    let pk = [0x9a0bc3394cbb042c, 0x88540e755af9271d, 0x89445aa9687ef106, 0x5d3f1ba4a05cd7d8, 0x8062101f3f4760d6, 0x7c78177949790523, 0x41fd12f07bf85513, 0xca9f6f3e9b4ed047];
    let z = [0x27aa3ede2525e0ac, 0xd50477e0af797fe5, 0xc79865ed8768a694, 0xe4f8b9b2fda7d682];
    let r = [0x953f2f2c8ff214dc, 0x737cea0572260b94, 0x8654b2e6598a1e75, 0x7b7e2d70e4f68d4c];
    let s = [0x4586df444763614d, 0x0e9f3c2958cdfcbb, 0x1f8cd8919d1aaa54, 0x568eadae58e4c4d5];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #5
    let pk = [0x8df9333dcbf4768e, 0x5040cef3f6f6b265, 0x22724ad6699a98fe, 0x61d25b19ec7183ff, 0x64980f526a4a7d81, 0x148b82d7dba4379e, 0xad8d77b4f1b3abda, 0x1c286215817cd11f];
    let z = [0x54554420c89049c7, 0x172d65da42fb802d, 0xa6265fc59525aa1b, 0x9685b3998063691c];
    let r = [0x2294186fd8bc8b15, 0xd6ac3deb13a6b4de, 0x03ff72ed1b3c79c6, 0x01f94dc679457c45];
    let s = [0xe2e92a89e0847f6f, 0x4fb139911960474e, 0x188d1bbe3dd4dcbe, 0x2ed71bafe6991259];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-z #5
    let pk = [0x8df9333dcbf4768e, 0x5040cef3f6f6b265, 0x22724ad6699a98fe, 0x61d25b19ec7183ff, 0x64980f526a4a7d81, 0x148b82d7dba4379e, 0xad8d77b4f1b3abda, 0x1c286215817cd11f];
    let z = [0x54554420c89049c8, 0x172d65da42fb802d, 0xa6265fc59525aa1b, 0x9685b3998063691c];
    let r = [0x2294186fd8bc8b15, 0xd6ac3deb13a6b4de, 0x03ff72ed1b3c79c6, 0x01f94dc679457c45];
    let s = [0xe2e92a89e0847f6f, 0x4fb139911960474e, 0x188d1bbe3dd4dcbe, 0x2ed71bafe6991259];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #6
    let pk = [0xc31c8aa434ee45ba, 0x1a5052e886e398fd, 0x11e0b9f26151029f, 0x3c9740a547e91276, 0x969347f604de7902, 0x96306b755dbfa535, 0xb255b95ee2bcd3b7, 0x449047090bad93d1];
    let z = [0x2b631f1d67a57add, 0x657525f0b341de48, 0xab78d713a7a9f2c6, 0x2de088a3acaf531e];
    let r = [0x1768a3e2aed469dc, 0x57bc39e232688f87, 0xd4209e4c3a75fc5e, 0x387393aacac4386f];
    let s = [0x2fca13c2eb80007f, 0xe79d3da75f020794, 0x79d87cb5f4cff7d9, 0x7cf0bc39207ac453];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #7 z=0
    let pk = [0x53b763937e538e7c, 0x05edf101a13ec4b5, 0x8995740747d5761f, 0x3a7a9b49a654f0e7, 0xefad194b8cd904df, 0xaea43fcd9cf9a47a, 0x85be61169788cc0f, 0x40301ac5e47f921e];
    let z = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r = [0x008754434e50710c, 0x84d82a71d3a74b05, 0x3b5bc19b9eee259b, 0x55b0b6f7b0b35eed];
    let s = [0x54aa85bb007b2fbb, 0xe034c8e0ec02800c, 0x19a8621948fb6f14, 0x642913c5be6497b5];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #8
    let pk = [0xf40d0c0de12e055b, 0xe58165c8e5bb5cb0, 0x8ba0dcd0323fac88, 0xd6e4c6921ee23535, 0xa2fbe26a148257be, 0x3ba860d3412c6360, 0xe58e56bdbe367960, 0xbfc8bc891379aaf5];
    let z = [0xb1b9ccce662734b0, 0x214c37ab0cf67d6f, 0x1bab15fae95307d6, 0x055143822a607287];
    let r = [0x2deaa62986adc55f, 0x2f970bfa59ec057b, 0x173059734b8d0c81, 0x2c7c1d1b2e7836c3];
    let s = [0xc4441697bd861a55, 0x3d1d9dc3b239c802, 0x7243425ebf764d77, 0x10b88028bdede833];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-s #8
    let pk = [0xf40d0c0de12e055b, 0xe58165c8e5bb5cb0, 0x8ba0dcd0323fac88, 0xd6e4c6921ee23535, 0xa2fbe26a148257be, 0x3ba860d3412c6360, 0xe58e56bdbe367960, 0xbfc8bc891379aaf5];
    let z = [0xb1b9ccce662734b0, 0x214c37ab0cf67d6f, 0x1bab15fae95307d6, 0x055143822a607287];
    let r = [0x2deaa62986adc55f, 0x2f970bfa59ec057b, 0x173059734b8d0c81, 0x2c7c1d1b2e7836c3];
    let s = [0xc4441697bd861a56, 0x3d1d9dc3b239c802, 0x7243425ebf764d77, 0x10b88028bdede833];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #9
    let pk = [0xd4b78202a78eec61, 0xfd71d40f06680f22, 0x44a4f27226059d22, 0x647e591f66bf02b7, 0xb28e7b1ba441dc1b, 0x45bc3057941bf38b, 0xaed28cc9cf672357, 0x3615c1abd222ab9a];
    let z = [0xde2795ed805ee66e, 0x9bbae67efef230e7, 0xa9085d45e73821db, 0x113789613cc0acf8];
    let r = [0x9c36a452c1abcf4f, 0xfa5f38f45344b2c8, 0x46a8f13418f631c5, 0x5d89a82950522d04];
    let s = [0xbe30f83843d2af29, 0xb9147e0b00df8df8, 0xd915bd420606e5ba, 0x588a24e4caac1910];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #10
    let pk = [0x405136cad3f16882, 0x5e925f21c5dc9139, 0x6eef2299d8abeade, 0x6d72d29ab0ed69cd, 0xa756bf174ad74fe7, 0x2ac479abd72212a9, 0xb138c73f9f4e6a52, 0xd6d91379fee391f2];
    let z = [0x4a8a03a20ac4461f, 0xd989cf0c8fb9484d, 0x049c53449bb5f1de, 0xd4c01683cd25f90a];
    let r = [0xf2612454b4a9d262, 0x58ce59938c19e4bd, 0x11cf66108bb044b8, 0xdba55b6ca5808213];
    let s = [0x3085166f59525e8f, 0x8892c0754d55641f, 0x8bfccd5affd1b4f3, 0x6cce180bbb758a0c];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-z #10
    let pk = [0x405136cad3f16882, 0x5e925f21c5dc9139, 0x6eef2299d8abeade, 0x6d72d29ab0ed69cd, 0xa756bf174ad74fe7, 0x2ac479abd72212a9, 0xb138c73f9f4e6a52, 0xd6d91379fee391f2];
    let z = [0x4a8a03a20ac44620, 0xd989cf0c8fb9484d, 0x049c53449bb5f1de, 0xd4c01683cd25f90a];
    let r = [0xf2612454b4a9d262, 0x58ce59938c19e4bd, 0x11cf66108bb044b8, 0xdba55b6ca5808213];
    let s = [0x3085166f59525e8f, 0x8892c0754d55641f, 0x8bfccd5affd1b4f3, 0x6cce180bbb758a0c];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #11
    let pk = [0x844913c2543a46aa, 0x9f960d4ba37ceed7, 0x64466dd2a366e934, 0x2f92b8b677665362, 0x72a843e8b4710d39, 0x0b66c36c5816261c, 0xd07cabb7fe6ef45d, 0xd9060481c838f825];
    let z = [0x035346b53f350ca4, 0xec7a1989314ecfd1, 0x927740771556372a, 0x986905c278a763fc];
    let r = [0x4ffbadd11df90c0a, 0x8403450a07588f10, 0xa1c185ef2e4f88ac, 0x97a31ef3c0850831];
    let s = [0x8424f89f56bd70dd, 0xf0ce824946da0e26, 0xc6bc6ebf096a7123, 0x5884fe53d48ae406];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #12
    let pk = [0xeb65769ec893b447, 0xb5ec10b21a59f869, 0x2212105ff5b6483e, 0xfb6ef8a59cb5efac, 0x453224027c4b8b74, 0x7fe4e42d50da8cf5, 0xb3f40b4322a619d6, 0x938b27aa2fd03d05];
    let z = [0x8d25ecbd5b3894a5, 0xde288890a50e49b7, 0xe397e316c900db0c, 0xbb4b265b80cd3d96];
    let r = [0x174425cf8b4e41f0, 0xabf6b29eaa57abb1, 0xf33522e85654567b, 0xdd60c674cf588e44];
    let s = [0xa0aab59119cfd87f, 0x166c57210edf836f, 0xb95026b321cd5627, 0x7c2880ee32682776];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-s #12
    let pk = [0xeb65769ec893b447, 0xb5ec10b21a59f869, 0x2212105ff5b6483e, 0xfb6ef8a59cb5efac, 0x453224027c4b8b74, 0x7fe4e42d50da8cf5, 0xb3f40b4322a619d6, 0x938b27aa2fd03d05];
    let z = [0x8d25ecbd5b3894a5, 0xde288890a50e49b7, 0xe397e316c900db0c, 0xbb4b265b80cd3d96];
    let r = [0x174425cf8b4e41f0, 0xabf6b29eaa57abb1, 0xf33522e85654567b, 0xdd60c674cf588e44];
    let s = [0xa0aab59119cfd880, 0x166c57210edf836f, 0xb95026b321cd5627, 0x7c2880ee32682776];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #13
    let pk = [0xedddd06db5817876, 0x556fc5f0b5331d3b, 0x52175de978f4d805, 0xb769593a5641a06f, 0xa0a2f3c47d7b56cb, 0x24b98a792b9c6a30, 0xc4f858799479fb77, 0x151d4b4736eb37c4];
    let z = [0x12a7070e1ca07346, 0x96a83bd7ff274ab6, 0x433849afd5b2035c, 0x9152445b77fc3660];
    let r = [0x3b304b2b42ffef39, 0xf3235842fb24ffee, 0xa8813d73b214dea2, 0x23a38d1c9f46f26e];
    let s = [0xe2c721c953dd2af5, 0x18e8706e2267b062, 0x2c1c7c5992a925ea, 0x4c2cb9feb6429df8];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #14 z=0
    let pk = [0x92acaa1177f78c5f, 0xab51ff1df2a22b5d, 0xaa2e654317d18e0f, 0xb198a202d3f669e9, 0xfac5e7b0f6c0faf6, 0xea7d93ebe3fe83aa, 0x6a66346b41d28096, 0x24250d461847e248];
    let z = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r = [0x81bb7848dea46d68, 0x7fed4ece223e89fe, 0xb096a7d4f65f2ab6, 0x2f5f8626f78686b4];
    let s = [0xcddb0c4c675b5efc, 0x4e9e67c1d01034d6, 0xdee7361c4e8b18b8, 0x5c03b9c39286787d];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #15
    let pk = [0x518173273c1ed038, 0xcec2b26bc4f43b50, 0x72eb1dfcb6e82810, 0x2165f130a58a297a, 0xf0a490d1b8099012, 0x86e4803d5aa95c42, 0x44ab1e336c765c53, 0xd6c88a79e2a82f1c];
    let z = [0xae40e0f6e2a80a72, 0xb0cf26915131cea0, 0x03e400142136ae1e, 0x32226dddb7a9977f];
    let r = [0xfe848a3af4c82ff5, 0x69834c67153ab7b8, 0x0f8e77e0e90cd93a, 0x7b82e3facf7bf5a3];
    let s = [0xc7befc6f4acc0f3d, 0xcd53c9e765f4d868, 0xc5a65c4a9a062fec, 0x405ea130f64f7365];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-z #15
    let pk = [0x518173273c1ed038, 0xcec2b26bc4f43b50, 0x72eb1dfcb6e82810, 0x2165f130a58a297a, 0xf0a490d1b8099012, 0x86e4803d5aa95c42, 0x44ab1e336c765c53, 0xd6c88a79e2a82f1c];
    let z = [0xae40e0f6e2a80a73, 0xb0cf26915131cea0, 0x03e400142136ae1e, 0x32226dddb7a9977f];
    let r = [0xfe848a3af4c82ff5, 0x69834c67153ab7b8, 0x0f8e77e0e90cd93a, 0x7b82e3facf7bf5a3];
    let s = [0xc7befc6f4acc0f3d, 0xcd53c9e765f4d868, 0xc5a65c4a9a062fec, 0x405ea130f64f7365];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #16
    let pk = [0x653a2a85fb482e7e, 0xac8e47065ae3232e, 0xc2bb6b6066286a81, 0x0a0104dbe9a9192d, 0x1786bdb83ffc18b1, 0xada99542aa29b658, 0x51521f38ea785af3, 0x50e3575330efcf52];
    let z = [0xc593805a4c586f6e, 0x191c28be17062472, 0xdd7dc1b6e7c7d05f, 0x985681ccef6d5a34];
    let r = [0xc563164fcd4700da, 0xd6116da666245500, 0x943bca5e244987c1, 0x172d510d5a6538e1];
    let s = [0xfc30a68c19e291e9, 0x3bc361819208b6b7, 0x99c8ef1e6a20b918, 0x6319447f13e7172e];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-s #16
    let pk = [0x653a2a85fb482e7e, 0xac8e47065ae3232e, 0xc2bb6b6066286a81, 0x0a0104dbe9a9192d, 0x1786bdb83ffc18b1, 0xada99542aa29b658, 0x51521f38ea785af3, 0x50e3575330efcf52];
    let z = [0xc593805a4c586f6e, 0x191c28be17062472, 0xdd7dc1b6e7c7d05f, 0x985681ccef6d5a34];
    let r = [0xc563164fcd4700da, 0xd6116da666245500, 0x943bca5e244987c1, 0x172d510d5a6538e1];
    let s = [0xfc30a68c19e291ea, 0x3bc361819208b6b7, 0x99c8ef1e6a20b918, 0x6319447f13e7172e];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #17
    let pk = [0xbcd1c1149c70a2c2, 0x60e6c973a5783a5f, 0x2024b1893b44fd54, 0x32d8cab5dd65ece5, 0x107addd28e7e14e1, 0x1aa3e02258bceb12, 0xd214a3b1f3ad8f2d, 0xf348bf2085c8e96b];
    let z = [0x3bdd8177d7736a8d, 0x991dfa401ed99dd7, 0xce0ffff8940e8fb5, 0xf323a672b6257bc8];
    let r = [0x92767fa3d42aeb5e, 0x1f707e1596a24292, 0xb3d3247af01eff32, 0xd67a2316b5bd2f30];
    let s = [0x246c017c0ec5870f, 0xb5c03825efd598a0, 0xc4c0fa8cb99ac6ab, 0x223867983bd17ded];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #18
    let pk = [0xf38f539ee0414e71, 0x140cce12f17b4724, 0xa4e96be48acf4cc1, 0x6b281577c56c116c, 0x537a3b546344c9c5, 0x3dafea12ccfb5043, 0x6424bf8e0e575430, 0x79eb59db81b4a75d];
    let z = [0x19fba1bbcc5f236c, 0x08e7d99f1d39e000, 0xba051557cdb72fc8, 0xf8b69098b569b3f8];
    let r = [0xb5edc13ce026dbe1, 0x152943a5164db3ea, 0xdbd238e3996bfd0b, 0x466789de64d5cbae];
    let s = [0xd216fc0f5b4d819f, 0x77a97f5679e07efe, 0xae9c13270a10423e, 0x1ab97d287ccb7cfe];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #19
    let pk = [0x3ec189a11fde3f63, 0xa73dd59c51c71304, 0xbab6e07704a82da4, 0xed7d34c7c1aecdf3, 0x3ede61cbd0acd6c7, 0x49af78faeef18b89, 0x269296db5eb014ce, 0xf30ad7b3b87896cf];
    let z = [0x84d728fed8dc29e1, 0x478c2bc207f6b39c, 0xa1a9d9447736b3f0, 0xd6e0acafc2912461];
    let r = [0x20c5d206c1ade9be, 0xad85f98d1bd7cb76, 0x97ff2ed54d5a93ed, 0xb232c2fcf85552da];
    let s = [0xc5c67dc19e3dfba0, 0x0f356bf596796ed4, 0x85ca4585ce921097, 0x411c0ffbd8d34b0f];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #20
    let pk = [0x3580b6ad8e9fb56b, 0xebc9b28672d460b1, 0x024e7d40fd997720, 0xeab666dd28ab8269, 0x7ad31b252d7e7497, 0xeb05c9a903494819, 0xb5062c52c6665fff, 0xd2fcd0dda80dd4b5];
    let z = [0x5b30639d252f3cd6, 0x8e52ba949596643d, 0x7c5e5e5e3d22829b, 0x6b978e05eac86f70];
    let r = [0xfdc67291a944a439, 0x44d0e6a8fbc47606, 0xac7d532240da6368, 0x1ebe09b912b34bae];
    let s = [0x1791cc80c18b27ef, 0x0d73d630f9b9953b, 0x407f6d9971e4047b, 0x74ad63dfaa17be80];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-s #20
    let pk = [0x3580b6ad8e9fb56b, 0xebc9b28672d460b1, 0x024e7d40fd997720, 0xeab666dd28ab8269, 0x7ad31b252d7e7497, 0xeb05c9a903494819, 0xb5062c52c6665fff, 0xd2fcd0dda80dd4b5];
    let z = [0x5b30639d252f3cd6, 0x8e52ba949596643d, 0x7c5e5e5e3d22829b, 0x6b978e05eac86f70];
    let r = [0xfdc67291a944a439, 0x44d0e6a8fbc47606, 0xac7d532240da6368, 0x1ebe09b912b34bae];
    let s = [0x1791cc80c18b27f0, 0x0d73d630f9b9953b, 0x407f6d9971e4047b, 0x74ad63dfaa17be80];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-z #20
    let pk = [0x3580b6ad8e9fb56b, 0xebc9b28672d460b1, 0x024e7d40fd997720, 0xeab666dd28ab8269, 0x7ad31b252d7e7497, 0xeb05c9a903494819, 0xb5062c52c6665fff, 0xd2fcd0dda80dd4b5];
    let z = [0x5b30639d252f3cd7, 0x8e52ba949596643d, 0x7c5e5e5e3d22829b, 0x6b978e05eac86f70];
    let r = [0xfdc67291a944a439, 0x44d0e6a8fbc47606, 0xac7d532240da6368, 0x1ebe09b912b34bae];
    let s = [0x1791cc80c18b27ef, 0x0d73d630f9b9953b, 0x407f6d9971e4047b, 0x74ad63dfaa17be80];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #21 z=0
    let pk = [0xdad6f54eaab5d6c3, 0xf86d835b842ce6da, 0xe92d65d5ac447c1e, 0xb655cc9402c02c8e, 0xc26476105a03eab4, 0x8e2ace237b0e12e2, 0x3500c027f7d4d663, 0x00c350e233ee11b4];
    let z = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r = [0x069b108d27382814, 0xf0f5e39a57b93338, 0xee0f0a8b0b319bb3, 0x3c7cea8a0f21dfb0];
    let s = [0xbf9d6494ed5bf378, 0x5cdab285078c2f56, 0xcbfc8823b9d72bf0, 0x3967561959aa0a56];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #22
    let pk = [0xbd8a994fdd177038, 0x19c04e1509280566, 0x65a51868c9d2b5c9, 0xa851ce25508441c1, 0xf7e201035b28f1ae, 0x74e1d54318ec88a6, 0x2f472c9d5bb7f08d, 0xe62176ab97087695];
    let z = [0x6777c7d58b6d46e5, 0x69e890c13afef9e0, 0xd559dbcbbd5b9c7a, 0xd789c4b5251d9ed7];
    let r = [0xd7ed7bf4f91d2be8, 0x852121dbae7dd59f, 0x89ed9b66ad09d150, 0x98886d7ff99f515c];
    let s = [0x7bb5fd9dde5e4eeb, 0xb86d18e7d138d100, 0x570051baed3e6b4e, 0x256991e17409007d];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #23
    let pk = [0xd379a6b65d28073e, 0x00caa332a61c6066, 0x98f76c17e6bc30b6, 0xa99a50559ef1b198, 0x4a3a5b99f9f45f9e, 0xed4dc76e4c3f12be, 0xe4b8cc4a964c3355, 0x2718706265ed3079];
    let z = [0x8e77f7ee19de76e9, 0x42c1c5ee5de8bb90, 0xfa980dc24e966acb, 0xac5c97a11d343945];
    let r = [0x0b860107506cfc03, 0x18a59e46c00a4483, 0x90662f1483795827, 0x7b65aeca5e220b55];
    let s = [0x7f8b7c056fd0b638, 0x07b9df9c34edf2d5, 0xbebaf6808c098734, 0x7c6817ea509dadee];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #24
    let pk = [0x32ec92df57652b73, 0x390d18ce53a91346, 0x219394d4bd77beb7, 0xb14b94b9d696dd42, 0x8582e2946eb84c07, 0x4ed181607b954e3b, 0x224c74774f260fc1, 0x8cd14eda2114d94c];
    let z = [0xf8abb8893bad7595, 0xb64d4c0d181291e8, 0xc1eee040f0139a45, 0xf27a7c188b626844];
    let r = [0xd01ab759e9e10d04, 0x99c29623322061ec, 0x7d9147f5626015c2, 0xe20e725fc167e77f];
    let s = [0x72299d4d95ce7fd6, 0xc8a5bc4b79925932, 0x2a1ee200acb32137, 0x29607827ab3b88cc];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-s #24
    let pk = [0x32ec92df57652b73, 0x390d18ce53a91346, 0x219394d4bd77beb7, 0xb14b94b9d696dd42, 0x8582e2946eb84c07, 0x4ed181607b954e3b, 0x224c74774f260fc1, 0x8cd14eda2114d94c];
    let z = [0xf8abb8893bad7595, 0xb64d4c0d181291e8, 0xc1eee040f0139a45, 0xf27a7c188b626844];
    let r = [0xd01ab759e9e10d04, 0x99c29623322061ec, 0x7d9147f5626015c2, 0xe20e725fc167e77f];
    let s = [0x72299d4d95ce7fd7, 0xc8a5bc4b79925932, 0x2a1ee200acb32137, 0x29607827ab3b88cc];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #25
    let pk = [0xfdac7a91aa38dd05, 0x8f3f7238c0141da5, 0x84cf97dd85cb4c22, 0x4f5b6e2f16259565, 0x58dfd5ccb19c7377, 0xc8e86950c8b6bd1b, 0x50c605bcef5a5df9, 0x1520706987fca191];
    let z = [0x9a4854004e41267d, 0x2124ce4d69b5376f, 0x74e33b623455e00e, 0x54b7a5d9fdc037a5];
    let r = [0xb67c2255716ad275, 0xcf2deea158239d45, 0x500752295e938807, 0x284820796a4056fc];
    let s = [0x33be23f5d508ea15, 0xd625c8126c4f6b90, 0x4f2d5fa8be15acc1, 0x301b2f58d47b438b];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-z #25
    let pk = [0xfdac7a91aa38dd05, 0x8f3f7238c0141da5, 0x84cf97dd85cb4c22, 0x4f5b6e2f16259565, 0x58dfd5ccb19c7377, 0xc8e86950c8b6bd1b, 0x50c605bcef5a5df9, 0x1520706987fca191];
    let z = [0x9a4854004e41267e, 0x2124ce4d69b5376f, 0x74e33b623455e00e, 0x54b7a5d9fdc037a5];
    let r = [0xb67c2255716ad275, 0xcf2deea158239d45, 0x500752295e938807, 0x284820796a4056fc];
    let s = [0x33be23f5d508ea15, 0xd625c8126c4f6b90, 0x4f2d5fa8be15acc1, 0x301b2f58d47b438b];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #26
    let pk = [0xa5d9106390b39c92, 0xde2e029bf319c7e7, 0x0ac66c258f84d179, 0xad4e705b16a06e3c, 0xa2067b3e24e00179, 0x57cf98c9017f9039, 0x917217563da5b16d, 0x8f22b1ee50a3e96f];
    let z = [0x8139d05d4a6597f8, 0xcb81d09a93b31c82, 0x2deab36c0542db13, 0xbdaed017bb6ec231];
    let r = [0xc429827aec13f9d2, 0xee5498769af0adfe, 0xb2cbfb9030ba15b0, 0xecb3046aa81ca4bd];
    let s = [0xae3035642bf454fa, 0xfd35f550532bc136, 0xdefc8414b9e109d3, 0x65c9d6634fd61253];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #27
    let pk = [0xd71c6554fb7b5135, 0x830776b348c19721, 0x1dc1d8c616c317af, 0x6acfa44d53a986a4, 0x70d86bb59dda151e, 0x4320b34ff674734e, 0x442369892d57601d, 0x68410c1a4a60ae59];
    let z = [0xf49b705d412e1fe3, 0x054eafeaabece976, 0x139f6206c981dac4, 0x95f3889047527dda];
    let r = [0x1abdd0f851b9205b, 0xb83c99755c11b10c, 0x67476ddbe816f4d6, 0xd0681f127ae7f48b];
    let s = [0x5c0ae59a9e64a6d6, 0x8472246f31d2c54e, 0x566306dd900cdfa5, 0x28ee30f2db8a71e0];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #28 z=0
    let pk = [0x9bbcc4236788e8ff, 0x12f6021e23d25acc, 0x9dcb940032ee8dc4, 0xeb78ff63d4016056, 0x8c996a48368b915e, 0xf6e6ab811f28a3a7, 0x2881d33e827c9a76, 0x5dc81d13073ddd67];
    let z = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r = [0x9a93a3bf185d8e23, 0x8df329e91e060e19, 0xa3522f1cd390056d, 0xf15eebaee3b672d8];
    let s = [0x319c29a474e80219, 0x0897a0bdac9aa7a9, 0x5c3174848defb856, 0x55150138c1a72fc2];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-s #28
    let pk = [0x9bbcc4236788e8ff, 0x12f6021e23d25acc, 0x9dcb940032ee8dc4, 0xeb78ff63d4016056, 0x8c996a48368b915e, 0xf6e6ab811f28a3a7, 0x2881d33e827c9a76, 0x5dc81d13073ddd67];
    let z = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r = [0x9a93a3bf185d8e23, 0x8df329e91e060e19, 0xa3522f1cd390056d, 0xf15eebaee3b672d8];
    let s = [0x319c29a474e8021a, 0x0897a0bdac9aa7a9, 0x5c3174848defb856, 0x55150138c1a72fc2];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #29
    let pk = [0x131138b26e5b6947, 0x472f2887577ffd5a, 0x42097ea3beb31fc7, 0x8a614a10ffd0ba3b, 0x16f29389260ccd4c, 0x528da3d882bbab66, 0x0ed9afd69b73077a, 0xd63e9a38ac5ced78];
    let z = [0x75934def31265242, 0xabf5eb98865d92f9, 0xfd7c8ac1b1b21a66, 0x8ac9667f7143526e];
    let r = [0xa45004970d322210, 0xa6a9e06beb733283, 0x6c690c5854aecdf7, 0x8c1fc92855f94774];
    let s = [0xd0c4ff366a7f2c18, 0xc48716ea7b1a3081, 0x05f7a87b0692df5d, 0x36b967a3cc76ba74];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #30
    let pk = [0xa2b6fcaa358d481e, 0x85397b0b72649b7b, 0x8ff03b3f015ecc64, 0x8fd4ac317d115527, 0xf40668614da59ee1, 0x2483f606ccf86bd4, 0x6bf0a8863718d1f2, 0x2c8b7aadbb9174dd];
    let z = [0x5d4538964a94ae4a, 0x357a6c2c42791aa6, 0x973907210f5b7104, 0x3f951a2dd7aa025d];
    let r = [0x978fca55fb9f0492, 0x0410f43ccdfc69fe, 0x3a93d37efa6d118d, 0x38399843e671325d];
    let s = [0x251dcc6c6b9704cb, 0x1e1aec474ad3c6e8, 0xeab04bef66574368, 0x244e7c8f7cbbe108];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-z #30
    let pk = [0xa2b6fcaa358d481e, 0x85397b0b72649b7b, 0x8ff03b3f015ecc64, 0x8fd4ac317d115527, 0xf40668614da59ee1, 0x2483f606ccf86bd4, 0x6bf0a8863718d1f2, 0x2c8b7aadbb9174dd];
    let z = [0x5d4538964a94ae4b, 0x357a6c2c42791aa6, 0x973907210f5b7104, 0x3f951a2dd7aa025d];
    let r = [0x978fca55fb9f0492, 0x0410f43ccdfc69fe, 0x3a93d37efa6d118d, 0x38399843e671325d];
    let s = [0x251dcc6c6b9704cb, 0x1e1aec474ad3c6e8, 0xeab04bef66574368, 0x244e7c8f7cbbe108];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #31
    let pk = [0x2423d2cd0904ba4d, 0xc6b0ff91684a93cb, 0xebb389f9618dbcb9, 0x314d424491b6ca47, 0xa89c648ad773e6b9, 0x0506908319c09553, 0x2c60c55a69f3bd03, 0x6b91f7bdd74f9fdc];
    let z = [0xfdbdb6eb76d6bd34, 0x639080650132224e, 0x929bb8441ddae8dd, 0xa733e1bc22c12efd];
    let r = [0x9e4e7584f77478cb, 0x70040c755c5c6d5e, 0x4f532585e0925e18, 0xb81077916c013e25];
    let s = [0xd160d3d5b2b2093a, 0x93aa866af938add0, 0x2332d00d9dec999f, 0x51a9684953e644df];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #32
    let pk = [0x883901e93b3016ce, 0x6bd0b900e7351f77, 0x86f2c231e9838622, 0xc03cf9d8fd9cf8a6, 0xcba97a5eeb3bfd2f, 0x8c15a40a2c0f37bd, 0x5183668d65980978, 0xc3716b6d61339659];
    let z = [0xe7d508f436066457, 0x0bc5f0c5c3233490, 0x6292bec73be58913, 0x3f0ce8888a38aaf5];
    let r = [0x58d68f4d418a2349, 0x2055af01d1c0ca0d, 0xb00c3539ed8e0bd0, 0x7d72c38e5f1c608b];
    let s = [0x565c9e94cd09e8b4, 0x465b9874ce58f9a1, 0x6c9f5e30667b1aa8, 0x03979859c8c5ca11];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-s #32
    let pk = [0x883901e93b3016ce, 0x6bd0b900e7351f77, 0x86f2c231e9838622, 0xc03cf9d8fd9cf8a6, 0xcba97a5eeb3bfd2f, 0x8c15a40a2c0f37bd, 0x5183668d65980978, 0xc3716b6d61339659];
    let z = [0xe7d508f436066457, 0x0bc5f0c5c3233490, 0x6292bec73be58913, 0x3f0ce8888a38aaf5];
    let r = [0x58d68f4d418a2349, 0x2055af01d1c0ca0d, 0xb00c3539ed8e0bd0, 0x7d72c38e5f1c608b];
    let s = [0x565c9e94cd09e8b5, 0x465b9874ce58f9a1, 0x6c9f5e30667b1aa8, 0x03979859c8c5ca11];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #33
    let pk = [0x89dab3368af24601, 0x939b69592518b7fc, 0xa8fb3532c59978ae, 0x685b37e7f8e94679, 0x2053b1cf5dc7faf4, 0xc8e4df836169d31b, 0x9219883218368deb, 0xf3be655e18feec71];
    let z = [0x9f221d264ebb4bf2, 0xea51ef80fb89227f, 0x023565e3144d683a, 0x64ba5a1053b39002];
    let r = [0xe653fe2a5040834b, 0xcc0222777e8ee205, 0x99b7aeff3d7495cf, 0x69e5ea3376269fdd];
    let s = [0x3f34e44883b68662, 0x0f034943b6402349, 0x1b75bebf0cf93777, 0x26c223f9f7e81521];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #34
    let pk = [0x6250e97dc6ecae03, 0x7c519c4fafcac2c9, 0xbb6b09c2fdef4c68, 0xe147378017805c86, 0xa92850a3ce6a2221, 0xc9e77a96f17500c3, 0x4b8d59795cae9388, 0x95603351a31c3926];
    let z = [0x5f166842a49031be, 0xa1c1fc8346fb0d85, 0x9337af29b892f0cc, 0x71020d968c06b9a4];
    let r = [0xf9a449cf941c3ada, 0x915ada68acefbf7f, 0x6a8d3bb1fc4c0591, 0x8d4f5362a49007bf];
    let s = [0x4f302f313f85802d, 0x879a60377adb2015, 0x17e8d3f317ccb868, 0x51f3a54d934880f1];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #35 z=0
    let pk = [0xca5d6f823d6c85de, 0xb1268fae2ee5549f, 0x766be200175fd50c, 0x2fe74bcaaf4194ad, 0x7ef102cfb7c26ccd, 0x2bee298d8c90ffa2, 0xe8a246adb518f9d4, 0x81e998fdbdc86227];
    let z = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r = [0x8be1f6a45ab1c1fb, 0x59bd59f9bd63aea8, 0xc7f953554ce4866b, 0x732dc5290b762cfc];
    let s = [0xc5fbcd7c42c644c0, 0x0dcc08d4135e3c6c, 0x338508ed10d26a1c, 0x06cda21593d24056];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-z #35
    let pk = [0xca5d6f823d6c85de, 0xb1268fae2ee5549f, 0x766be200175fd50c, 0x2fe74bcaaf4194ad, 0x7ef102cfb7c26ccd, 0x2bee298d8c90ffa2, 0xe8a246adb518f9d4, 0x81e998fdbdc86227];
    let z = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r = [0x8be1f6a45ab1c1fb, 0x59bd59f9bd63aea8, 0xc7f953554ce4866b, 0x732dc5290b762cfc];
    let s = [0xc5fbcd7c42c644c0, 0x0dcc08d4135e3c6c, 0x338508ed10d26a1c, 0x06cda21593d24056];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #36
    let pk = [0xd24ab578c5e57db9, 0x066e57089d65f4f3, 0x3a0517352e39cb3b, 0xe9fe0fbc71c0f954, 0xc28fbcd3baee69e6, 0xd0e1f4d7c075bb89, 0xeeb0e79850fd1bf7, 0x91079b86d5374abd];
    let z = [0xa7607320d6a1d4ab, 0x06c217095ce0c64e, 0x82750a29bde66443, 0xc1e0076f49006f52];
    let r = [0x08b2e4fd5f01da7e, 0xcac20d7947063f2e, 0xc2498a9a68797b3a, 0xd21df477ff2e9e2e];
    let s = [0xb3e0582b7f93363f, 0xdf07f491993046d8, 0x5e49e2afa0d74faf, 0x44cfc347482c04ce];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // tampered-s #36
    let pk = [0xd24ab578c5e57db9, 0x066e57089d65f4f3, 0x3a0517352e39cb3b, 0xe9fe0fbc71c0f954, 0xc28fbcd3baee69e6, 0xd0e1f4d7c075bb89, 0xeeb0e79850fd1bf7, 0x91079b86d5374abd];
    let z = [0xa7607320d6a1d4ab, 0x06c217095ce0c64e, 0x82750a29bde66443, 0xc1e0076f49006f52];
    let r = [0x08b2e4fd5f01da7e, 0xcac20d7947063f2e, 0xc2498a9a68797b3a, 0xd21df477ff2e9e2e];
    let s = [0xb3e0582b7f933640, 0xdf07f491993046d8, 0x5e49e2afa0d74faf, 0x44cfc347482c04ce];
    assert!(!ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #37
    let pk = [0xa599f5b503f6f117, 0x7f6cc73730f6585d, 0xcac9f3cf63763082, 0xa14e4154e3e7c243, 0x17f9ad5b1886eb23, 0xda53e09353663995, 0x64e5590c944744fb, 0xd18baec2a0c6f1e1];
    let z = [0x1cfcc7c4a649a13c, 0xa0996c68926f0ffb, 0x6d6afaaa4152695a, 0x16acad168da85d9d];
    let r = [0x0fef065af9438627, 0x5e695ddea42cbadf, 0x4163f1e2ad61e6d7, 0x4c978e78a09c8846];
    let s = [0x9fe7dcedad0162d0, 0xdf6f3f6d24c8b191, 0x55634e94aad60235, 0x2ed43e7b75eac39b];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #38
    let pk = [0x29abfb93b5429269, 0x7814673277f04347, 0x5d447c5b947fe94d, 0x13fd623f93d0cb47, 0x2adc704bf623d9b9, 0x839f2e90d95b9a1a, 0xa2c29b5462dd3bab, 0x0f8a378202e899d8];
    let z = [0x99aecccd1752a7be, 0x9f37409a9d8486dd, 0xcad359cc255bb2f3, 0x73d6460058ee83b5];
    let r = [0xf06d0151edcd1487, 0x71cdb278a11430a2, 0xa53631823c4554e5, 0x8c0be5e15b71470e];
    let s = [0x40f0f93612e89fb3, 0x7d65a617e32969ba, 0x897c8abf2c907e51, 0x6ecd8a273e0434b3];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
    // valid #39
    let pk = [0xf8011344fbdf70dd, 0xc4fb1520c9c47598, 0x5b80e1405fc4a803, 0xa07079eb41b81c8c, 0xea15175a62257238, 0x94f856178689a830, 0x4fbfc4e8eadbfa91, 0xe759a593758b3674];
    let z = [0x11a688ca00ff2184, 0x8c7b44c6e885b9a9, 0x849b6aa821839053, 0x847e2889e2aa38de];
    let r = [0x899fb75871e5c8cb, 0x3e7611c44b9eb57d, 0x2b5b2135c6168ad9, 0xc6401c8e8449048d];
    let s = [0x2349f4eac11aa35d, 0x7b17a2a92664e6ee, 0xd149f83be5a31989, 0x6ee47610f106e6f7];
    assert!(ecdsa_verify_secp256k1(&pk, &z, &r, &s));
}
