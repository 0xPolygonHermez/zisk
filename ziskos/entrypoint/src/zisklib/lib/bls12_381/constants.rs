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
    0xB9FE_FFFF_FFFF_AAAB,
    0x1EAB_FFFE_B153_FFFF,
    0x6730_D2A0_F6B0_F624,
    0x6477_4B84_F385_12BF,
    0x4B1B_A7B6_434B_ACD7,
    0x1A01_11EA_397F_E69A,
];

/// Base field size minus one
pub const P_MINUS_ONE: [u64; 6] = [P[0] - 1, P[1], P[2], P[3], P[4], P[5]];

/// Scalar field size
pub const R: [u64; 4] =
    [0xFFFF_FFFF_0000_0001, 0x53BD_A402_FFFE_5BFE, 0x3339_D808_09A1_D805, 0x73ED_A753_299D_7D48];

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

// ============================================================================
// Constants for G1 mapping (11-isogenous curve E': y² = x³ + A'x + B')
// ============================================================================

/// A' coefficient of the isogenous curve E' for G1
/// A' = 0x144698a3b8e9433d693a02c96d4982b0ea985383ee66a8d8e8981aefd881ac98936f8da0e0f97f5cf428082d584c1d
pub const ISO_A_G1: [u64; 6] = [
    0x5CF4_2808_2D58_4C1D,
    0x9893_6F8D_A0E0_F97F,
    0xD8E8_981A_EFD8_81AC,
    0xB0EA_9853_83EE_66A8,
    0x3D69_3A02_C96D_4982,
    0x0014_4698_A3B8_E943,
];

/// B' coefficient of the isogenous curve E' for G1
/// B' = 0x12e2908d11688030018b12e8753eee3b2016c1f0f24f4070a0b9c14fcef35ef55a23215a316ceaa5d1cc48e98e172be0
pub const ISO_B_G1: [u64; 6] = [
    0xD1CC_48E9_8E17_2BE0,
    0x5A23_215A_316C_EAA5,
    0xA0B9_C14F_CEF3_5EF5,
    0x2016_C1F0_F24F_4070,
    0x018B_12E8_753E_EE3B,
    0x12E2_908D_1168_8030,
];

/// Z constant for G1 SWU: Z = 11
pub const SWU_Z_G1: [u64; 6] = [0x0B, 0, 0, 0, 0, 0];
pub const SWU_Z2_G1: [u64; 6] = [0x79, 0, 0, 0, 0, 0]; // 0x0B^2

/// Cofactor for G1
pub const COFACTOR_G1: [u64; 4] = [0xD201000000010001, 0x0, 0x0, 0x0];

// ============================================================================
// G1 Isogeny Map Coefficients (11-isogeny from E' to E)
// ============================================================================

/// Isogeny map x-numerator coefficients for G1
pub const ISO_X_NUM_G1: [[u64; 6]; 12] = [
    [
        0xAEAC_1662_7346_49B7,
        0x5610_C2D5_F2E6_2D6E,
        0xF262_7B56_CDB4_E2C8,
        0x6B30_3E88_A2D7_005F,
        0xB809_101D_D998_1585,
        0x11A0_5F2B_1E83_3340,
    ],
    [
        0xE834_EEF1_B3CB_83BB,
        0x4838_F2A6_F318_C356,
        0xF565_E33C_70D1_E86B,
        0x7C17_E75B_2F6A_8417,
        0x0588_BAB2_2147_A81C,
        0x1729_4ED3_E943_AB2F,
    ],
    [
        0xE017_9F9D_AC9E_DCB0,
        0x958C_3E3D_2A09_729F,
        0x6878_E501_EC68_E25C,
        0xCE03_2473_2959_83E5,
        0x1D10_48C5_D10A_9A1B,
        0x0D54_005D_B976_78EC,
    ],
    [
        0xC5B3_8864_1D9B_6861,
        0x5336_E25C_E310_7193,
        0xF1B3_3289_F1B3_3083,
        0xD7F5_E465_6A8D_BF25,
        0x4E06_09D3_07E5_5412,
        0x1778_E716_6FCC_6DB7,
    ],
    [
        0x5115_4CE9_AC88_95D9,
        0x985A_286F_301E_77C4,
        0x086E_EB65_982F_AC18,
        0x99DB_995A_1257_FB3F,
        0x6642_B4B3_E411_8E54,
        0x0E99_726A_3199_F443,
    ],
    [
        0xCD13_C1C6_6F65_2983,
        0xA087_0D2D_CAE7_3D19,
        0x9ED3_AB90_97E6_8F90,
        0xDB3C_B17D_D952_799B,
        0x01D1_201B_F7A7_4AB5,
        0x1630_C325_0D73_13FF,
    ],
    [
        0xDDD7_F225_A139_ED84,
        0x8DA2_5128_C105_2ECA,
        0x9008_E218_F9C8_6B2A,
        0xB115_8626_4F0F_8CE1,
        0x6A37_26C3_8AE6_52BF,
        0x0D6E_D655_3FE4_4D29,
    ],
    [
        0x9CCB_5618_E3F0_C88E,
        0x39B7_C8F8_C8F4_75AF,
        0xA682_C62E_F0F2_7533,
        0x356D_E5AB_275B_4DB1,
        0xE874_3884_D111_7E53,
        0x17B8_1E77_01AB_DBE2,
    ],
    [
        0x6D71_986A_8497_E317,
        0x4FA2_95F2_96B7_4E95,
        0xA2C5_96C9_28C5_D1DE,
        0xC43B_756C_E79F_5574,
        0x7B90_B335_63BE_990D,
        0x080D_3CF1_F9A7_8FC4,
    ],
    [
        0x7F24_1067_BE39_0C9E,
        0xA319_0B2E_DC03_2779,
        0x6763_14BA_F4BB_1B7F,
        0xDD2E_CB80_3A0C_5C99,
        0x2E0C_3751_5D13_8F22,
        0x169B_1F8E_1BCF_A7C4,
    ],
    [
        0xCA67_DF3F_1605_FB7B,
        0xF69B_771F_8C28_5DEC,
        0xD50A_F360_03B1_4866,
        0xFA7D_CCDD_E678_7F96,
        0x72D8_EC09_D256_5B0D,
        0x1032_1DA0_79CE_07E2,
    ],
    [
        0xA9C8_BA2E_8BA2_D229,
        0xC24B_1B80_B64D_391F,
        0x23C0_BF1B_C24C_6B68,
        0x31D7_9D7E_22C8_37BC,
        0xBD1E_9623_81ED_EE3D,
        0x06E0_8C24_8E26_0E70,
    ],
];

/// Isogeny map x-denominator coefficients for G1
pub const ISO_X_DEN_G1: [[u64; 6]; 11] = [
    [
        0x993C_F9FA_40D2_1B1C,
        0xB558_D681_BE34_3DF8,
        0x9C95_8861_7FC8_AC62,
        0x01D5_EF4B_A35B_48BA,
        0x18B2_E62F_4BD3_FA6F,
        0x08CA_8D54_8CFF_19AE,
    ],
    [
        0xE5C8_276E_C82B_3BFF,
        0x13DA_A884_6CB0_26E9,
        0x0126_C258_8C48_BF57,
        0x7041_E8CA_0CF0_800C,
        0x48B4_7112_98E5_3636,
        0x1256_1A5D_EB55_9C43,
    ],
    [
        0xFCC2_39BA_5CB8_3E19,
        0xD6A3_D096_7C94_FEDC,
        0xFCA6_4E00_B11A_CEAC,
        0x6F89_416F_5A71_8CD1,
        0x8137_E629_BFF2_991F,
        0x0B29_62FE_57A3_225E,
    ],
    [
        0x130D_E893_8DC6_2CD8,
        0x4976_D524_3EEC_F5C4,
        0x54CC_A8AB_C28D_6FD0,
        0x5B08_243F_16B1_6551,
        0xC83A_AFEF_7C40_EB54,
        0x0342_5581_A58A_E2FE,
    ],
    [
        0x539D_395B_3532_A21E,
        0x9BD2_9BA8_1F35_781D,
        0x8D6B_44E8_33B3_06DA,
        0xFFDF_C759_A120_62BB,
        0x0A6F_1D5F_43E7_A07D,
        0x13A8_E162_0229_14A8,
    ],
    [
        0xC02D_F9A2_9F63_04A5,
        0x7400_D24B_C422_8F11,
        0x0A43_BCEF_24B8_982F,
        0x3957_35E9_CE9C_AD4D,
        0x5539_0F7F_0506_C6E9,
        0x0E73_55F8_E4E6_67B9,
    ],
    [
        0xEC25_7449_6EE8_4A3A,
        0xEA73_B353_8F0D_E06C,
        0x4E2E_0730_62AE_DE9C,
        0x570F_5799_AF53_A189,
        0x0F3E_0C63_E059_6721,
        0x0772_CAAC_F169_3619,
    ],
    [
        0x11F7_D99B_BDCC_5A5E,
        0x0FA5_B948_9D11_E2D3,
        0x1996_E1CD_F982_2C58,
        0x6E7F_63C2_1BCA_68A8,
        0x30B3_F5B0_74CF_0199,
        0x14A7_AC2A_9D64_A8B2,
    ],
    [
        0x4776_EC3A_79A1_D641,
        0x0382_6692_ABBA_4370,
        0x7410_0DA6_7F39_8835,
        0xE07F_8D1D_7161_366B,
        0x5E92_0B3D_AFC7_A3CC,
        0x0A10_ECF6_ADA5_4F82,
    ],
    [
        0x2D63_84D1_68EC_DD0A,
        0x9317_4E4B_4B78_6500,
        0x76DF_5339_78F3_1C15,
        0xF682_B4EE_96F7_D037,
        0x476D_6E3E_B3A5_6680,
        0x095F_C13A_B9E9_2AD4,
    ],
    [0x1, 0x0, 0x0, 0x0, 0x0, 0x0],
];

/// Isogeny map y-numerator coefficients for G1
pub const ISO_Y_NUM_G1: [[u64; 6]; 16] = [
    [
        0xBE98_4571_9707_BB33,
        0xCD0C_7AEE_9B3B_A3C2,
        0x2B52_AF6C_9565_43D3,
        0x11AD_138E_48A8_6952,
        0x259D_1F09_4980_DCFA,
        0x090D_97C8_1BA2_4EE0,
    ],
    [
        0xE097_E75A_2E41_C696,
        0xD6C5_6711_962F_A8BF,
        0x0F90_6343_EB67_AD34,
        0x1223_E96C_254F_383D,
        0xD510_36D7_76FB_4683,
        0x1349_96A1_04EE_5811,
    ],
    [
        0xB8DF_E240_C72D_E1F6,
        0xD26D_5216_28B0_0523,
        0xC344_BE4B_9140_0DA7,
        0x2552_E2D6_58A3_1CE2,
        0xF4A3_84C8_6A3B_4994,
        0x00CC_786B_AA96_6E66,
    ],
    [
        0xA635_5C77_B0E5_F4CB,
        0xDE40_5ABA_9EC6_1DEC,
        0x09E4_A3EC_0325_1CF9,
        0xD42A_A7B9_0EEB_791C,
        0x7898_751A_D874_6757,
        0x01F8_6376_E898_1C21,
    ],
    [
        0x41B6_DAEC_F2E8_FEDB,
        0x2EE7_F8DC_0990_40A8,
        0x7983_3FD2_2135_1ADC,
        0x1955_36FB_E3CE_50B8,
        0x5CAF_4FE2_A215_29C4,
        0x08CC_03FD_EFE0_FF13,
    ],
    [
        0x99B2_3AB1_3633_A5F0,
        0x203F_6326_C95A_8072,
        0x7650_5C3D_3AD5_544E,
        0x74A7_D0D4_AFAD_B7BD,
        0x2211_E11D_B8F0_A6A0,
        0x1660_3FCA_4063_4B6A,
    ],
    [
        0xC961_F885_5FE9_D6F2,
        0x47A8_7AC2_460F_415E,
        0x5231_413C_4D63_4F37,
        0xE75B_B8CA_2BE1_84CB,
        0xB2C9_77D0_2779_6B3C,
        0x04AB_0B9B_CFAC_1BBC,
    ],
    [
        0xA15E_4CA3_1870_FB29,
        0x42F6_4550_FEDF_E935,
        0xFD03_8DA6_C26C_8426,
        0x170A_05BF_E3BD_D81F,
        0xDE99_26BD_2CA6_C674,
        0x0987_C8D5_333A_B86F,
    ],
    [
        0x6037_0E57_7BDB_A587,
        0x69D6_5201_C786_07A3,
        0x1E8B_6E6A_1F20_CABE,
        0x8F3A_BD16_679D_C26C,
        0xE88C_9E22_1E4D_A1BB,
        0x09FC_4018_BD96_684B,
    ],
    [
        0x2BAF_AAEB_CA73_1C30,
        0x9B3F_7055_DD4E_BA6F,
        0x0698_5E7E_D1E4_D43B,
        0xC42A_0CA7_915A_F6FE,
        0x223A_BDE7_ADA1_4A23,
        0x0E1B_BA7A_1186_BDB5,
    ],
    [
        0xE813_711A_D011_C132,
        0x31BF_3A5C_CE3F_BAFC,
        0xD118_3E41_6389_E610,
        0xCD2F_CBCB_6CAF_493F,
        0x0DFD_0B8F_1D43_FB93,
        0x1971_3E47_937C_D1BE,
    ],
    [
        0xCE07_C8A4_D007_4D8E,
        0x49D9_CDF4_1B44_D606,
        0x2E6B_FE7F_911F_6432,
        0x5235_59B8_AAF0_C246,
        0xB918_C143_FED2_EDCC,
        0x18B4_6A90_8F36_F6DE,
    ],
    [
        0x0D4C_04F0_0B97_1EF8,
        0x06C8_51C1_9192_11F2,
        0xC027_10E8_07B4_633F,
        0x7AA7_B12A_3426_B08E,
        0xD155_0960_04F5_3F44,
        0x0B18_2CAC_101B_9399,
    ],
    [
        0x42D9_D3F5_DB98_0133,
        0xC6CF_90AD_1C23_2A64,
        0x13E6_632D_3C40_659C,
        0x757B_3B08_0D4C_1580,
        0x72FC_00AE_7BE3_15DC,
        0x0245_A394_AD1E_CA9B,
    ],
    [
        0x866B_1E71_5475_224B,
        0x6BA1_049B_6579_AFB7,
        0xD9AB_0F5D_396A_7CE4,
        0x5E67_3D81_D7E8_6568,
        0x02A1_59F7_48C4_A3FC,
        0x05C1_2964_5E44_CF11,
    ],
    [
        0x04B4_56BE_69C8_B604,
        0xB665_027E_FEC0_1C77,
        0x57AD_D4FA_95AF_01B2,
        0xCB18_1D8F_8496_5A39,
        0x4EA5_0B3B_42DF_2EB5,
        0x15E6_BE4E_990F_03CE,
    ],
];

/// Isogeny map y-denominator coefficients for G1
pub const ISO_Y_DEN_G1: [[u64; 6]; 16] = [
    [
        0x0147_9253_B036_63C1,
        0x07F3_688E_F60C_206D,
        0xEEC3_232B_5BE7_2E7A,
        0x601A_6DE5_7898_0BE6,
        0x5218_1140_FAD0_EAE9,
        0x1611_2C4C_3A9C_98B2,
    ],
    [
        0x32F6_102C_2E49_A03D,
        0x78A4_2607_6352_9E35,
        0xA4A1_0356_F453_E01F,
        0x85C8_4FF7_31C4_D59C,
        0x1A0C_BD6C_43C3_48B8,
        0x1962_D75C_2381_201E,
    ],
    [
        0x1E25_38B5_3DBF_67F2,
        0xA675_7CD6_36F9_6F89,
        0x0C35_A5DD_279C_D2EC,
        0x78C4_8555_51AE_7F31,
        0x6FAA_AE7D_6E8E_B157,
        0x058D_F330_6640_DA27,
    ],
    [
        0xA8D2_6D98_445F_5416,
        0x7273_64F2_C282_97AD,
        0x123D_A489_E726_AF41,
        0xD115_C5DB_DDBC_D30E,
        0xF20D_23BF_89ED_B4D1,
        0x16B7_D288_798E_5395,
    ],
    [
        0xDA39_1423_11A5_001D,
        0xA20B_15DC_0FD2_EDED,
        0x542E_DA0F_C9DE_C916,
        0xC6D1_9C9F_0F69_BBB0,
        0xB00C_C912_F822_8DDC,
        0x0BE0_E079_545F_43E4,
    ],
    [
        0x02C6_477F_AAF9_B7AC,
        0x49F3_8DB9_DFA9_CCE2,
        0xC5EC_D87B_6F0F_5A64,
        0xB701_52C6_5550_D881,
        0x9FB2_66EA_AC78_3182,
        0x08D9_E529_7186_DB2D,
    ],
    [
        0x3D1A_1399_126A_775C,
        0xD5FA_9C01_A58B_1FB9,
        0x5DD3_65BC_400A_0051,
        0x5EEC_FDFA_8D0C_F8EF,
        0xC3BA_8734_ACE9_824B,
        0x1660_07C0_8A99_DB2F,
    ],
    [
        0x60EE_415A_1581_2ED9,
        0xB920_F5B0_0801_DEE4,
        0xFEB3_4FD2_0635_7132,
        0xE5A4_375E_FA1F_4FD7,
        0x03BC_DDFA_BBA6_FF6E,
        0x16A3_EF08_BE3E_A7EA,
    ],
    [
        0x6B23_3D9D_5553_5D4A,
        0x52CF_E2F7_BB92_4883,
        0xABC5_750C_4BF3_9B48,
        0xF9FB_0CE4_C6AF_5920,
        0x1A1B_E54F_D1D7_4CC4,
        0x1866_C8ED_336C_6123,
    ],
    [
        0x346E_F48B_B891_3F55,
        0xC738_5EA3_D529_B35E,
        0x5308_592E_7EA7_D4FB,
        0x3216_F763_E13D_87BB,
        0xEA82_0597_D94A_8490,
        0x167A_55CD_A70A_6E1C,
    ],
    [
        0x00F8_B49C_BA8F_6AA8,
        0x71A5_C29F_4F83_0604,
        0x0E59_1B36_E636_A5C8,
        0x9C6D_D039_BB61_A629,
        0x48F0_10A0_1AD2_911D,
        0x04D2_F259_EEA4_05BD,
    ],
    [
        0x9684_B529_E256_1092,
        0x16F9_6898_6F7E_BBEA,
        0x8C0F_9A88_CEA7_9135,
        0x7F94_FF8A_EFCE_42D2,
        0xF585_2C1E_48C5_0C47,
        0x0ACC_BB67_481D_033F,
    ],
    [
        0x1E99_B138_5733_45CC,
        0x9300_0763_E3B9_0AC1,
        0x7D5C_EEF9_A00D_9B86,
        0x5433_46D9_8ADF_0226,
        0xC361_3144_B45F_1496,
        0x0AD6_B951_4C76_7FE3,
    ],
    [
        0xD1FA_DC13_26ED_06F7,
        0x4205_17BD_8714_CC80,
        0xCB74_8DF2_7942_480E,
        0xBF56_5B94_E729_27C1,
        0x628B_DD0D_53CD_76F2,
        0x0266_0400_EB2E_4F3B,
    ],
    [
        0x4415_473A_1D63_4B8F,
        0x5CA2_F570_F134_9780,
        0x324E_FCD6_356C_AA20,
        0x71C4_0F65_E273_B853,
        0x6B24_255E_0D78_19C1,
        0x0E0F_A1D8_16DD_C03E,
    ],
    [0x1, 0x0, 0x0, 0x0, 0x0, 0x0],
];

// ============================================================================
// Constants for G2 mapping (3-isogenous curve E': y² = x³ + A'x + B')
// ============================================================================

/// A' coefficient of the isogenous curve E' for G2
/// A' = 0xF0 * I
pub const ISO_A_G2: [u64; 12] =
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00];

/// B' coefficient of the isogenous curve E' for G2
/// B' = 0x03F4 * (1 + I)
pub const ISO_B_G2: [u64; 12] = [
    0x03F4, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x03F4, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
];

/// Z constant for G2 SWU: Z = -(2 + I)
pub const SWU_Z_G2: [u64; 12] =
    [P[0] - 2, P[1], P[2], P[3], P[4], P[5], P[0] - 1, P[1], P[2], P[3], P[4], P[5]];

// ============================================================================
// G2 Isogeny Map Coefficients (3-isogeny from E' to E)
// ============================================================================

/// Isogeny map x-numerator coefficients for G2
pub const ISO_X_NUM_G2: [[u64; 12]; 4] = [
    [
        0x6238_AAAA_AAAA_97D6,
        0x5C26_38E3_43D9_C71C,
        0x88B5_8423_C50A_E15D,
        0x32C5_2D39_FD3A_042A,
        0xBB5B_7A9A_47D7_ED85,
        0x05C7_5950_7E8E_333E,
        0x6238_AAAA_AAAA_97D6,
        0x5C26_38E3_43D9_C71C,
        0x88B5_8423_C50A_E15D,
        0x32C5_2D39_FD3A_042A,
        0xBB5B_7A9A_47D7_ED85,
        0x05C7_5950_7E8E_333E,
    ],
    [
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x26A9_FFFF_FFFF_C71A,
        0x1472_AAA9_CB8D_5555,
        0x9A20_8C6B_4F20_A418,
        0x984F_87AD_F7AE_0C7F,
        0x3212_6FCE_D787_C88F,
        0x1156_0BF1_7BAA_99BC,
    ],
    [
        0x26A9_FFFF_FFFF_C71E,
        0x1472_AAA9_CB8D_5555,
        0x9A20_8C6B_4F20_A418,
        0x984F_87AD_F7AE_0C7F,
        0x3212_6FCE_D787_C88F,
        0x1156_0BF1_7BAA_99BC,
        0x9354_FFFF_FFFF_E38D,
        0x0A39_5554_E5C6_AAAA,
        0xCD10_4635_A790_520C,
        0xCC27_C3D6_FBD7_063F,
        0x1909_37E7_6BC3_E447,
        0x08AB_05F8_BDD5_4CDE,
    ],
    [
        0x88E2_AAAA_AAAA_5ED1,
        0x7098_E38D_0F67_1C71,
        0x22D6_108F_142B_8575,
        0xCB14_B4E7_F4E8_10AA,
        0xED6D_EA69_1F5F_B614,
        0x171D_6541_FA38_CCFA,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
    ],
];

/// Isogeny map x-denominator coefficients for G2
pub const ISO_X_DEN_G2: [[u64; 12]; 3] = [
    [
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0xB9FE_FFFF_FFFF_AA63,
        0x1EAB_FFFE_B153_FFFF,
        0x6730_D2A0_F6B0_F624,
        0x6477_4B84_F385_12BF,
        0x4B1B_A7B6_434B_ACD7,
        0x1A01_11EA_397F_E69A,
    ],
    [
        0x0000_0000_0000_000C,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0xB9FE_FFFF_FFFF_AA9F,
        0x1EAB_FFFE_B153_FFFF,
        0x6730_D2A0_F6B0_F624,
        0x6477_4B84_F385_12BF,
        0x4B1B_A7B6_434B_ACD7,
        0x1A01_11EA_397F_E69A,
    ],
    [0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0],
];

/// Isogeny map y-numerator coefficients for G2
pub const ISO_Y_NUM_G2: [[u64; 12]; 4] = [
    [
        0x12CF_C71C_71C6_D706,
        0xFC8C_25EB_F8C9_2F68,
        0xF544_39D8_7D27_E500,
        0x0F7D_A5D4_A07F_649B,
        0x59A4_C18B_076D_1193,
        0x1530_477C_7AB4_113B,
        0x12CF_C71C_71C6_D706,
        0xFC8C_25EB_F8C9_2F68,
        0xF544_39D8_7D27_E500,
        0x0F7D_A5D4_A07F_649B,
        0x59A4_C18B_076D_1193,
        0x1530_477C_7AB4_113B,
    ],
    [
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x6238_AAAA_AAAA_97BE,
        0x5C26_38E3_43D9_C71C,
        0x88B5_8423_C50A_E15D,
        0x32C5_2D39_FD3A_042A,
        0xBB5B_7A9A_47D7_ED85,
        0x05C7_5950_7E8E_333E,
    ],
    [
        0x26A9_FFFF_FFFF_C71C,
        0x1472_AAA9_CB8D_5555,
        0x9A20_8C6B_4F20_A418,
        0x984F_87AD_F7AE_0C7F,
        0x3212_6FCE_D787_C88F,
        0x1156_0BF1_7BAA_99BC,
        0x9354_FFFF_FFFF_E38F,
        0x0A39_5554_E5C6_AAAA,
        0xCD10_4635_A790_520C,
        0xCC27_C3D6_FBD7_063F,
        0x1909_37E7_6BC3_E447,
        0x08AB_05F8_BDD5_4CDE,
    ],
    [
        0xE1B3_71C7_1C71_8B10,
        0x4E79_097A_56DC_4BD9,
        0xB0E9_77C6_9AA2_7452,
        0x761B_0F37_A1E2_6286,
        0xFBF7_043D_E381_1AD0,
        0x124C_9AD4_3B6C_F79B,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
    ],
];

/// Isogeny map y-denominator coefficients for G2
pub const ISO_Y_DEN_G2: [[u64; 12]; 4] = [
    [
        0xB9FE_FFFF_FFFF_A8FB,
        0x1EAB_FFFE_B153_FFFF,
        0x6730_D2A0_F6B0_F624,
        0x6477_4B84_F385_12BF,
        0x4B1B_A7B6_434B_ACD7,
        0x1A01_11EA_397F_E69A,
        0xB9FE_FFFF_FFFF_A8FB,
        0x1EAB_FFFE_B153_FFFF,
        0x6730_D2A0_F6B0_F624,
        0x6477_4B84_F385_12BF,
        0x4B1B_A7B6_434B_ACD7,
        0x1A01_11EA_397F_E69A,
    ],
    [
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0xB9FE_FFFF_FFFF_A9D3,
        0x1EAB_FFFE_B153_FFFF,
        0x6730_D2A0_F6B0_F624,
        0x6477_4B84_F385_12BF,
        0x4B1B_A7B6_434B_ACD7,
        0x1A01_11EA_397F_E69A,
    ],
    [
        0x0000_0000_0000_0012,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        0xB9FE_FFFF_FFFF_AA99,
        0x1EAB_FFFE_B153_FFFF,
        0x6730_D2A0_F6B0_F624,
        0x6477_4B84_F385_12BF,
        0x4B1B_A7B6_434B_ACD7,
        0x1A01_11EA_397F_E69A,
    ],
    [0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0],
];

pub const PSI_C1: [u64; 12] = [
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x8BFD_0000_0000_AAAD,
    0x4094_27EB_4F49_FFFD,
    0x897D_2965_0FB8_5F9B,
    0xAA0D_857D_8975_9AD4,
    0xEC02_4086_63D4_DE85,
    0x1A01_11EA_397F_E699,
];

pub const PSI_C2: [u64; 12] = [
    0xF1EE_7B04_121B_DEA2,
    0x3044_66CF_3E67_FA0A,
    0xEF39_6489_F61E_B45E,
    0x1C3D_EDD9_30B1_CF60,
    0xE2E9_C448_D77A_2CD9,
    0x1352_03E6_0180_A68E,
    0xC810_84FB_EDE3_CC09,
    0xEE67_992F_72EC_05F4,
    0x77F7_6E17_0092_41C5,
    0x4839_5DAB_C2D3_435E,
    0x6831_E36D_6BD1_7FFE,
    0x06AF_0E04_37FF_400B,
];

pub const PSI2_C1: [u64; 12] = [
    0x8BFD_0000_0000_AAAC,
    0x4094_27EB_4F49_FFFD,
    0x897D_2965_0FB8_5F9B,
    0xAA0D_857D_8975_9AD4,
    0xEC02_4086_63D4_DE85,
    0x1A01_11EA_397F_E699,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0000,
];
