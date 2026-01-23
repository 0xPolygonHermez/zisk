//! Constants for the BLS12-381 elliptic curve

/// Family parameter X = -0xd201000000010000
pub const X_ABS_BIN_BE: [u8; 64] = [
    1, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// B parameter of the curve E: y² = x³ + 4
pub const E_B: [u64; 6] = [0x4, 0, 0, 0, 0, 0];

/// B parameter of the twist E': y² = x³ + 4·(1+u)
pub const ETWISTED_B: [u64; 12] = [0x4, 0x0, 0x0, 0x0, 0x0, 0x0, 0x4, 0x0, 0x0, 0x0, 0x0, 0x0];

/// Identity element in G1
pub const G1_IDENTITY: [u64; 12] = [0; 12];

/// Identity element in G2
pub const G2_IDENTITY: [u64; 24] = [0; 24];

/// G1 generator point for BLS12-381
pub const G1_GENERATOR: [u64; 12] = [
    0xFB3A_F00A_DB22_C6BB,
    0x6C55_E83F_F97A_1AEF,
    0xA14E_3A3F_171B_AC58,
    0xC368_8C4F_9774_B905,
    0x2695_638C_4FA9_AC0F,
    0x17F1_D3A7_3197_D794,
    0x0CAA_2329_46C5_E7E1,
    0xD03C_C744_A288_8AE4,
    0x00DB_18CB_2C04_B3ED,
    0xFCF5_E095_D5D0_0AF6,
    0xA09E_30ED_741D_8AE4,
    0x08B3_F481_E3AA_A0F1,
];

/// G2 generator point for BLS12-381
pub const G2_GENERATOR: [u64; 24] = [
    0xD480_56C8_C121_BDB8,
    0x0BAC_0326_A805_BBEF,
    0xB451_0B64_7AE3_D177,
    0xC6E4_7AD4_FA40_3B02,
    0x2608_0527_2DC5_1051,
    0x024A_A2B2_F08F_0A91,
    0xE5AC_7D05_5D04_2B7E,
    0x334C_F112_1394_5D57,
    0xB5DA_61BB_DC7F_5049,
    0x596B_D0D0_9920_B61A,
    0x7DAC_D3A0_8827_4F65,
    0x13E0_2B60_5271_9F60,
    0xE193_5486_08B8_2801,
    0x923A_C9CC_3BAC_A289,
    0x6D42_9A69_5160_D12C,
    0xADFD_9BAA_8CBD_D3A7,
    0x8CC9_CDC6_DA2E_351A,
    0x0CE5_D527_727D_6E11,
    0xAAA9_075F_F05F_79BE,
    0x3F37_0D27_5CEC_1DA1,
    0x2674_92AB_572E_99AB,
    0xCB3E_287E_85A7_63AF,
    0x32AC_D2B0_2BC2_8B99,
    0x0606_C4A0_2EA7_34CC,
];

/// Base field size
pub const P: [u64; 6] = [
    0xB9FEFFFFFFFFAAAB,
    0x1EABFFFEB153FFFF,
    0x6730D2A0F6B0F624,
    0x64774B84F38512BF,
    0x4B1BA7B6434BACD7,
    0x1A0111EA397FE69A,
];

/// Base field size minus one
pub const P_MINUS_ONE: [u64; 6] = [P[0] - 1, P[1], P[2], P[3], P[4], P[5]];

/// Scalar field size
pub const R: [u64; 4] =
    [0xFFFFFFFF00000001, 0x53BDA402FFFE5BFE, 0x3339D80809A1D805, 0x73EDA753299D7D48];

/// Scalar field size minus one
pub const R_MINUS_ONE: [u64; 4] = [R[0] - 1, R[1], R[2], R[3]];

/// A known non-quadratic residue in Fp
pub const NQR_FP: [u64; 6] = [2, 0, 0, 0, 0, 0];

/// A known non-quadratic residue in Fp2
pub const NQR_FP2: [u64; 12] = [1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0];

/// This is the the order-3 element of for the σ endomorphism
pub const GAMMA: [u64; 6] = [
    0x8BFD_0000_0000_AAAC,
    0x4094_27EB_4F49_FFFD,
    0x897D_2965_0FB8_5F9B,
    0xAA0D_857D_8975_9AD4,
    0xEC02_4086_63D4_DE85,
    0x1A01_11EA_397F_E699,
];

/// Representation of 1 + u, where u is s.t. u² = - 1
pub const EXT_U: [u64; 12] = [0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0];

/// Representation of (1 + u)⁻¹
pub const EXT_U_INV: [u64; 12] = [
    0xDCFF_7FFF_FFFF_D556,
    0x0F55_FFFF_58A9_FFFF,
    0xB398_6950_7B58_7B12,
    0xB23B_A5C2_79C2_895F,
    0x258D_D3DB_21A5_D66B,
    0x0D00_88F5_1CBF_F34D,
    0xDCFF_7FFF_FFFF_D555,
    0x0F55_FFFF_58A9_FFFF,
    0xB398_6950_7B58_7B12,
    0xB23B_A5C2_79C2_895F,
    0x258D_D3DB_21A5_D66B,
    0x0D00_88F5_1CBF_F34D,
];

/// Representation of v, where v is s.t. w² = v
pub const EXT_V: [u64; 12] = [0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];

// Precomputed frobenius operator constants. These are used to compute f^p and f^p2 efficiently, where f ∈ Fp12
// See: https://hackmd.io/4gg0L936QsGdvSwZOjonlQ?view#Frobenius-Operator

/// Frobenius operator constant 𝛾₁₁ := (1 + u)^((p-1)/6)
pub const FROBENIUS_GAMMA11: [u64; 12] = [
    0x8D07_75ED_9223_5FB8,
    0xF67E_A53D_63E7_813D,
    0x7B24_43D7_84BA_B9C4,
    0x0FD6_03FD_3CBD_5F4F,
    0xC231_BEB4_202C_0D1F,
    0x1904_D3BF_02BB_0667,
    0x2CF7_8A12_6DDC_4AF3,
    0x282D_5AC1_4D6C_7EC2,
    0xEC0C_8EC9_71F6_3C5F,
    0x54A1_4787_B6C7_B36F,
    0x88E9_E902_231F_9FB8,
    0x00FC_3E2B_36C4_E032,
];

/// Frobenius operator constant 𝛾₁₂ := (1 + u)^(2·(p-1)/6)
pub const FROBENIUS_GAMMA12: [u64; 12] = [
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x8BFD_0000_0000_AAAC,
    0x4094_27EB_4F49_FFFD,
    0x897D_2965_0FB8_5F9B,
    0xAA0D_857D_8975_9AD4,
    0xEC02_4086_63D4_DE85,
    0x1A01_11EA_397F_E699,
];

/// Frobenius operator constant 𝛾₁₃ := (1 + u)^(3·(p-1)/6)
pub const FROBENIUS_GAMMA13: [u64; 12] = [
    0xC810_84FB_EDE3_CC09,
    0xEE67_992F_72EC_05F4,
    0x77F7_6E17_0092_41C5,
    0x4839_5DAB_C2D3_435E,
    0x6831_E36D_6BD1_7FFE,
    0x06AF_0E04_37FF_400B,
    0xC810_84FB_EDE3_CC09,
    0xEE67_992F_72EC_05F4,
    0x77F7_6E17_0092_41C5,
    0x4839_5DAB_C2D3_435E,
    0x6831_E36D_6BD1_7FFE,
    0x06AF_0E04_37FF_400B,
];

/// Frobenius operator constant 𝛾₁₄ := (1 + u)^(4·(p-1)/6)
pub const FROBENIUS_GAMMA14: [u64; 6] = [
    0x8BFD_0000_0000_AAAD,
    0x4094_27EB_4F49_FFFD,
    0x897D_2965_0FB8_5F9B,
    0xAA0D_857D_8975_9AD4,
    0xEC02_4086_63D4_DE85,
    0x1A01_11EA_397F_E699,
];

/// Frobenius operator constant 𝛾₁₅ := (1 + u)^(5·(p-1)/6)
pub const FROBENIUS_GAMMA15: [u64; 12] = [
    0x9B18_FAE9_8007_8116,
    0xC63A_3E6E_257F_8732,
    0x8BEA_DF4D_8E9C_0566,
    0xF398_1624_0C0B_8FEE,
    0xDF47_FA6B_48B1_E045,
    0x05B2_CFD9_013A_5FD8,
    0x1EE6_0516_7FF8_2995,
    0x5871_C190_8BD4_78CD,
    0xDB45_F353_6814_F0BD,
    0x70DF_3560_E779_82D0,
    0x6BD3_AD4A_FA99_CC91,
    0x144E_4211_3845_86C1,
];

/// Frobenius operator constant 𝛾₂₁ := (1 + u)^((p²-1)/6)
pub const FROBENIUS_GAMMA21: [u64; 6] = [
    0x2E01_FFFF_FFFE_FFFF,
    0xDE17_D813_620A_0002,
    0xDDB3_A93B_E6F8_9688,
    0xBA69_C607_6A0F_77EA,
    0x5F19_672F_DF76_CE51,
    0x0000_0000_0000_0000,
];

/// Frobenius operator constant 𝛾₂₂ := (1 + u)^(2·(p²-1)/6)
pub const FROBENIUS_GAMMA22: [u64; 6] = [
    0x2E01_FFFF_FFFE_FFFE,
    0xDE17_D813_620A_0002,
    0xDDB3_A93B_E6F8_9688,
    0xBA69_C607_6A0F_77EA,
    0x5F19_672F_DF76_CE51,
    0x0000_0000_0000_0000,
];

/// Frobenius operator constant 𝛾₂₃ := (1 + u)^(3·(p²-1)/6)
pub const FROBENIUS_GAMMA23: [u64; 6] = [
    0xB9FE_FFFF_FFFF_AAAA,
    0x1EAB_FFFE_B153_FFFF,
    0x6730_D2A0_F6B0_F624,
    0x6477_4B84_F385_12BF,
    0x4B1B_A7B6_434B_ACD7,
    0x1A01_11EA_397F_E69A,
];

/// Frobenius operator constant 𝛾₂₄ := (1 + u)^(4·(p²-1)/6)
pub const FROBENIUS_GAMMA24: [u64; 6] = [
    0x8BFD_0000_0000_AAAC,
    0x4094_27EB_4F49_FFFD,
    0x897D_2965_0FB8_5F9B,
    0xAA0D_857D_8975_9AD4,
    0xEC02_4086_63D4_DE85,
    0x1A01_11EA_397F_E699,
];

/// Frobenius operator constant 𝛾₂₅ := (1 + u)^(5·(p²-1)/6)
pub const FROBENIUS_GAMMA25: [u64; 6] = [
    0x8BFD_0000_0000_AAAD,
    0x4094_27EB_4F49_FFFD,
    0x897D_2965_0FB8_5F9B,
    0xAA0D_857D_8975_9AD4,
    0xEC02_4086_63D4_DE85,
    0x1A01_11EA_397F_E699,
];

/// Trusted setup G2 point `[τ]₂ := τ·G2` from the Ethereum KZG ceremony (uncompressed format)
/// For reference, see: https://github.com/ethereum/kzg-ceremony
pub const TRUSTED_SETUP_TAU_G2: [u64; 24] = [
    0xc98edada20c1def2,
    0x087041de621000ed,
    0xa36851477ba4c60b,
    0x3926c911cceceac9,
    0x734429b7b38608e2,
    0x185cbfee53492714,
    0xafaaab24f3499f72,
    0x2914e5870cb452d2,
    0x1009a2ce615ac53d,
    0x26187075cbfbefa8,
    0x843bc287230af389,
    0x15bfd7dd8cdeb128,
    0xee689bfbbb832a99,
    0x4ce26d105941f383,
    0xe82451a496a9c979,
    0x131569490e28de18,
    0xd7d5ee8599d1fca2,
    0x014353bdb96b626d,
    0x23048ef30d0a154f,
    0x9495346f3d7ac9cd,
    0xda5ed1ba9bfa0789,
    0xef79de09fc63671f,
    0x03432fcae0181b4b,
    0x1666c54b0a325295,
];
