// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.7.0 <0.9.0;

/// @title PilfflonkVerifier
/// @notice Verifies the pilfflonk proofs of the vkey with digest 0xe9eb2c78fc2cff7c38c459927e3afd00189a06af869a597c8109240ab0aedd3a: its
///         pilfflonk.vkey.json, from which `proofman-setup setup-pilfflonk --solidity` or
///         `proofman-setup pilfflonk-solidity` generated it.
/// @dev    It accepts exactly the proofs the JS verifier (pilfflonk/js, verify.js) accepts, and its
///         steps are the JS's. Calldata: `proof` is the proof's bytes (the commitments of the
///         non-fixed f_i, W, W', the evaluations, the Q_i(xi) if Q is split, inv and invZh) as
///         211 big-endian words; `pubSignals`, the publics.
contract PilfflonkVerifier {
    uint32 constant n = 1048576; // Domain size, N = 2^20

    // Scalar field size
    uint256 constant q    = 21888242871839275222246405745257275088548364400416034343698204186575808495617;
    // Base field size
    uint256 constant qf   = 21888242871839275222246405745257275088696311157297823662689037894645226208583;
    // [1]_1
    uint256 constant G1x  = 1;
    uint256 constant G1y  = 2;
    // [1]_2
    uint256 constant G2x1 = 10857046999023057135944570762232829481370756359578518086990519993285655852781;
    uint256 constant G2x2 = 11559732032986387107991004021392285783925812861821192530917403151452391805634;
    uint256 constant G2y1 = 8495653923123431417604973247489272438418190587263600148770280649306958101930;
    uint256 constant G2y2 = 4082367875863433681332203403145435568316851327593401208105741076214120093531;

    // Verifier preprocessed input x·[1]_2: the vkey's X_2, [tau]_2
    uint256 constant X2x1 = 21831381940315734285607113342023901060522397560371972897001948545212302161822;
    uint256 constant X2x2 = 17231025384763736816414546592865244497437017442647097510447326538965263639101;
    uint256 constant X2y1 = 2388026358213174446665280700919698872609886601280537296205114254867301080648;
    uint256 constant X2y2 = 11507326595632554467052522095592665270651932854513688777769618397986436103170;

    // The vkey's digest mod r, the first value of the transcript
    uint256 constant DIGEST = 18251438231356955361895171878011201186838108164977877081486940065832090131766;

    // The commitments of the fixed f_i, the vkey's: never the proof's
    uint256 constant f0x = 19573761451558791146226214640171246995539838761881667881680761549587441659716;
    uint256 constant f0y = 14197247730542030129742498883637073871063644820820915223585966674364231218749;
    uint256 constant f1x = 14549828975748461929610615568523374540236214528319232172803830048540616671856;
    uint256 constant f1y = 9427574348585946786196869084912168403129029584809827137178717339728698522210;
    uint256 constant f2x = 12538749938208312323235953005704558633727415088234566760143761554192307904797;
    uint256 constant f2y = 852757597599400124383701304743157055532480041050379105000885594745611800890;
    uint256 constant f3x = 1629218841826553250808522290457231574509104745320440655007676224910579415807;
    uint256 constant f3y = 11969251427647224160527854216675576905598723981155430914351102250810251245612;

    // Proof calldata: the byte offset of every value (proof.rs, Proof::to_bytes)
    uint256 constant pF4 = 4; // [f4]_1, stage 1
    uint256 constant pF5 = 68; // [f5]_1, stage 1
    uint256 constant pF6 = 132; // [f6]_1, stage 1
    uint256 constant pF7 = 196; // [f7]_1, stage 1
    uint256 constant pF8 = 260; // [f8]_1, stage 2
    uint256 constant pF9 = 324; // [f9]_1, stage 2
    uint256 constant pF10 = 388; // [f10]_1, stage 3
    uint256 constant pW = 452; // [W]_1
    uint256 constant pWp = 516; // [W']_1
    uint256 constant pEval2 = 580; // Wrap.S[0] (evMap[2])
    uint256 constant pEval3 = 612; // Wrap.S[1] (evMap[3])
    uint256 constant pEval4 = 644; // Wrap.S[2] (evMap[4])
    uint256 constant pEval5 = 676; // Wrap.S[3] (evMap[5])
    uint256 constant pEval6 = 708; // Wrap.S[4] (evMap[6])
    uint256 constant pEval7 = 740; // Wrap.S[5] (evMap[7])
    uint256 constant pEval8 = 772; // Wrap.S[6] (evMap[8])
    uint256 constant pEval9 = 804; // Wrap.S[7] (evMap[9])
    uint256 constant pEval10 = 836; // Wrap.S[8] (evMap[10])
    uint256 constant pEval11 = 868; // Wrap.S[9] (evMap[11])
    uint256 constant pEval12 = 900; // Wrap.S[10] (evMap[12])
    uint256 constant pEval13 = 932; // Wrap.S[11] (evMap[13])
    uint256 constant pEval14 = 964; // Wrap.S[12] (evMap[14])
    uint256 constant pEval15 = 996; // Wrap.S[13] (evMap[15])
    uint256 constant pEval16 = 1028; // Wrap.S[14] (evMap[16])
    uint256 constant pEval17 = 1060; // Wrap.S[15] (evMap[17])
    uint256 constant pEval18 = 1092; // Wrap.S[16] (evMap[18])
    uint256 constant pEval19 = 1124; // Wrap.S[17] (evMap[19])
    uint256 constant pEval20 = 1156; // Wrap.C[0] (evMap[20])
    uint256 constant pEval21 = 1188; // Wrap.C[1] (evMap[21])
    uint256 constant pEval22 = 1220; // Wrap.C[2] (evMap[22])
    uint256 constant pEval23 = 1252; // Wrap.C[3] (evMap[23])
    uint256 constant pEval24 = 1284; // Wrap.C[4] (evMap[24])
    uint256 constant pEval25 = 1316; // Wrap.PLONK (evMap[25])
    uint256 constant pEval26 = 1348; // Wrap.RANGE_CHECK (evMap[26])
    uint256 constant pEval27 = 1380; // Wrap.G[0] (evMap[27])
    uint256 constant pEval28 = 1412; // Wrap.G[1] (evMap[28])
    uint256 constant pEval29 = 1444; // Wrap.G[2] (evMap[29])
    uint256 constant pEval30 = 1476; // Wrap.G[3] (evMap[30])
    uint256 constant pEval31 = 1508; // Wrap.G[4] (evMap[31])
    uint256 constant pEval32 = 1540; // Wrap.G[5] (evMap[32])
    uint256 constant pEval33 = 1572; // Wrap.G[6] (evMap[33])
    uint256 constant pEval34 = 1604; // Wrap.G[7] (evMap[34])
    uint256 constant pEval35 = 1636; // Wrap.HOLD (evMap[35])
    uint256 constant pEval36 = 1668; // Wrap.INIT_NODE (evMap[36])
    uint256 constant pEval37 = 1700; // Wrap.INIT_CHUNK (evMap[37])
    uint256 constant pEval38 = 1732; // Wrap.INIT_PARENT (evMap[38])
    uint256 constant pEval39 = 1764; // Wrap.IN_NODE (evMap[39])
    uint256 constant pEval40 = 1796; // Wrap.IN_CHUNK (evMap[40])
    uint256 constant pEval41 = 1828; // Wrap.IN_PARENT (evMap[41])
    uint256 constant pEval42 = 1860; // Wrap.BLOCKLEN_ROW (evMap[42])
    uint256 constant pEval43 = 1892; // Wrap.FLAGS (evMap[43])
    uint256 constant pEval44 = 1924; // Wrap.MSG_KEY_X (evMap[44])
    uint256 constant pEval45 = 1956; // Wrap.MSG_KEY_Y (evMap[45])
    uint256 constant pEval46 = 1988; // Wrap.MSG_MUL (evMap[46])
    uint256 constant pEval47 = 2020; // Wrap.FF[0] (evMap[47])
    uint256 constant pEval48 = 2052; // Wrap.FF[1] (evMap[48])
    uint256 constant pEval49 = 2084; // Wrap.FF[2] (evMap[49])
    uint256 constant pEval50 = 2116; // Wrap.FF[3] (evMap[50])
    uint256 constant pEval51 = 2148; // Wrap.FF[4] (evMap[51])
    uint256 constant pEval52 = 2180; // Wrap.FF[5] (evMap[52])
    uint256 constant pEval53 = 2212; // Wrap.FF[6] (evMap[53])
    uint256 constant pEval54 = 2244; // Wrap.FF[7] (evMap[54])
    uint256 constant pEval55 = 2276; // Wrap.FF_NODE (evMap[55])
    uint256 constant pEval56 = 2308; // Wrap.FF_CV (evMap[56])
    uint256 constant pEval57 = 2340; // Wrap.FF_IV[0] (evMap[57])
    uint256 constant pEval58 = 2372; // Wrap.FF_IV[1] (evMap[58])
    uint256 constant pEval59 = 2404; // Wrap.A (evMap[59])
    uint256 constant pEval60 = 2436; // Wrap.B (evMap[60])
    uint256 constant pEval61 = 2468; // Wrap.ROTATION (evMap[61])
    uint256 constant pEval62 = 2500; // Wrap.C_ROT[0] (evMap[62])
    uint256 constant pEval63 = 2532; // Wrap.C_ROT[1] (evMap[63])
    uint256 constant pEval64 = 2564; // Wrap.PUBLICS_ROW[0] (evMap[64])
    uint256 constant pEval65 = 2596; // Wrap.ID (evMap[65])
    uint256 constant pEval66 = 2628; // __L1__ (evMap[66])
    uint256 constant pEval174 = 2660; // __L1__w (evMap[174])
    uint256 constant pEval0 = 2692; // gsumw-1 (evMap[0])
    uint256 constant pEval1 = 2724; // gprodw-1 (evMap[1])
    uint256 constant pEval67 = 2756; // a[0] (evMap[67])
    uint256 constant pEval68 = 2788; // a[1] (evMap[68])
    uint256 constant pEval69 = 2820; // a[2] (evMap[69])
    uint256 constant pEval70 = 2852; // a[3] (evMap[70])
    uint256 constant pEval71 = 2884; // a[4] (evMap[71])
    uint256 constant pEval72 = 2916; // a[5] (evMap[72])
    uint256 constant pEval73 = 2948; // a[6] (evMap[73])
    uint256 constant pEval74 = 2980; // a[7] (evMap[74])
    uint256 constant pEval75 = 3012; // a[8] (evMap[75])
    uint256 constant pEval76 = 3044; // a[9] (evMap[76])
    uint256 constant pEval77 = 3076; // a[10] (evMap[77])
    uint256 constant pEval78 = 3108; // a[11] (evMap[78])
    uint256 constant pEval79 = 3140; // a[12] (evMap[79])
    uint256 constant pEval80 = 3172; // a[13] (evMap[80])
    uint256 constant pEval81 = 3204; // a[14] (evMap[81])
    uint256 constant pEval82 = 3236; // a[15] (evMap[82])
    uint256 constant pEval83 = 3268; // a[16] (evMap[83])
    uint256 constant pEval84 = 3300; // a[17] (evMap[84])
    uint256 constant pEval85 = 3332; // st[0] (evMap[85])
    uint256 constant pEval86 = 3364; // st[1] (evMap[86])
    uint256 constant pEval87 = 3396; // st[2] (evMap[87])
    uint256 constant pEval88 = 3428; // st[3] (evMap[88])
    uint256 constant pEval89 = 3460; // st[4] (evMap[89])
    uint256 constant pEval90 = 3492; // st[5] (evMap[90])
    uint256 constant pEval91 = 3524; // st[6] (evMap[91])
    uint256 constant pEval92 = 3556; // st[7] (evMap[92])
    uint256 constant pEval93 = 3588; // st[8] (evMap[93])
    uint256 constant pEval94 = 3620; // st[9] (evMap[94])
    uint256 constant pEval95 = 3652; // st[10] (evMap[95])
    uint256 constant pEval96 = 3684; // st[11] (evMap[96])
    uint256 constant pEval97 = 3716; // st[12] (evMap[97])
    uint256 constant pEval98 = 3748; // st[13] (evMap[98])
    uint256 constant pEval99 = 3780; // st[14] (evMap[99])
    uint256 constant pEval100 = 3812; // st[15] (evMap[100])
    uint256 constant pEval101 = 3844; // va[0] (evMap[101])
    uint256 constant pEval102 = 3876; // va[1] (evMap[102])
    uint256 constant pEval103 = 3908; // vc (evMap[103])
    uint256 constant pEval104 = 3940; // vb[0] (evMap[104])
    uint256 constant pEval105 = 3972; // vb[1] (evMap[105])
    uint256 constant pEval106 = 4004; // vb[2] (evMap[106])
    uint256 constant pEval107 = 4036; // vb[3] (evMap[107])
    uint256 constant pEval108 = 4068; // vd[0] (evMap[108])
    uint256 constant pEval109 = 4100; // vd[1] (evMap[109])
    uint256 constant pEval110 = 4132; // vd[2] (evMap[110])
    uint256 constant pEval111 = 4164; // vd[3] (evMap[111])
    uint256 constant pEval112 = 4196; // x[0] (evMap[112])
    uint256 constant pEval113 = 4228; // x[1] (evMap[113])
    uint256 constant pEval114 = 4260; // y[0] (evMap[114])
    uint256 constant pEval115 = 4292; // y[1] (evMap[115])
    uint256 constant pEval116 = 4324; // va_p[0] (evMap[116])
    uint256 constant pEval117 = 4356; // va_p[1] (evMap[117])
    uint256 constant pEval118 = 4388; // va_p[2] (evMap[118])
    uint256 constant pEval119 = 4420; // va_p[3] (evMap[119])
    uint256 constant pEval120 = 4452; // vd_p[0] (evMap[120])
    uint256 constant pEval121 = 4484; // vd_p[1] (evMap[121])
    uint256 constant pEval122 = 4516; // vd_p[2] (evMap[122])
    uint256 constant pEval123 = 4548; // vd_p[3] (evMap[123])
    uint256 constant pEval124 = 4580; // vc_p[0] (evMap[124])
    uint256 constant pEval125 = 4612; // vc_p[1] (evMap[125])
    uint256 constant pEval126 = 4644; // vc_p[2] (evMap[126])
    uint256 constant pEval127 = 4676; // vc_p[3] (evMap[127])
    uint256 constant pEval128 = 4708; // vb_p_s[0][0] (evMap[128])
    uint256 constant pEval129 = 4740; // vb_p_s[0][1] (evMap[129])
    uint256 constant pEval130 = 4772; // vb_p_s[1][0] (evMap[130])
    uint256 constant pEval131 = 4804; // vb_p_s[1][1] (evMap[131])
    uint256 constant pEval132 = 4836; // vb_p_s[2][0] (evMap[132])
    uint256 constant pEval133 = 4868; // vb_p_s[2][1] (evMap[133])
    uint256 constant pEval134 = 4900; // vb_p_s[3][0] (evMap[134])
    uint256 constant pEval135 = 4932; // vb_p_s[3][1] (evMap[135])
    uint256 constant pEval136 = 4964; // va_pp[0] (evMap[136])
    uint256 constant pEval137 = 4996; // va_pp[1] (evMap[137])
    uint256 constant pEval138 = 5028; // va_pp[2] (evMap[138])
    uint256 constant pEval139 = 5060; // va_pp[3] (evMap[139])
    uint256 constant pEval140 = 5092; // vd_pp[0] (evMap[140])
    uint256 constant pEval141 = 5124; // vd_pp[1] (evMap[141])
    uint256 constant pEval142 = 5156; // vd_pp[2] (evMap[142])
    uint256 constant pEval143 = 5188; // vd_pp[3] (evMap[143])
    uint256 constant pEval144 = 5220; // vc_pp[0] (evMap[144])
    uint256 constant pEval145 = 5252; // vc_pp[1] (evMap[145])
    uint256 constant pEval146 = 5284; // vc_pp[2] (evMap[146])
    uint256 constant pEval147 = 5316; // vc_pp[3] (evMap[147])
    uint256 constant pEval148 = 5348; // vb_pp_xor[0] (evMap[148])
    uint256 constant pEval149 = 5380; // vb_pp_xor[1] (evMap[149])
    uint256 constant pEval150 = 5412; // vb_pp_xor[2] (evMap[150])
    uint256 constant pEval151 = 5444; // vb_pp_xor[3] (evMap[151])
    uint256 constant pEval152 = 5476; // vb_pp_t (evMap[152])
    uint256 constant pEval153 = 5508; // d_inv (evMap[153])
    uint256 constant pEval154 = 5540; // mul_table (evMap[154])
    uint256 constant pEval155 = 5572; // mul_range (evMap[155])
    uint256 constant pEval156 = 5604; // gsum (evMap[156])
    uint256 constant pEval157 = 5636; // im_cluster[0] (evMap[157])
    uint256 constant pEval158 = 5668; // im_cluster[1] (evMap[158])
    uint256 constant pEval159 = 5700; // im_cluster[2] (evMap[159])
    uint256 constant pEval160 = 5732; // im_cluster[3] (evMap[160])
    uint256 constant pEval161 = 5764; // im_cluster[4] (evMap[161])
    uint256 constant pEval162 = 5796; // im_cluster[5] (evMap[162])
    uint256 constant pEval163 = 5828; // im_cluster[6] (evMap[163])
    uint256 constant pEval164 = 5860; // im_cluster[7] (evMap[164])
    uint256 constant pEval165 = 5892; // im_cluster[8] (evMap[165])
    uint256 constant pEval166 = 5924; // im_cluster[9] (evMap[166])
    uint256 constant pEval167 = 5956; // im_cluster[10] (evMap[167])
    uint256 constant pEval168 = 5988; // im_cluster[11] (evMap[168])
    uint256 constant pEval169 = 6020; // gprod (evMap[169])
    uint256 constant pEval170 = 6052; // im_low[0] (evMap[170])
    uint256 constant pEval171 = 6084; // im_low[1] (evMap[171])
    uint256 constant pEval172 = 6116; // im_low[2] (evMap[172])
    uint256 constant pEval173 = 6148; // im_low[3] (evMap[173])
    uint256 constant pEval175 = 6180; // st[0]w (evMap[175])
    uint256 constant pEval176 = 6212; // st[1]w (evMap[176])
    uint256 constant pEval177 = 6244; // st[2]w (evMap[177])
    uint256 constant pEval178 = 6276; // st[3]w (evMap[178])
    uint256 constant pEval179 = 6308; // st[4]w (evMap[179])
    uint256 constant pEval180 = 6340; // st[5]w (evMap[180])
    uint256 constant pEval181 = 6372; // st[6]w (evMap[181])
    uint256 constant pEval182 = 6404; // st[7]w (evMap[182])
    uint256 constant pEval183 = 6436; // st[8]w (evMap[183])
    uint256 constant pEval184 = 6468; // st[9]w (evMap[184])
    uint256 constant pEval185 = 6500; // st[10]w (evMap[185])
    uint256 constant pEval186 = 6532; // st[11]w (evMap[186])
    uint256 constant pEval187 = 6564; // st[12]w (evMap[187])
    uint256 constant pEval188 = 6596; // st[13]w (evMap[188])
    uint256 constant pEval189 = 6628; // st[14]w (evMap[189])
    uint256 constant pEval190 = 6660; // st[15]w (evMap[190])
    uint256 constant pInv = 6692; // inv = 1/(prod_i Z_T_i(y), i >= 1, times the Lagrange denominators)
    uint256 constant pInvZh = 6724; // invZh = 1/Z_H(xi)
    // The publics, the unnamed `uint256[]` (pubSignals), follow at 6756.

    // Memory data, at absolute addresses from 0x80 (solidity.rs, MEM_BASE)
    uint256 constant pCh2_0 = 128; // challenge 0 of stage 2
    uint256 constant pCh2_1 = 160; // challenge 1 of stage 2
    uint256 constant pStdVc = 192; // std_vc
    uint256 constant pXiSeed = 224; // xiSeed (std_xi)
    uint256 constant pAlpha = 256; // alpha_S
    uint256 constant pY = 288; // y
    uint256 constant pXi = 320; // xi = xiSeed^powerW
    uint256 constant pXiN = 352; // xi^N
    uint256 constant pZh = 384; // Z_H(xi) = xi^N - 1
    uint256 constant pZi0 = 416; // Zi of boundaries[0], EveryRow
    uint256 constant pQ = 448; // Q(xi)
    uint256 constant pYk1 = 480; // y^1
    uint256 constant pYk2 = 512; // y^2
    uint256 constant pYk16 = 544; // y^16
    uint256 constant pYk18 = 576; // y^18
    uint256 constant pYk24 = 608; // y^24
    uint256 constant pYk26 = 640; // y^26
    uint256 constant pYk29 = 672; // y^29
    uint256 constant pZ0 = 704; // xi*w^s of each offset s of k = 16, offsets [0]
    uint256 constant pZt0 = 736; // Z_T(y) of k = 16, offsets [0]
    uint256 constant pZ1 = 768; // xi*w^s of each offset s of k = 24, offsets [0]
    uint256 constant pZt1 = 800; // Z_T(y) of k = 24, offsets [0]
    uint256 constant pZ2 = 832; // xi*w^s of each offset s of k = 1, offsets [0, 1]
    uint256 constant pZt2 = 896; // Z_T(y) of k = 1, offsets [0, 1]
    uint256 constant pL2 = 928; // the CRT factor of each offset of k = 1, offsets [0, 1]
    uint256 constant pXiNeg1 = 992; // xi^-1
    uint256 constant pZ3 = 1024; // xi*w^s of each offset s of k = 18, offsets [0]
    uint256 constant pZt3 = 1056; // Z_T(y) of k = 18, offsets [0]
    uint256 constant pZ4 = 1088; // xi*w^s of each offset s of k = 26, offsets [0]
    uint256 constant pZt4 = 1120; // Z_T(y) of k = 26, offsets [0]
    uint256 constant pZ5 = 1152; // xi*w^s of each offset s of k = 29, offsets [0]
    uint256 constant pZt5 = 1184; // Z_T(y) of k = 29, offsets [0]
    uint256 constant pZ6 = 1216; // xi*w^s of each offset s of k = 16, offsets [0, 1]
    uint256 constant pZt6 = 1280; // Z_T(y) of k = 16, offsets [0, 1]
    uint256 constant pL6 = 1312; // the CRT factor of each offset of k = 16, offsets [0, 1]
    uint256 constant pZ7 = 1376; // xi*w^s of each offset s of k = 2, offsets [-1, 0]
    uint256 constant pZt7 = 1440; // Z_T(y) of k = 2, offsets [-1, 0]
    uint256 constant pL7 = 1472; // the CRT factor of each offset of k = 2, offsets [-1, 0]
    uint256 constant pZ8 = 1536; // xi*w^s of each offset s of k = 1, offsets [0]
    uint256 constant pZt8 = 1568; // Z_T(y) of k = 1, offsets [0]
    uint256 constant pInvZt1 = 1600; // Z_T(y) of f1, then its inverse
    uint256 constant pInvZt2 = 1632; // Z_T(y) of f2, then its inverse
    uint256 constant pInvZt3 = 1664; // Z_T(y) of f3, then its inverse
    uint256 constant pInvZt4 = 1696; // Z_T(y) of f4, then its inverse
    uint256 constant pInvZt5 = 1728; // Z_T(y) of f5, then its inverse
    uint256 constant pInvZt6 = 1760; // Z_T(y) of f6, then its inverse
    uint256 constant pInvZt7 = 1792; // Z_T(y) of f7, then its inverse
    uint256 constant pInvZt8 = 1824; // Z_T(y) of f8, then its inverse
    uint256 constant pInvZt9 = 1856; // Z_T(y) of f9, then its inverse
    uint256 constant pInvZt10 = 1888; // Z_T(y) of f10, then its inverse
    uint256 constant pR0 = 1920; // r_0(y)
    uint256 constant pR1 = 1952; // r_1(y)
    uint256 constant pR2 = 1984; // r_2(y)
    uint256 constant pR3 = 2016; // r_3(y)
    uint256 constant pR4 = 2048; // r_4(y)
    uint256 constant pR5 = 2080; // r_5(y)
    uint256 constant pR6 = 2112; // r_6(y)
    uint256 constant pR7 = 2144; // r_7(y)
    uint256 constant pR8 = 2176; // r_8(y)
    uint256 constant pR9 = 2208; // r_9(y)
    uint256 constant pR10 = 2240; // r_10(y)
    uint256 constant pF = 2272; // [F]_1
    uint256 constant pE = 2336; // [E]_1
    uint256 constant pJ = 2400; // [J]_1
    uint256 constant pTmp = 2464; // the stored values of the qVerifier
    uint256 constant lastMem = 3136;

    // `proof` has no name: the contract reads its words at the constant offsets above.
    function verifyProof(bytes32[211] calldata, uint256[1] calldata) public view returns (bool) {
        assembly {
            mstore(0x60, q)
            // The proof is rejected: verifyProof returns false.
            function fail() {
                mstore(0, 0)
                return(0, 0x20)
            }

            // elements.js, frFromObject: every scalar below r. bad is whether one is not, accumulated
            // over them without a branch each.
            function checkFields(bad) {
                if bad {
                    fail()
                }
            }

            // elements.js, g1FromObject: a point of G1, affine, its coordinates below the base
            // field's q, not the point at infinity (0, 0), and on y^2 = x^3 + 3, which is G1: its
            // cofactor is 1.
            function checkPointBelongsToBN128Curve(p) {
                let x := calldataload(p)
                let y := calldataload(add(p, 32))
                if iszero(and(lt(x, qf), lt(y, qf))) {
                    fail()
                }
                if iszero(or(x, y)) {
                    fail()
                }
                if iszero(eq(mulmod(y, y, qf), addmod(mulmod(x, mulmod(x, x, qf), qf), 3, qf))) {
                    fail()
                }
            }

            // transcript.js, addPolCommitment: a point the transcript absorbs has no coordinate
            // below 2^192.
            function checkAbsorbed(p) {
                if or(iszero(shr(192, calldataload(p))), iszero(shr(192, calldataload(add(p, 32))))) {
                    fail()
                }
            }

            // STEPS 1-3 (verify.js; proof.js, fromObjectProof and fromObjectPublics): the
            // commitments, W and W' in G1; the evaluations, the pieces of Q, inv, invZh (and the
            // auxiliary inverses) and the publics in Fr. The lengths are the vkey's, fixed by the
            // types of the arguments; the vkey is this contract's constants.
            function checkInput() {
                checkPointBelongsToBN128Curve(4)
                checkPointBelongsToBN128Curve(68)
                checkPointBelongsToBN128Curve(132)
                checkPointBelongsToBN128Curve(196)
                checkPointBelongsToBN128Curve(260)
                checkPointBelongsToBN128Curve(324)
                checkPointBelongsToBN128Curve(388)
                checkPointBelongsToBN128Curve(452)
                checkPointBelongsToBN128Curve(516)
                // Those the transcript absorbs: the commitments and W (challenges.js).
                checkAbsorbed(4)
                checkAbsorbed(68)
                checkAbsorbed(132)
                checkAbsorbed(196)
                checkAbsorbed(260)
                checkAbsorbed(324)
                checkAbsorbed(388)
                checkAbsorbed(452)
                let r := mload(0x60)
                let bad := 0
                bad := or(bad, iszero(lt(calldataload(580), r)))
                bad := or(bad, iszero(lt(calldataload(612), r)))
                bad := or(bad, iszero(lt(calldataload(644), r)))
                bad := or(bad, iszero(lt(calldataload(676), r)))
                bad := or(bad, iszero(lt(calldataload(708), r)))
                bad := or(bad, iszero(lt(calldataload(740), r)))
                bad := or(bad, iszero(lt(calldataload(772), r)))
                bad := or(bad, iszero(lt(calldataload(804), r)))
                bad := or(bad, iszero(lt(calldataload(836), r)))
                bad := or(bad, iszero(lt(calldataload(868), r)))
                bad := or(bad, iszero(lt(calldataload(900), r)))
                bad := or(bad, iszero(lt(calldataload(932), r)))
                bad := or(bad, iszero(lt(calldataload(964), r)))
                bad := or(bad, iszero(lt(calldataload(996), r)))
                bad := or(bad, iszero(lt(calldataload(1028), r)))
                bad := or(bad, iszero(lt(calldataload(1060), r)))
                bad := or(bad, iszero(lt(calldataload(1092), r)))
                bad := or(bad, iszero(lt(calldataload(1124), r)))
                bad := or(bad, iszero(lt(calldataload(1156), r)))
                bad := or(bad, iszero(lt(calldataload(1188), r)))
                bad := or(bad, iszero(lt(calldataload(1220), r)))
                bad := or(bad, iszero(lt(calldataload(1252), r)))
                bad := or(bad, iszero(lt(calldataload(1284), r)))
                bad := or(bad, iszero(lt(calldataload(1316), r)))
                bad := or(bad, iszero(lt(calldataload(1348), r)))
                bad := or(bad, iszero(lt(calldataload(1380), r)))
                bad := or(bad, iszero(lt(calldataload(1412), r)))
                bad := or(bad, iszero(lt(calldataload(1444), r)))
                bad := or(bad, iszero(lt(calldataload(1476), r)))
                bad := or(bad, iszero(lt(calldataload(1508), r)))
                bad := or(bad, iszero(lt(calldataload(1540), r)))
                bad := or(bad, iszero(lt(calldataload(1572), r)))
                bad := or(bad, iszero(lt(calldataload(1604), r)))
                bad := or(bad, iszero(lt(calldataload(1636), r)))
                bad := or(bad, iszero(lt(calldataload(1668), r)))
                bad := or(bad, iszero(lt(calldataload(1700), r)))
                bad := or(bad, iszero(lt(calldataload(1732), r)))
                bad := or(bad, iszero(lt(calldataload(1764), r)))
                bad := or(bad, iszero(lt(calldataload(1796), r)))
                bad := or(bad, iszero(lt(calldataload(1828), r)))
                bad := or(bad, iszero(lt(calldataload(1860), r)))
                bad := or(bad, iszero(lt(calldataload(1892), r)))
                bad := or(bad, iszero(lt(calldataload(1924), r)))
                bad := or(bad, iszero(lt(calldataload(1956), r)))
                bad := or(bad, iszero(lt(calldataload(1988), r)))
                bad := or(bad, iszero(lt(calldataload(2020), r)))
                bad := or(bad, iszero(lt(calldataload(2052), r)))
                bad := or(bad, iszero(lt(calldataload(2084), r)))
                bad := or(bad, iszero(lt(calldataload(2116), r)))
                bad := or(bad, iszero(lt(calldataload(2148), r)))
                bad := or(bad, iszero(lt(calldataload(2180), r)))
                bad := or(bad, iszero(lt(calldataload(2212), r)))
                bad := or(bad, iszero(lt(calldataload(2244), r)))
                bad := or(bad, iszero(lt(calldataload(2276), r)))
                bad := or(bad, iszero(lt(calldataload(2308), r)))
                bad := or(bad, iszero(lt(calldataload(2340), r)))
                bad := or(bad, iszero(lt(calldataload(2372), r)))
                bad := or(bad, iszero(lt(calldataload(2404), r)))
                bad := or(bad, iszero(lt(calldataload(2436), r)))
                bad := or(bad, iszero(lt(calldataload(2468), r)))
                bad := or(bad, iszero(lt(calldataload(2500), r)))
                bad := or(bad, iszero(lt(calldataload(2532), r)))
                bad := or(bad, iszero(lt(calldataload(2564), r)))
                bad := or(bad, iszero(lt(calldataload(2596), r)))
                bad := or(bad, iszero(lt(calldataload(2628), r)))
                bad := or(bad, iszero(lt(calldataload(2660), r)))
                bad := or(bad, iszero(lt(calldataload(2692), r)))
                bad := or(bad, iszero(lt(calldataload(2724), r)))
                bad := or(bad, iszero(lt(calldataload(2756), r)))
                bad := or(bad, iszero(lt(calldataload(2788), r)))
                bad := or(bad, iszero(lt(calldataload(2820), r)))
                bad := or(bad, iszero(lt(calldataload(2852), r)))
                bad := or(bad, iszero(lt(calldataload(2884), r)))
                bad := or(bad, iszero(lt(calldataload(2916), r)))
                bad := or(bad, iszero(lt(calldataload(2948), r)))
                bad := or(bad, iszero(lt(calldataload(2980), r)))
                bad := or(bad, iszero(lt(calldataload(3012), r)))
                bad := or(bad, iszero(lt(calldataload(3044), r)))
                bad := or(bad, iszero(lt(calldataload(3076), r)))
                bad := or(bad, iszero(lt(calldataload(3108), r)))
                bad := or(bad, iszero(lt(calldataload(3140), r)))
                bad := or(bad, iszero(lt(calldataload(3172), r)))
                bad := or(bad, iszero(lt(calldataload(3204), r)))
                bad := or(bad, iszero(lt(calldataload(3236), r)))
                bad := or(bad, iszero(lt(calldataload(3268), r)))
                bad := or(bad, iszero(lt(calldataload(3300), r)))
                bad := or(bad, iszero(lt(calldataload(3332), r)))
                bad := or(bad, iszero(lt(calldataload(3364), r)))
                bad := or(bad, iszero(lt(calldataload(3396), r)))
                bad := or(bad, iszero(lt(calldataload(3428), r)))
                bad := or(bad, iszero(lt(calldataload(3460), r)))
                bad := or(bad, iszero(lt(calldataload(3492), r)))
                bad := or(bad, iszero(lt(calldataload(3524), r)))
                bad := or(bad, iszero(lt(calldataload(3556), r)))
                bad := or(bad, iszero(lt(calldataload(3588), r)))
                bad := or(bad, iszero(lt(calldataload(3620), r)))
                bad := or(bad, iszero(lt(calldataload(3652), r)))
                bad := or(bad, iszero(lt(calldataload(3684), r)))
                bad := or(bad, iszero(lt(calldataload(3716), r)))
                bad := or(bad, iszero(lt(calldataload(3748), r)))
                bad := or(bad, iszero(lt(calldataload(3780), r)))
                bad := or(bad, iszero(lt(calldataload(3812), r)))
                bad := or(bad, iszero(lt(calldataload(3844), r)))
                bad := or(bad, iszero(lt(calldataload(3876), r)))
                bad := or(bad, iszero(lt(calldataload(3908), r)))
                bad := or(bad, iszero(lt(calldataload(3940), r)))
                bad := or(bad, iszero(lt(calldataload(3972), r)))
                bad := or(bad, iszero(lt(calldataload(4004), r)))
                bad := or(bad, iszero(lt(calldataload(4036), r)))
                bad := or(bad, iszero(lt(calldataload(4068), r)))
                bad := or(bad, iszero(lt(calldataload(4100), r)))
                bad := or(bad, iszero(lt(calldataload(4132), r)))
                bad := or(bad, iszero(lt(calldataload(4164), r)))
                bad := or(bad, iszero(lt(calldataload(4196), r)))
                bad := or(bad, iszero(lt(calldataload(4228), r)))
                bad := or(bad, iszero(lt(calldataload(4260), r)))
                bad := or(bad, iszero(lt(calldataload(4292), r)))
                bad := or(bad, iszero(lt(calldataload(4324), r)))
                bad := or(bad, iszero(lt(calldataload(4356), r)))
                bad := or(bad, iszero(lt(calldataload(4388), r)))
                bad := or(bad, iszero(lt(calldataload(4420), r)))
                bad := or(bad, iszero(lt(calldataload(4452), r)))
                bad := or(bad, iszero(lt(calldataload(4484), r)))
                bad := or(bad, iszero(lt(calldataload(4516), r)))
                bad := or(bad, iszero(lt(calldataload(4548), r)))
                bad := or(bad, iszero(lt(calldataload(4580), r)))
                bad := or(bad, iszero(lt(calldataload(4612), r)))
                bad := or(bad, iszero(lt(calldataload(4644), r)))
                bad := or(bad, iszero(lt(calldataload(4676), r)))
                bad := or(bad, iszero(lt(calldataload(4708), r)))
                bad := or(bad, iszero(lt(calldataload(4740), r)))
                bad := or(bad, iszero(lt(calldataload(4772), r)))
                bad := or(bad, iszero(lt(calldataload(4804), r)))
                bad := or(bad, iszero(lt(calldataload(4836), r)))
                bad := or(bad, iszero(lt(calldataload(4868), r)))
                bad := or(bad, iszero(lt(calldataload(4900), r)))
                bad := or(bad, iszero(lt(calldataload(4932), r)))
                bad := or(bad, iszero(lt(calldataload(4964), r)))
                bad := or(bad, iszero(lt(calldataload(4996), r)))
                bad := or(bad, iszero(lt(calldataload(5028), r)))
                bad := or(bad, iszero(lt(calldataload(5060), r)))
                bad := or(bad, iszero(lt(calldataload(5092), r)))
                bad := or(bad, iszero(lt(calldataload(5124), r)))
                bad := or(bad, iszero(lt(calldataload(5156), r)))
                bad := or(bad, iszero(lt(calldataload(5188), r)))
                bad := or(bad, iszero(lt(calldataload(5220), r)))
                bad := or(bad, iszero(lt(calldataload(5252), r)))
                bad := or(bad, iszero(lt(calldataload(5284), r)))
                bad := or(bad, iszero(lt(calldataload(5316), r)))
                bad := or(bad, iszero(lt(calldataload(5348), r)))
                bad := or(bad, iszero(lt(calldataload(5380), r)))
                bad := or(bad, iszero(lt(calldataload(5412), r)))
                bad := or(bad, iszero(lt(calldataload(5444), r)))
                bad := or(bad, iszero(lt(calldataload(5476), r)))
                bad := or(bad, iszero(lt(calldataload(5508), r)))
                bad := or(bad, iszero(lt(calldataload(5540), r)))
                bad := or(bad, iszero(lt(calldataload(5572), r)))
                bad := or(bad, iszero(lt(calldataload(5604), r)))
                bad := or(bad, iszero(lt(calldataload(5636), r)))
                bad := or(bad, iszero(lt(calldataload(5668), r)))
                bad := or(bad, iszero(lt(calldataload(5700), r)))
                bad := or(bad, iszero(lt(calldataload(5732), r)))
                bad := or(bad, iszero(lt(calldataload(5764), r)))
                bad := or(bad, iszero(lt(calldataload(5796), r)))
                bad := or(bad, iszero(lt(calldataload(5828), r)))
                bad := or(bad, iszero(lt(calldataload(5860), r)))
                bad := or(bad, iszero(lt(calldataload(5892), r)))
                bad := or(bad, iszero(lt(calldataload(5924), r)))
                bad := or(bad, iszero(lt(calldataload(5956), r)))
                bad := or(bad, iszero(lt(calldataload(5988), r)))
                bad := or(bad, iszero(lt(calldataload(6020), r)))
                bad := or(bad, iszero(lt(calldataload(6052), r)))
                bad := or(bad, iszero(lt(calldataload(6084), r)))
                bad := or(bad, iszero(lt(calldataload(6116), r)))
                bad := or(bad, iszero(lt(calldataload(6148), r)))
                bad := or(bad, iszero(lt(calldataload(6180), r)))
                bad := or(bad, iszero(lt(calldataload(6212), r)))
                bad := or(bad, iszero(lt(calldataload(6244), r)))
                bad := or(bad, iszero(lt(calldataload(6276), r)))
                bad := or(bad, iszero(lt(calldataload(6308), r)))
                bad := or(bad, iszero(lt(calldataload(6340), r)))
                bad := or(bad, iszero(lt(calldataload(6372), r)))
                bad := or(bad, iszero(lt(calldataload(6404), r)))
                bad := or(bad, iszero(lt(calldataload(6436), r)))
                bad := or(bad, iszero(lt(calldataload(6468), r)))
                bad := or(bad, iszero(lt(calldataload(6500), r)))
                bad := or(bad, iszero(lt(calldataload(6532), r)))
                bad := or(bad, iszero(lt(calldataload(6564), r)))
                bad := or(bad, iszero(lt(calldataload(6596), r)))
                bad := or(bad, iszero(lt(calldataload(6628), r)))
                bad := or(bad, iszero(lt(calldataload(6660), r)))
                bad := or(bad, iszero(lt(calldataload(6692), r)))
                bad := or(bad, iszero(lt(calldataload(6724), r)))
                bad := or(bad, iszero(lt(calldataload(6756), r)))
                checkFields(bad)
            }

            // STEP 4: challenges.js, computeChallenges, the transcript. Its buffer, pT,
            // holds what is absorbed since the last squeeze, which is keccak256 of it mod r and
            // then the buffer's only value (transcript.js, squeeze).
            function computeChallenges(pPublic) {
                let pT := mload(0x40)
                let c
                // Step 1: digest mod r, the number of instances of the AIR (1), the publics
                mstore(pT, DIGEST)
                mstore(add(pT, 32), 1)
                calldatacopy(add(pT, 64), pPublic, 32)
                // Step 2, stage 1: the commitments of its f
                calldatacopy(add(pT, 96), pF4, 256)
                // challenge 0 of stage 2
                c := mod(keccak256(pT, 352), mload(0x60))
                mstore(pCh2_0, c)
                mstore(pT, c)
                // challenge 1 of stage 2
                c := mod(keccak256(pT, 32), mload(0x60))
                mstore(pCh2_1, c)
                mstore(pT, c)
                // Step 2, stage 2: the commitments of its f
                calldatacopy(add(pT, 32), pF8, 128)
                // Step 3: std_vc; the commitments of Q; xiSeed
                c := mod(keccak256(pT, 160), mload(0x60))
                mstore(pStdVc, c)
                mstore(pT, c)
                calldatacopy(add(pT, 32), pF10, 64)
                c := mod(keccak256(pT, 96), mload(0x60))
                mstore(pXiSeed, c)
                mstore(pT, c)
                // Step 4: the evaluations, in the order of the proof, and the pieces of Q
                calldatacopy(add(pT, 32), pEval2, 6112)
                c := mod(keccak256(pT, 6144), mload(0x60))
                mstore(pAlpha, c)
                mstore(pT, c)
                // Step 5: [W]_1; y
                calldatacopy(add(pT, 32), pW, 64)
                c := mod(keccak256(pT, 96), mload(0x60))
                mstore(pY, c)
                mstore(pT, c)
            }

            // b^e mod r.
            function powMod(b, e) -> result {
                result := 1
                for { } gt(e, 0) { e := shr(1, e) } {
                    if and(e, 1) {
                        result := mulmod(result, b, mload(0x60))
                    }
                    b := mulmod(b, b, mload(0x60))
                }
            }

            // STEP 5 (verify.js): xi = xiSeed^powerW, and Z_H(xi) = xi^N - 1 checked against the
            // proof's invZh.
            function computeZh() {
                let xi := powMod(mload(pXiSeed), 54288)
                mstore(pXi, xi)
                let xin := xi
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                xin := mulmod(xin, xin, mload(0x60))
                mstore(pXiN, xin)
                let zh := addmod(xin, sub(mload(0x60), 1), mload(0x60))
                mstore(pZh, zh)
                if iszero(eq(mulmod(zh, calldataload(pInvZh), mload(0x60)), 1)) {
                    fail()
                }
            }

            // STEP 5: qverifier.js, computeZi, Zi(D) at xi for each boundary D:
            // 1/Z_H(xi) for everyRow, Z_H(xi)/Z_D(xi) for the others.
            function computeZi() {
                let xi := mload(pXi)
                let zh := mload(pZh)
                // boundaries[0], everyRow: 1/Z_H(xi): the proof's invZh
                mstore(pZi0, calldataload(pInvZh))
            }

            // STEP 5: verify.js, computeQ, and qverifier.js, executeCode: Q(xi) with the vkey's
            // qVerifier, the STARK's fold of the constraints, over the evaluations.
            function computeQ(pPublic) {
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), mulmod(calldataload(pEval25), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval20), mulmod(calldataload(pEval67), calldataload(pEval68), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval21), calldataload(pEval67), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval22), calldataload(pEval68), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval23), calldataload(pEval69), mload(0x60)), mload(0x60)), calldataload(pEval24), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval25), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval20), mulmod(calldataload(pEval70), calldataload(pEval71), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval21), calldataload(pEval70), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval22), calldataload(pEval71), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval23), calldataload(pEval72), mload(0x60)), mload(0x60)), calldataload(pEval24), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mulmod(calldataload(pEval25), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval20), mulmod(calldataload(pEval73), calldataload(pEval74), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval21), calldataload(pEval73), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval22), calldataload(pEval74), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval23), calldataload(pEval75), mload(0x60)), mload(0x60)), calldataload(pEval24), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mulmod(calldataload(pEval25), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval20), mulmod(calldataload(pEval76), calldataload(pEval77), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval21), calldataload(pEval76), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval22), calldataload(pEval77), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval23), calldataload(pEval78), mload(0x60)), mload(0x60)), calldataload(pEval24), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mulmod(calldataload(pEval25), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval20), mulmod(calldataload(pEval79), calldataload(pEval80), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval21), calldataload(pEval79), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval22), calldataload(pEval80), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval23), calldataload(pEval81), mload(0x60)), mload(0x60)), calldataload(pEval24), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mulmod(calldataload(pEval25), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval20), mulmod(calldataload(pEval82), calldataload(pEval83), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval21), calldataload(pEval82), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval22), calldataload(pEval83), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval23), calldataload(pEval84), mload(0x60)), mload(0x60)), calldataload(pEval24), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mulmod(calldataload(pEval26), addmod(calldataload(pEval67), sub(mload(0x60), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval20), calldataload(pEval68), mload(0x60)), mulmod(calldataload(pEval21), calldataload(pEval69), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval22), calldataload(pEval70), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval23), calldataload(pEval71), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval24), calldataload(pEval72), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mulmod(calldataload(pEval26), addmod(calldataload(pEval73), sub(mload(0x60), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval20), calldataload(pEval74), mload(0x60)), mulmod(calldataload(pEval21), calldataload(pEval75), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval22), calldataload(pEval76), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval23), calldataload(pEval77), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval24), calldataload(pEval78), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mulmod(calldataload(pEval26), addmod(calldataload(pEval79), sub(mload(0x60), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval20), calldataload(pEval80), mload(0x60)), mulmod(calldataload(pEval21), calldataload(pEval81), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval22), calldataload(pEval82), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval23), calldataload(pEval83), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval24), calldataload(pEval84), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 32), addmod(addmod(addmod(addmod(addmod(addmod(addmod(calldataload(pEval27), calldataload(pEval28), mload(0x60)), calldataload(pEval29), mload(0x60)), calldataload(pEval30), mload(0x60)), calldataload(pEval31), mload(0x60)), calldataload(pEval32), mload(0x60)), calldataload(pEval33), mload(0x60)), calldataload(pEval34), mload(0x60)))
                mstore(add(pTmp, 64), addmod(calldataload(pEval101), mulmod(calldataload(pEval102), 65536, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 96), addmod(mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 64)), mload(0x60)), sub(mload(0x60), addmod(addmod(addmod(addmod(addmod(addmod(addmod(mulmod(calldataload(pEval27), calldataload(pEval85), mload(0x60)), mulmod(calldataload(pEval28), calldataload(pEval86), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval29), calldataload(pEval87), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval30), calldataload(pEval88), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval31), calldataload(pEval85), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval32), calldataload(pEval86), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval33), calldataload(pEval87), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval34), calldataload(pEval88), mload(0x60)), mload(0x60))), mload(0x60)))
                mstore(add(pTmp, 128), addmod(addmod(addmod(calldataload(pEval104), mulmod(calldataload(pEval105), 256, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval106), 65536, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval107), 16777216, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 160), addmod(mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 128)), mload(0x60)), sub(mload(0x60), addmod(addmod(addmod(addmod(addmod(addmod(addmod(mulmod(calldataload(pEval27), calldataload(pEval89), mload(0x60)), mulmod(calldataload(pEval28), calldataload(pEval90), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval29), calldataload(pEval91), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval30), calldataload(pEval92), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval31), calldataload(pEval90), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval32), calldataload(pEval91), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval33), calldataload(pEval92), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval34), calldataload(pEval89), mload(0x60)), mload(0x60))), mload(0x60)))
                mstore(add(pTmp, 192), addmod(mulmod(mload(add(pTmp, 32)), calldataload(pEval103), mload(0x60)), sub(mload(0x60), addmod(addmod(addmod(addmod(addmod(addmod(addmod(mulmod(calldataload(pEval27), calldataload(pEval93), mload(0x60)), mulmod(calldataload(pEval28), calldataload(pEval94), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval29), calldataload(pEval95), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval30), calldataload(pEval96), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval31), calldataload(pEval95), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval32), calldataload(pEval96), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval33), calldataload(pEval93), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval34), calldataload(pEval94), mload(0x60)), mload(0x60))), mload(0x60)))
                mstore(add(pTmp, 224), addmod(addmod(addmod(calldataload(pEval108), mulmod(calldataload(pEval109), 256, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval110), 65536, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval111), 16777216, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 256), addmod(mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 224)), mload(0x60)), sub(mload(0x60), addmod(addmod(addmod(addmod(addmod(addmod(addmod(mulmod(calldataload(pEval27), calldataload(pEval97), mload(0x60)), mulmod(calldataload(pEval28), calldataload(pEval98), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval29), calldataload(pEval99), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval30), calldataload(pEval100), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval31), calldataload(pEval100), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval32), calldataload(pEval97), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval33), calldataload(pEval98), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval34), calldataload(pEval99), mload(0x60)), mload(0x60))), mload(0x60)))
                mstore(add(pTmp, 288), addmod(addmod(addmod(calldataload(pEval116), mulmod(calldataload(pEval117), 256, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval118), 65536, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval119), 16777216, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 320), addmod(calldataload(pEval112), mulmod(calldataload(pEval113), 65536, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(addmod(mload(add(pTmp, 288)), sub(mload(0x60), mload(add(pTmp, 64))), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 128))), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 320))), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mload(add(pTmp, 96)), mload(0x60)), mload(0x60)), mload(add(pTmp, 160)), mload(0x60)), mload(0x60)), mload(add(pTmp, 192)), mload(0x60)), mload(0x60)), mload(add(pTmp, 256)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mulmod(mulmod(mload(add(pTmp, 64)), addmod(mload(add(pTmp, 64)), 4294967296, mload(0x60)), mload(0x60)), addmod(mload(add(pTmp, 64)), 8589934592, mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(addmod(calldataload(pEval124), mulmod(calldataload(pEval125), 256, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval126), 65536, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval127), 16777216, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 96), addmod(addmod(addmod(calldataload(pEval120), mulmod(calldataload(pEval121), 256, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval122), 65536, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval123), 16777216, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 160), addmod(addmod(mload(add(pTmp, 64)), sub(mload(0x60), calldataload(pEval103)), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 96))), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(addmod(calldataload(pEval136), mulmod(calldataload(pEval137), 256, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval138), 65536, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval139), 16777216, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 256), addmod(calldataload(pEval131), calldataload(pEval132), mload(0x60)))
                mstore(add(pTmp, 352), addmod(calldataload(pEval133), calldataload(pEval134), mload(0x60)))
                mstore(add(pTmp, 384), addmod(calldataload(pEval128), calldataload(pEval135), mload(0x60)))
                mstore(add(pTmp, 416), addmod(calldataload(pEval129), calldataload(pEval130), mload(0x60)))
                mstore(add(pTmp, 448), addmod(calldataload(pEval114), mulmod(calldataload(pEval115), 65536, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 480), addmod(addmod(addmod(mload(add(pTmp, 192)), sub(mload(0x60), mload(add(pTmp, 288))), mload(0x60)), sub(mload(0x60), addmod(addmod(addmod(mload(add(pTmp, 256)), mulmod(mload(add(pTmp, 352)), 256, mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 384)), 65536, mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 416)), 16777216, mload(0x60)), mload(0x60))), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 448))), mload(0x60)))
                mstore(add(pTmp, 512), addmod(addmod(addmod(calldataload(pEval144), mulmod(calldataload(pEval145), 256, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval146), 65536, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval147), 16777216, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 544), addmod(addmod(addmod(calldataload(pEval140), mulmod(calldataload(pEval141), 256, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval142), 65536, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval143), 16777216, mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 576), addmod(addmod(mload(add(pTmp, 512)), sub(mload(0x60), mload(add(pTmp, 64))), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 544))), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mulmod(mload(add(pTmp, 160)), addmod(mload(add(pTmp, 160)), 4294967296, mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mulmod(mulmod(mload(add(pTmp, 480)), addmod(mload(add(pTmp, 480)), 4294967296, mload(0x60)), mload(0x60)), addmod(mload(add(pTmp, 480)), 8589934592, mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mulmod(mload(add(pTmp, 576)), addmod(mload(add(pTmp, 576)), 4294967296, mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 160), addmod(mload(add(pTmp, 32)), calldataload(pEval35), mload(0x60)))
                mstore(add(pTmp, 480), addmod(mload(add(pTmp, 192)), sub(mload(0x60), calldataload(pEval85)), mload(0x60)))
                mstore(add(pTmp, 576), addmod(mload(add(pTmp, 192)), sub(mload(0x60), calldataload(pEval86)), mload(0x60)))
                mstore(add(pTmp, 608), addmod(mload(add(pTmp, 192)), sub(mload(0x60), calldataload(pEval87)), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mulmod(mulmod(mload(add(pTmp, 32)), calldataload(pEval152), mload(0x60)), addmod(1, sub(mload(0x60), calldataload(pEval152)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval175), sub(mload(0x60), calldataload(pEval85)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval27), mload(add(pTmp, 480)), mload(0x60)), mulmod(calldataload(pEval31), mload(add(pTmp, 480)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval176), sub(mload(0x60), calldataload(pEval86)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval28), mload(add(pTmp, 576)), mload(0x60)), mulmod(calldataload(pEval32), mload(add(pTmp, 576)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval177), sub(mload(0x60), calldataload(pEval87)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval29), mload(add(pTmp, 608)), mload(0x60)), mulmod(calldataload(pEval33), mload(add(pTmp, 608)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 480), addmod(mload(add(pTmp, 192)), sub(mload(0x60), calldataload(pEval88)), mload(0x60)))
                mstore(add(pTmp, 576), addmod(addmod(mulmod(2, addmod(addmod(addmod(calldataload(pEval149), mulmod(calldataload(pEval150), 256, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval151), 65536, mload(0x60)), mload(0x60)), mulmod(calldataload(pEval148), 16777216, mload(0x60)), mload(0x60)), mload(0x60)), sub(mload(0x60), mulmod(4294967296, calldataload(pEval152), mload(0x60))), mload(0x60)), calldataload(pEval152), mload(0x60)))
                mstore(add(pTmp, 608), addmod(mload(add(pTmp, 576)), sub(mload(0x60), calldataload(pEval89)), mload(0x60)))
                mstore(add(pTmp, 640), addmod(mload(add(pTmp, 576)), sub(mload(0x60), calldataload(pEval90)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval178), sub(mload(0x60), calldataload(pEval88)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval30), mload(add(pTmp, 480)), mload(0x60)), mulmod(calldataload(pEval34), mload(add(pTmp, 480)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval179), sub(mload(0x60), calldataload(pEval89)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval27), mload(add(pTmp, 608)), mload(0x60)), mulmod(calldataload(pEval34), mload(add(pTmp, 608)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval180), sub(mload(0x60), calldataload(pEval90)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval28), mload(add(pTmp, 640)), mload(0x60)), mulmod(calldataload(pEval31), mload(add(pTmp, 640)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 480), addmod(mload(add(pTmp, 576)), sub(mload(0x60), calldataload(pEval91)), mload(0x60)))
                mstore(add(pTmp, 576), addmod(mload(add(pTmp, 576)), sub(mload(0x60), calldataload(pEval92)), mload(0x60)))
                mstore(add(pTmp, 608), addmod(mload(add(pTmp, 512)), sub(mload(0x60), calldataload(pEval93)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval181), sub(mload(0x60), calldataload(pEval91)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval29), mload(add(pTmp, 480)), mload(0x60)), mulmod(calldataload(pEval32), mload(add(pTmp, 480)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval182), sub(mload(0x60), calldataload(pEval92)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval30), mload(add(pTmp, 576)), mload(0x60)), mulmod(calldataload(pEval33), mload(add(pTmp, 576)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval183), sub(mload(0x60), calldataload(pEval93)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval27), mload(add(pTmp, 608)), mload(0x60)), mulmod(calldataload(pEval33), mload(add(pTmp, 608)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 480), addmod(mload(add(pTmp, 512)), sub(mload(0x60), calldataload(pEval94)), mload(0x60)))
                mstore(add(pTmp, 576), addmod(mload(add(pTmp, 512)), sub(mload(0x60), calldataload(pEval95)), mload(0x60)))
                mstore(add(pTmp, 512), addmod(mload(add(pTmp, 512)), sub(mload(0x60), calldataload(pEval96)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval184), sub(mload(0x60), calldataload(pEval94)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval28), mload(add(pTmp, 480)), mload(0x60)), mulmod(calldataload(pEval34), mload(add(pTmp, 480)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval185), sub(mload(0x60), calldataload(pEval95)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval29), mload(add(pTmp, 576)), mload(0x60)), mulmod(calldataload(pEval31), mload(add(pTmp, 576)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval186), sub(mload(0x60), calldataload(pEval96)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval30), mload(add(pTmp, 512)), mload(0x60)), mulmod(calldataload(pEval32), mload(add(pTmp, 512)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 480), addmod(mload(add(pTmp, 544)), sub(mload(0x60), calldataload(pEval97)), mload(0x60)))
                mstore(add(pTmp, 512), addmod(mload(add(pTmp, 544)), sub(mload(0x60), calldataload(pEval98)), mload(0x60)))
                mstore(add(pTmp, 576), addmod(mload(add(pTmp, 544)), sub(mload(0x60), calldataload(pEval99)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval187), sub(mload(0x60), calldataload(pEval97)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval27), mload(add(pTmp, 480)), mload(0x60)), mulmod(calldataload(pEval32), mload(add(pTmp, 480)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval188), sub(mload(0x60), calldataload(pEval98)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval28), mload(add(pTmp, 512)), mload(0x60)), mulmod(calldataload(pEval33), mload(add(pTmp, 512)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval189), sub(mload(0x60), calldataload(pEval99)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval29), mload(add(pTmp, 576)), mload(0x60)), mulmod(calldataload(pEval34), mload(add(pTmp, 576)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 480), addmod(mload(add(pTmp, 544)), sub(mload(0x60), calldataload(pEval100)), mload(0x60)))
                mstore(add(pTmp, 512), addmod(1, sub(mload(0x60), calldataload(pEval69)), mload(0x60)))
                mstore(add(pTmp, 544), mulmod(4294967296, mload(add(pTmp, 448)), mload(0x60)))
                mstore(add(pTmp, 576), addmod(calldataload(pEval67), sub(mload(0x60), mload(add(pTmp, 320))), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval190), sub(mload(0x60), calldataload(pEval100)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval30), mload(add(pTmp, 480)), mload(0x60)), mulmod(calldataload(pEval31), mload(add(pTmp, 480)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval39), addmod(addmod(addmod(mulmod(mload(add(pTmp, 512)), calldataload(pEval67), mload(0x60)), mulmod(calldataload(pEval69), calldataload(pEval68), mload(0x60)), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 320))), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 544))), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval40), addmod(mload(add(pTmp, 576)), sub(mload(0x60), mload(add(pTmp, 544))), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mulmod(calldataload(pEval41), mload(add(pTmp, 576)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval41), addmod(calldataload(pEval68), sub(mload(0x60), mload(add(pTmp, 448))), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(addmod(calldataload(pEval39), calldataload(pEval40), mload(0x60)), mulmod(mload(add(pTmp, 320)), addmod(mulmod(addmod(mload(add(pTmp, 448)), sub(mload(0x60), 4294967295), mload(0x60)), calldataload(pEval153), mload(0x60)), sub(mload(0x60), 1), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mulmod(calldataload(pEval36), calldataload(pEval69), mload(0x60)), mload(add(pTmp, 512)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 160), addmod(calldataload(pEval36), calldataload(pEval38), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval85), sub(mload(0x60), 1779033703), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval37), addmod(calldataload(pEval85), sub(mload(0x60), calldataload(pEval70)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval86), sub(mload(0x60), 3144134277), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval37), addmod(calldataload(pEval86), sub(mload(0x60), calldataload(pEval71)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval87), sub(mload(0x60), 1013904242), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval37), addmod(calldataload(pEval87), sub(mload(0x60), calldataload(pEval72)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval88), sub(mload(0x60), 2773480762), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval37), addmod(calldataload(pEval88), sub(mload(0x60), calldataload(pEval73)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval89), sub(mload(0x60), 1359893119), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval37), addmod(calldataload(pEval89), sub(mload(0x60), calldataload(pEval74)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval90), sub(mload(0x60), 2600822924), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval37), addmod(calldataload(pEval90), sub(mload(0x60), calldataload(pEval75)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval91), sub(mload(0x60), 528734635), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval37), addmod(calldataload(pEval91), sub(mload(0x60), calldataload(pEval76)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval92), sub(mload(0x60), 1541459225), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval37), addmod(calldataload(pEval92), sub(mload(0x60), calldataload(pEval77)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 160), addmod(addmod(calldataload(pEval36), calldataload(pEval37), mload(0x60)), calldataload(pEval38), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval93), sub(mload(0x60), 1779033703), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval94), sub(mload(0x60), 3144134277), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval95), sub(mload(0x60), 1013904242), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval96), sub(mload(0x60), 2773480762), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mulmod(mload(add(pTmp, 160)), calldataload(pEval98), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval100), sub(mload(0x60), calldataload(pEval43)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval36), calldataload(pEval97), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval36), addmod(calldataload(pEval99), sub(mload(0x60), 64), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 160), addmod(calldataload(pEval37), calldataload(pEval38), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval97), sub(mload(0x60), calldataload(pEval79)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 160)), addmod(calldataload(pEval99), sub(mload(0x60), calldataload(pEval78)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval42), calldataload(pEval109), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval42), calldataload(pEval110), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 160), addmod(addmod(addmod(addmod(addmod(addmod(addmod(calldataload(pEval47), calldataload(pEval48), mload(0x60)), calldataload(pEval49), mload(0x60)), calldataload(pEval50), mload(0x60)), calldataload(pEval51), mload(0x60)), calldataload(pEval52), mload(0x60)), calldataload(pEval53), mload(0x60)), calldataload(pEval54), mload(0x60)))
                mstore(add(pTmp, 128), addmod(mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 128)), mload(0x60)), sub(mload(0x60), addmod(addmod(addmod(addmod(addmod(addmod(addmod(mulmod(calldataload(pEval47), calldataload(pEval85), mload(0x60)), mulmod(calldataload(pEval48), calldataload(pEval87), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval49), calldataload(pEval89), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval50), calldataload(pEval91), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval51), calldataload(pEval93), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval52), calldataload(pEval95), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval53), calldataload(pEval97), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval54), calldataload(pEval99), mload(0x60)), mload(0x60))), mload(0x60)))
                mstore(add(pTmp, 96), addmod(mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 96)), mload(0x60)), sub(mload(0x60), addmod(addmod(addmod(addmod(addmod(addmod(addmod(mulmod(calldataload(pEval47), calldataload(pEval86), mload(0x60)), mulmod(calldataload(pEval48), calldataload(pEval88), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval49), calldataload(pEval90), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval50), calldataload(pEval92), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval51), calldataload(pEval94), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval52), calldataload(pEval96), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval53), calldataload(pEval98), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval54), calldataload(pEval100), mload(0x60)), mload(0x60))), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mulmod(calldataload(pEval42), calldataload(pEval111), mload(0x60)), mload(0x60)), mload(0x60)), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 224)), mload(0x60)), sub(mload(0x60), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval47), calldataload(pEval93), mload(0x60)), mulmod(calldataload(pEval48), calldataload(pEval95), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval49), calldataload(pEval97), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval50), calldataload(pEval99), mload(0x60)), mload(0x60)), addmod(mulmod(calldataload(pEval56), calldataload(pEval69), mload(0x60)), calldataload(pEval57), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), mload(add(pTmp, 96)), mload(0x60)))
                mstore(add(pTmp, 96), addmod(mload(add(pTmp, 160)), sub(mload(0x60), calldataload(pEval55)), mload(0x60)))
                mstore(add(pTmp, 128), addmod(calldataload(pEval67), sub(mload(0x60), mload(add(pTmp, 288))), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), addmod(mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 64)), mload(0x60)), sub(mload(0x60), addmod(addmod(addmod(addmod(mulmod(calldataload(pEval47), calldataload(pEval94), mload(0x60)), mulmod(calldataload(pEval48), calldataload(pEval96), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval49), calldataload(pEval98), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval50), calldataload(pEval100), mload(0x60)), mload(0x60)), addmod(mulmod(calldataload(pEval56), calldataload(pEval70), mload(0x60)), calldataload(pEval58), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 96)), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), mulmod(calldataload(pEval55), calldataload(pEval68), mload(0x60)))
                mstore(add(pTmp, 224), addmod(1, sub(mload(0x60), calldataload(pEval68)), mload(0x60)))
                mstore(add(pTmp, 480), addmod(mload(add(pTmp, 192)), sub(mload(0x60), 4294967295), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mulmod(mload(add(pTmp, 96)), addmod(calldataload(pEval68), sub(mload(0x60), mload(add(pTmp, 192))), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval55), addmod(addmod(mload(add(pTmp, 128)), sub(mload(0x60), mulmod(4294967296, mload(add(pTmp, 192)), mload(0x60))), mload(0x60)), mulmod(calldataload(pEval68), 18446744069414584321, mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 480)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mulmod(mload(add(pTmp, 64)), addmod(mulmod(mload(add(pTmp, 288)), calldataload(pEval153), mload(0x60)), sub(mload(0x60), 1), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(mulmod(calldataload(pEval55), mload(add(pTmp, 224)), mload(0x60)), mulmod(mload(add(pTmp, 288)), addmod(mulmod(mload(add(pTmp, 480)), calldataload(pEval153), mload(0x60)), sub(mload(0x60), 1), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval64), addmod(calldataload(pEval67), sub(mload(0x60), calldataload(pPublic)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(calldataload(pEval71), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), addmod(addmod(mulmod(calldataload(pEval72), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 128), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 96)), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(mulmod(calldataload(pEval74), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mload(add(pTmp, 128)), mload(add(pTmp, 192)), mload(0x60)))
                mstore(add(pTmp, 288), addmod(addmod(mulmod(calldataload(pEval75), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 480), addmod(0, sub(mload(0x60), calldataload(pEval26)), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval157), mulmod(mload(add(pTmp, 224)), mload(add(pTmp, 288)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 96)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 64)), mload(0x60)), mload(0x60)), mload(add(pTmp, 192)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)), mload(add(pTmp, 288)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(calldataload(pEval76), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), addmod(addmod(mulmod(calldataload(pEval77), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 128), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 96)), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(mulmod(calldataload(pEval78), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mload(add(pTmp, 128)), mload(add(pTmp, 192)), mload(0x60)))
                mstore(add(pTmp, 288), addmod(addmod(mulmod(calldataload(pEval80), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval158), mulmod(mload(add(pTmp, 224)), mload(add(pTmp, 288)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 96)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 64)), mload(0x60)), mload(0x60)), mload(add(pTmp, 192)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)), mload(add(pTmp, 288)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(calldataload(pEval81), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), addmod(addmod(mulmod(calldataload(pEval82), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 128), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 96)), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(mulmod(calldataload(pEval83), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mload(add(pTmp, 128)), mload(add(pTmp, 192)), mload(0x60)))
                mstore(add(pTmp, 288), addmod(addmod(mulmod(calldataload(pEval84), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval159), mulmod(mload(add(pTmp, 224)), mload(add(pTmp, 288)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 96)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 64)), mload(0x60)), mload(0x60)), mload(add(pTmp, 192)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)), mload(add(pTmp, 288)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(calldataload(pEval101), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), addmod(addmod(mulmod(calldataload(pEval112), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 128), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 96)), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(mulmod(calldataload(pEval114), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mload(add(pTmp, 128)), mload(add(pTmp, 192)), mload(0x60)))
                mstore(add(pTmp, 288), addmod(addmod(mulmod(calldataload(pEval102), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 32), addmod(0, sub(mload(0x60), mload(add(pTmp, 32))), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval160), mulmod(mload(add(pTmp, 224)), mload(add(pTmp, 288)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 96)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 64)), mload(0x60)), mload(0x60)), mload(add(pTmp, 192)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)), mload(add(pTmp, 288)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(calldataload(pEval113), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), addmod(addmod(mulmod(calldataload(pEval115), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 128), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 96)), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval122), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval116), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval108), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mload(add(pTmp, 128)), mload(add(pTmp, 192)), mload(0x60)))
                mstore(add(pTmp, 288), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval123), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval117), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval109), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval161), mulmod(mload(add(pTmp, 224)), mload(add(pTmp, 288)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 96)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 64)), mload(0x60)), mload(0x60)), mload(add(pTmp, 192)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)), mload(add(pTmp, 288)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval120), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval118), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval110), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval121), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval119), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval111), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 128), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 96)), mload(0x60)))
                mstore(add(pTmp, 192), mulmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(calldataload(pEval129), mload(pCh2_0), mload(0x60)), calldataload(pEval128), mload(0x60)), mload(pCh2_0), mload(0x60)), 12, mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval124), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval104), mload(0x60)), mload(pCh2_0), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(mload(add(pTmp, 192)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mload(add(pTmp, 128)), mload(add(pTmp, 192)), mload(0x60)))
                mstore(add(pTmp, 288), mulmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(calldataload(pEval131), mload(pCh2_0), mload(0x60)), calldataload(pEval130), mload(0x60)), mload(pCh2_0), mload(0x60)), 12, mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval125), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval105), mload(0x60)), mload(pCh2_0), mload(0x60)))
                mstore(add(pTmp, 288), addmod(addmod(mload(add(pTmp, 288)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval162), mulmod(mload(add(pTmp, 224)), mload(add(pTmp, 288)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 96)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 64)), mload(0x60)), mload(0x60)), mload(add(pTmp, 192)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)), mload(add(pTmp, 288)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), mulmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(calldataload(pEval133), mload(pCh2_0), mload(0x60)), calldataload(pEval132), mload(0x60)), mload(pCh2_0), mload(0x60)), 12, mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval126), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval106), mload(0x60)), mload(pCh2_0), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mload(add(pTmp, 64)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), mulmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(calldataload(pEval135), mload(pCh2_0), mload(0x60)), calldataload(pEval134), mload(0x60)), mload(pCh2_0), mload(0x60)), 12, mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval127), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval107), mload(0x60)), mload(pCh2_0), mload(0x60)))
                mstore(add(pTmp, 96), addmod(addmod(mload(add(pTmp, 96)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 128), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 96)), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval143), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval136), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval120), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mload(add(pTmp, 128)), mload(add(pTmp, 192)), mload(0x60)))
                mstore(add(pTmp, 288), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval140), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval137), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval121), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval163), mulmod(mload(add(pTmp, 224)), mload(add(pTmp, 288)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 96)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 64)), mload(0x60)), mload(0x60)), mload(add(pTmp, 192)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)), mload(add(pTmp, 288)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval141), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval138), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval122), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval142), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval139), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval123), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 128), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 96)), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval148), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval144), mload(0x60)), mload(pCh2_0), mload(0x60)), mload(add(pTmp, 256)), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mload(add(pTmp, 128)), mload(add(pTmp, 192)), mload(0x60)))
                mstore(add(pTmp, 256), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval149), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval145), mload(0x60)), mload(pCh2_0), mload(0x60)), mload(add(pTmp, 352)), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval164), mulmod(mload(add(pTmp, 224)), mload(add(pTmp, 256)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 96)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 64)), mload(0x60)), mload(0x60)), mload(add(pTmp, 192)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)), mload(add(pTmp, 256)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval150), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval146), mload(0x60)), mload(pCh2_0), mload(0x60)), mload(add(pTmp, 384)), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval151), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval147), mload(0x60)), mload(pCh2_0), mload(0x60)), mload(add(pTmp, 416)), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 128), mulmod(mload(add(pTmp, 64)), mload(add(pTmp, 96)), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 320)), mload(pCh2_0), mload(0x60)), calldataload(pEval44), mload(0x60)), mload(pCh2_0), mload(0x60)), 500, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mload(add(pTmp, 128)), mload(add(pTmp, 192)), mload(0x60)))
                mstore(add(pTmp, 256), addmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 448)), mload(pCh2_0), mload(0x60)), calldataload(pEval45), mload(0x60)), mload(pCh2_0), mload(0x60)), 500, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval165), mulmod(mload(add(pTmp, 224)), mload(add(pTmp, 256)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 96)), mload(0x60)), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 64)), mload(0x60)), mload(0x60)), mload(add(pTmp, 192)), mload(0x60)), mulmod(calldataload(pEval46), mload(add(pTmp, 128)), mload(0x60)), mload(0x60)), mload(add(pTmp, 256)), mload(0x60)), mulmod(calldataload(pEval46), mload(add(pTmp, 224)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 32), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval116), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval108), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval104), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval136), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval124), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval120), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 64)), mload(0x60)))
                mstore(add(pTmp, 128), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval117), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval109), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval105), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 192), mulmod(mload(add(pTmp, 96)), mload(add(pTmp, 128)), mload(0x60)))
                mstore(add(pTmp, 224), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval137), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval125), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval121), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 160), addmod(0, sub(mload(0x60), mload(add(pTmp, 160))), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval166), mulmod(mload(add(pTmp, 192)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 64)), mload(0x60)), mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 32)), mload(0x60)), mload(0x60)), mload(add(pTmp, 128)), mload(0x60)), mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 96)), mload(0x60)), mload(0x60)), mload(add(pTmp, 224)), mload(0x60)), mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 192)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 32), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval118), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval110), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval106), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval138), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval126), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval122), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 64)), mload(0x60)))
                mstore(add(pTmp, 128), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval119), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval111), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval107), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 192), mulmod(mload(add(pTmp, 96)), mload(add(pTmp, 128)), mload(0x60)))
                mstore(add(pTmp, 224), addmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(mulmod(calldataload(pEval139), mload(pCh2_0), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval127), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval123), mload(0x60)), mload(pCh2_0), mload(0x60)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval167), mulmod(mload(add(pTmp, 192)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(addmod(mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 64)), mload(0x60)), mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 32)), mload(0x60)), mload(0x60)), mload(add(pTmp, 128)), mload(0x60)), mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 96)), mload(0x60)), mload(0x60)), mload(add(pTmp, 224)), mload(0x60)), mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 192)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 32), mulmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(addmod(mulmod(calldataload(pEval63), mload(pCh2_0), mload(0x60)), calldataload(pEval62), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval61), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval60), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval59), mload(0x60)), mload(pCh2_0), mload(0x60)))
                mstore(add(pTmp, 32), addmod(addmod(mload(add(pTmp, 32)), 1000, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 64), addmod(addmod(mulmod(addmod(calldataload(pEval59), mulmod(256, calldataload(pEval60), mload(0x60)), mload(0x60)), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 96), addmod(1, sub(mload(0x60), calldataload(pEval66)), mload(0x60)))
                mstore(add(pTmp, 128), addmod(addmod(addmod(addmod(addmod(addmod(addmod(addmod(addmod(calldataload(pEval157), calldataload(pEval158), mload(0x60)), calldataload(pEval159), mload(0x60)), calldataload(pEval160), mload(0x60)), calldataload(pEval161), mload(0x60)), calldataload(pEval162), mload(0x60)), calldataload(pEval163), mload(0x60)), calldataload(pEval164), mload(0x60)), calldataload(pEval165), mload(0x60)), calldataload(pEval166), mload(0x60)))
                mstore(add(pTmp, 160), addmod(addmod(mulmod(calldataload(pEval68), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 192), addmod(addmod(mulmod(calldataload(pEval69), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mload(add(pTmp, 160)), mload(add(pTmp, 192)), mload(0x60)))
                mstore(add(pTmp, 256), addmod(addmod(mulmod(calldataload(pEval70), mload(pCh2_0), mload(0x60)), 250, mload(0x60)), mload(pCh2_1), mload(0x60)))
                mstore(add(pTmp, 0), mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval168), mulmod(mload(add(pTmp, 32)), mload(add(pTmp, 64)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(calldataload(pEval154), mload(add(pTmp, 64)), mload(0x60)), mulmod(calldataload(pEval155), mload(add(pTmp, 32)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(addmod(addmod(calldataload(pEval156), sub(mload(0x60), mulmod(calldataload(pEval0), mload(add(pTmp, 96)), mload(0x60))), mload(0x60)), sub(mload(0x60), addmod(addmod(mload(add(pTmp, 128)), calldataload(pEval167), mload(0x60)), calldataload(pEval168), mload(0x60))), mload(0x60)), mulmod(mload(add(pTmp, 224)), mload(add(pTmp, 256)), mload(0x60)), mload(0x60)), sub(mload(0x60), addmod(mulmod(addmod(mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 192)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 160)), mload(0x60)), mload(0x60)), mload(add(pTmp, 256)), mload(0x60)), mulmod(mload(add(pTmp, 480)), mload(add(pTmp, 224)), mload(0x60)), mload(0x60))), mload(0x60)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval174), addmod(0, sub(mload(0x60), calldataload(pEval156)), mload(0x60)), mload(0x60)), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 32), mulmod(mulmod(mulmod(addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval65), mload(pCh2_0), mload(0x60)), calldataload(pEval67), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(5266228460530200451425464971825753823072228272503274930591399474110020095489, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval68), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(7105417016332174850410570896152637638969286250121046049601916147253836355616, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval69), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(11368207234942670888835120453873516417666490693300823184588242808005266577922, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval70), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 64), mulmod(mulmod(mulmod(mulmod(addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval2), mload(pCh2_0), mload(0x60)), calldataload(pEval67), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval3), mload(pCh2_0), mload(0x60)), calldataload(pEval68), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval4), mload(pCh2_0), mload(0x60)), calldataload(pEval69), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval5), mload(pCh2_0), mload(0x60)), calldataload(pEval70), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval6), mload(pCh2_0), mload(0x60)), calldataload(pEval71), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 128), mulmod(mulmod(mulmod(addmod(addmod(mulmod(addmod(mulmod(mulmod(20868252896871137417296149845131845111459623412721253026236963874583831508289, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval71), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(228143782555411425321250818103251186713928635596850332649653733142161217259, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval72), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(10945236620154748055396050216833787909156199273451639663503130598895656939410, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval73), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(20116697486602678194958186545291394969653616150754282372500669671401371518655, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval74), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 160), mulmod(mulmod(mulmod(mulmod(calldataload(pEval170), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval7), mload(pCh2_0), mload(0x60)), calldataload(pEval72), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval8), mload(pCh2_0), mload(0x60)), calldataload(pEval73), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval9), mload(pCh2_0), mload(0x60)), calldataload(pEval74), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval10), mload(pCh2_0), mload(0x60)), calldataload(pEval75), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 192), mulmod(mulmod(mulmod(addmod(addmod(mulmod(addmod(mulmod(mulmod(3183002806311292071170077895443612615705583737297126452770910814205281990567, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval75), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(15992476177350765221967367463173600149218886479408231745218801140759517520498, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval76), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(20637483108438886850456811366326190112819920572148466038502633469200762337133, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval77), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(20290659656551426187223419893534405802426376066900122212344084778055618042344, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval78), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 224), mulmod(mulmod(mulmod(mulmod(calldataload(pEval171), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval11), mload(pCh2_0), mload(0x60)), calldataload(pEval76), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval12), mload(pCh2_0), mload(0x60)), calldataload(pEval77), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval13), mload(pCh2_0), mload(0x60)), calldataload(pEval78), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval14), mload(pCh2_0), mload(0x60)), calldataload(pEval79), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 256), mulmod(mulmod(mulmod(addmod(addmod(mulmod(addmod(mulmod(mulmod(10264829693514538752375104461414844078286386461657077025033696073485178035040, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval79), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(6295789257344313844297679290172655198753598683555581837131399416649609541633, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval80), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(756236446887263406500047863942771478815339996418426583701634617446061580593, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval81), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(4524779024348363369994158049633115033729409861545949847457214394535404728465, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval82), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 288), mulmod(mulmod(mulmod(mulmod(calldataload(pEval172), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval15), mload(pCh2_0), mload(0x60)), calldataload(pEval80), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval16), mload(pCh2_0), mload(0x60)), calldataload(pEval81), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval17), mload(pCh2_0), mload(0x60)), calldataload(pEval82), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval18), mload(pCh2_0), mload(0x60)), calldataload(pEval83), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 0), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), addmod(mload(add(pTmp, 0)), addmod(mulmod(calldataload(pEval170), mload(add(pTmp, 32)), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 64))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(calldataload(pEval171), mload(add(pTmp, 128)), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 160))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(calldataload(pEval172), mload(add(pTmp, 192)), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 224))), mload(0x60)), mload(0x60)), mload(0x60)), addmod(mulmod(calldataload(pEval173), mload(add(pTmp, 256)), mload(0x60)), sub(mload(0x60), mload(add(pTmp, 288))), mload(0x60)), mload(0x60)))
                mstore(add(pTmp, 32), addmod(mulmod(calldataload(pEval169), mulmod(addmod(addmod(mulmod(addmod(mulmod(mulmod(16363836320469239665341658501629121859103620353654992192491614230122740396338, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval83), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), addmod(addmod(mulmod(addmod(mulmod(mulmod(18496817780155866814153371401425551647480190199721553887757238959077140690007, calldataload(pEval65), mload(0x60)), mload(pCh2_0), mload(0x60)), calldataload(pEval84), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), mload(0x60)), sub(mload(0x60), mulmod(addmod(mulmod(calldataload(pEval1), mload(add(pTmp, 96)), mload(0x60)), calldataload(pEval66), mload(0x60)), mulmod(calldataload(pEval173), addmod(addmod(mulmod(addmod(mulmod(calldataload(pEval19), mload(pCh2_0), mload(0x60)), calldataload(pEval84), mload(0x60)), mload(pCh2_0), mload(0x60)), 1, mload(0x60)), mload(pCh2_1), mload(0x60)), mload(0x60)), mload(0x60))), mload(0x60)))
                mstore(pQ, mulmod(addmod(mulmod(mload(pStdVc), addmod(mulmod(mload(pStdVc), mload(add(pTmp, 0)), mload(0x60)), mload(add(pTmp, 32)), mload(0x60)), mload(0x60)), mulmod(calldataload(pEval174), addmod(1, sub(mload(0x60), calldataload(pEval169)), mload(0x60)), mload(0x60)), mload(0x60)), mload(pZi0), mload(0x60)))
            }

            // STEP 6 (verify.js); shplonk.js, computeRoots: the roots T_i of every f_i are, for each
            // offset s, the k-th roots of z_s = xi·w_N^s; the verifier needs only the z_s and y^k
            // (computeInversions, computeR), never the roots.
            function computeZ() {
                let xiSeed := mload(pXiSeed)
                // shplonk.js, checkOpening: xiSeed is not zero.
                if iszero(xiSeed) {
                    fail()
                }
                let xi := mload(pXi)
                let y := mload(pY)
                mstore(pYk1, y)
                mstore(pYk2, powMod(y, 2))
                mstore(pYk16, powMod(y, 16))
                mstore(pYk18, powMod(y, 18))
                mstore(pYk24, powMod(y, 24))
                mstore(pYk26, powMod(y, 26))
                mstore(pYk29, powMod(y, 29))
                // k = 16, offsets [0]
                mstore(pZ0, xi)
                // k = 24, offsets [0]
                mstore(pZ1, xi)
                // k = 1, offsets [0, 1]
                mstore(pZ2, xi)
                mstore(add(pZ2, 32), mulmod(xi, 17220337697351015657950521176323262483320249231368149235373741788599650842711, mload(0x60)))
                // k = 18, offsets [0]
                mstore(pZ3, xi)
                // k = 26, offsets [0]
                mstore(pZ4, xi)
                // k = 29, offsets [0]
                mstore(pZ5, xi)
                // k = 16, offsets [0, 1]
                mstore(pZ6, xi)
                mstore(add(pZ6, 32), mulmod(xi, 17220337697351015657950521176323262483320249231368149235373741788599650842711, mload(0x60)))
                // k = 2, offsets [-1, 0]
                mstore(pZ7, mulmod(xi, 7258560947846415542561780768613294257363507129322102188167496063542901903859, mload(0x60)))
                mstore(add(pZ7, 32), xi)
                // k = 1, offsets [0]
                mstore(pZ8, xi)
            }

            // shplonk.js, isValidInverse: inv·A·B = 1, with invB = inv·A.
            function checkInv(invB, b) {
                if iszero(eq(mulmod(invB, b, mload(0x60)), 1)) {
                    fail()
                }
            }

            // The inverses of the len values from pVals, given acc, the inverse of their product:
            // snarkjs's inverseArray without its check, which computeInversions makes.
            function batchInverse(pVals, len, acc) {
                let pAux := mload(0x40)
                let p := mload(pVals)
                mstore(pAux, p)
                for { let i := 1 } lt(i, len) { i := add(i, 1) } {
                    p := mulmod(p, mload(add(pVals, mul(i, 32))), mload(0x60))
                    mstore(add(pAux, mul(i, 32)), p)
                }
                for { let i := sub(len, 1) } gt(i, 0) { i := sub(i, 1) } {
                    let pV := add(pVals, mul(i, 32))
                    let v := mload(pV)
                    mstore(pV, mulmod(acc, mload(add(pAux, mul(sub(i, 1), 32))), mload(0x60)))
                    acc := mulmod(acc, v, mload(0x60))
                }
                mstore(pVals, acc)
            }

            // STEP 6 (verify.js): shplonk.js, computeZerofiers and computeInverseDenominators, and
            // isValidInverse. The proof's inv inverts A·B: A = prod_i Z_{T_i}(y), i ≥ 1, with
            // Z_T(y) = prod_s (y^k - z_s), and B the product of every f_i's Lagrange denominators,
            // which has the closed form G·xi^E·prod_i Z_{T_i}(y) (solidity.rs, Context::new): the check
            // inv·A·B = 1 is the JS's. Then 1/A = inv·B gives each 1/Z_{T_i}(y) and, for the sets of
            // several offsets, (1/B)·G·prod_i Z_{T_i}(y) = xi^(-E) gives their CRT factors.
            function computeInversions() {
                let d
                // k = 16, offsets [0]
                d := 1
                d := mulmod(d, addmod(mload(pYk16), sub(mload(0x60), mload(pZ0)), mload(0x60)), mload(0x60))
                // shplonk.js, verifyOpening: y is not a root of an f_i (inv·A·B = 1 checks it too).
                if iszero(d) {
                    fail()
                }
                mstore(pZt0, d)
                // k = 24, offsets [0]
                d := 1
                d := mulmod(d, addmod(mload(pYk24), sub(mload(0x60), mload(pZ1)), mload(0x60)), mload(0x60))
                // shplonk.js, verifyOpening: y is not a root of an f_i (inv·A·B = 1 checks it too).
                if iszero(d) {
                    fail()
                }
                mstore(pZt1, d)
                // k = 1, offsets [0, 1]
                d := 1
                d := mulmod(d, addmod(mload(pYk1), sub(mload(0x60), mload(pZ2)), mload(0x60)), mload(0x60))
                d := mulmod(d, addmod(mload(pYk1), sub(mload(0x60), mload(add(pZ2, 32))), mload(0x60)), mload(0x60))
                // shplonk.js, verifyOpening: y is not a root of an f_i (inv·A·B = 1 checks it too).
                if iszero(d) {
                    fail()
                }
                mstore(pZt2, d)
                // k = 18, offsets [0]
                d := 1
                d := mulmod(d, addmod(mload(pYk18), sub(mload(0x60), mload(pZ3)), mload(0x60)), mload(0x60))
                // shplonk.js, verifyOpening: y is not a root of an f_i (inv·A·B = 1 checks it too).
                if iszero(d) {
                    fail()
                }
                mstore(pZt3, d)
                // k = 26, offsets [0]
                d := 1
                d := mulmod(d, addmod(mload(pYk26), sub(mload(0x60), mload(pZ4)), mload(0x60)), mload(0x60))
                // shplonk.js, verifyOpening: y is not a root of an f_i (inv·A·B = 1 checks it too).
                if iszero(d) {
                    fail()
                }
                mstore(pZt4, d)
                // k = 29, offsets [0]
                d := 1
                d := mulmod(d, addmod(mload(pYk29), sub(mload(0x60), mload(pZ5)), mload(0x60)), mload(0x60))
                // shplonk.js, verifyOpening: y is not a root of an f_i (inv·A·B = 1 checks it too).
                if iszero(d) {
                    fail()
                }
                mstore(pZt5, d)
                // k = 16, offsets [0, 1]
                d := 1
                d := mulmod(d, addmod(mload(pYk16), sub(mload(0x60), mload(pZ6)), mload(0x60)), mload(0x60))
                d := mulmod(d, addmod(mload(pYk16), sub(mload(0x60), mload(add(pZ6, 32))), mload(0x60)), mload(0x60))
                // shplonk.js, verifyOpening: y is not a root of an f_i (inv·A·B = 1 checks it too).
                if iszero(d) {
                    fail()
                }
                mstore(pZt6, d)
                // k = 2, offsets [-1, 0]
                d := 1
                d := mulmod(d, addmod(mload(pYk2), sub(mload(0x60), mload(pZ7)), mload(0x60)), mload(0x60))
                d := mulmod(d, addmod(mload(pYk2), sub(mload(0x60), mload(add(pZ7, 32))), mload(0x60)), mload(0x60))
                // shplonk.js, verifyOpening: y is not a root of an f_i (inv·A·B = 1 checks it too).
                if iszero(d) {
                    fail()
                }
                mstore(pZt7, d)
                // k = 1, offsets [0]
                d := 1
                d := mulmod(d, addmod(mload(pYk1), sub(mload(0x60), mload(pZ8)), mload(0x60)), mload(0x60))
                // shplonk.js, verifyOpening: y is not a root of an f_i (inv·A·B = 1 checks it too).
                if iszero(d) {
                    fail()
                }
                mstore(pZt8, d)
                let a := 1
                d := mload(pZt1)
                mstore(pInvZt1, d)
                a := mulmod(a, d, mload(0x60))
                d := mload(pZt1)
                mstore(pInvZt2, d)
                a := mulmod(a, d, mload(0x60))
                d := mload(pZt2)
                mstore(pInvZt3, d)
                a := mulmod(a, d, mload(0x60))
                d := mload(pZt3)
                mstore(pInvZt4, d)
                a := mulmod(a, d, mload(0x60))
                d := mload(pZt4)
                mstore(pInvZt5, d)
                a := mulmod(a, d, mload(0x60))
                d := mload(pZt5)
                mstore(pInvZt6, d)
                a := mulmod(a, d, mload(0x60))
                d := mload(pZt6)
                mstore(pInvZt7, d)
                a := mulmod(a, d, mload(0x60))
                d := mload(pZt7)
                mstore(pInvZt8, d)
                a := mulmod(a, d, mload(0x60))
                d := mload(pZt0)
                mstore(pInvZt9, d)
                a := mulmod(a, d, mload(0x60))
                d := mload(pZt8)
                mstore(pInvZt10, d)
                a := mulmod(a, d, mload(0x60))
                let zts := mulmod(a, mload(pZt0), mload(0x60))
                let b := mulmod(zts, mulmod(16811741955435315087775785781660941032310258619080022454006845281148559853430, powMod(mload(pXi), 216), mload(0x60)), mload(0x60))
                let invB := mulmod(calldataload(pInv), a, mload(0x60))
                checkInv(invB, b)
                batchInverse(pInvZt1, 10, mulmod(calldataload(pInv), b, mload(0x60)))
                let xiNegE := mulmod(invB, mulmod(16811741955435315087775785781660941032310258619080022454006845281148559853430, zts, mload(0x60)), mload(0x60))
                mstore(pXiNeg1, mulmod(xiNegE, powMod(mload(pXi), 215), mload(0x60)))
                // k = 1, offsets [0, 1]: L_s = prod_{s' != s} (y^k - z_s')/(z_s - z_s')
                d := mulmod(18891174137072410644159713240788227550940121292175075153801163539494952511361, mload(pXiNeg1), mload(0x60))
                d := mulmod(d, addmod(mload(pYk1), sub(mload(0x60), mload(add(pZ2, 32))), mload(0x60)), mload(0x60))
                mstore(pL2, d)
                d := mulmod(2997068734766864578086692504469047537608243108240959189897040647080855984256, mload(pXiNeg1), mload(0x60))
                d := mulmod(d, addmod(mload(pYk1), sub(mload(0x60), mload(pZ2)), mload(0x60)), mload(0x60))
                mstore(add(pL2, 32), d)
                // k = 16, offsets [0, 1]: L_s = prod_{s' != s} (y^k - z_s')/(z_s - z_s')
                d := mulmod(18891174137072410644159713240788227550940121292175075153801163539494952511361, mload(pXiNeg1), mload(0x60))
                d := mulmod(d, addmod(mload(pYk16), sub(mload(0x60), mload(add(pZ6, 32))), mload(0x60)), mload(0x60))
                mstore(pL6, d)
                d := mulmod(2997068734766864578086692504469047537608243108240959189897040647080855984256, mload(pXiNeg1), mload(0x60))
                d := mulmod(d, addmod(mload(pYk16), sub(mload(0x60), mload(pZ6)), mload(0x60)), mload(0x60))
                mstore(add(pL6, 32), d)
                // k = 2, offsets [-1, 0]: L_s = prod_{s' != s} (y^k - z_s')/(z_s - z_s')
                d := mulmod(18891174137072410644159713240788227550940121292175075153801163539494952511360, mload(pXiNeg1), mload(0x60))
                d := mulmod(d, addmod(mload(pYk2), sub(mload(0x60), mload(add(pZ7, 32))), mload(0x60)), mload(0x60))
                mstore(pL7, d)
                d := mulmod(2997068734766864578086692504469047537608243108240959189897040647080855984257, mload(pXiNeg1), mload(0x60))
                d := mulmod(d, addmod(mload(pYk2), sub(mload(0x60), mload(pZ7)), mload(0x60)), mload(0x60))
                mstore(add(pL7, 32), d)
            }

            // STEP 7 (verify.js, openingEvaluations; shplonk.js, computeR): r_i(y), the polynomial of
            // degree < k·t that f_i is on its roots, at y. On the roots of an offset s, X^k = z_s and
            // f_i(X) = R_s(X) = sum_j p_j(xi·w^s)·X^j, of degree < k: with one offset r_i = R_s, and
            // with several the CRT, r_i = sum_s R_s·L_s. p_j(xi·w^s) are the proof's evaluations and,
            // for Q's f, Q(xi) computed or the proof's Q_i(xi) if Q is split.
            function computeR() {
                let y := mload(pY)
                let v
                // f0: stage 0, k = 16, offsets [0]
                v := calldataload(pEval50)
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval51), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval52), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval53), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval54), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval55), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval56), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval57), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval58), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval59), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval60), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval61), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval62), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval63), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval64), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval65), mload(0x60))
                mstore(pR0, v)
                // f1: stage 0, k = 24, offsets [0]
                v := calldataload(pEval26)
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval27), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval28), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval29), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval30), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval31), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval32), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval33), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval34), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval35), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval36), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval37), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval38), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval39), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval40), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval41), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval42), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval43), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval44), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval45), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval46), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval47), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval48), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval49), mload(0x60))
                mstore(pR1, v)
                // f2: stage 0, k = 24, offsets [0]
                v := calldataload(pEval2)
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval3), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval4), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval5), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval6), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval7), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval8), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval9), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval10), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval11), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval12), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval13), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval14), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval15), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval16), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval17), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval18), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval19), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval20), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval21), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval22), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval23), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval24), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval25), mload(0x60))
                mstore(pR2, v)
                // f3: stage 0, k = 1, offsets [0, 1]
                {
                    let acc := 0
                    v := calldataload(pEval66)
                    acc := addmod(acc, mulmod(v, mload(pL2), mload(0x60)), mload(0x60))
                    v := calldataload(pEval174)
                    acc := addmod(acc, mulmod(v, mload(add(pL2, 32)), mload(0x60)), mload(0x60))
                    mstore(pR3, acc)
                }
                // f4: stage 1, k = 18, offsets [0]
                v := calldataload(pEval138)
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval139), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval140), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval141), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval142), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval143), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval144), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval145), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval146), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval147), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval148), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval149), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval150), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval151), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval152), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval153), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval154), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval155), mload(0x60))
                mstore(pR4, v)
                // f5: stage 1, k = 26, offsets [0]
                v := calldataload(pEval112)
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval113), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval114), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval115), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval116), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval117), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval118), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval119), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval120), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval121), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval122), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval123), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval124), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval125), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval126), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval127), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval128), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval129), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval130), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval131), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval132), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval133), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval134), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval135), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval136), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval137), mload(0x60))
                mstore(pR5, v)
                // f6: stage 1, k = 29, offsets [0]
                v := calldataload(pEval67)
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval68), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval69), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval70), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval71), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval72), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval73), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval74), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval75), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval76), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval77), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval78), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval79), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval80), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval81), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval82), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval83), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval84), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval101), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval102), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval103), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval104), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval105), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval106), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval107), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval108), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval109), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval110), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval111), mload(0x60))
                mstore(pR6, v)
                // f7: stage 1, k = 16, offsets [0, 1]
                {
                    let acc := 0
                    v := calldataload(pEval85)
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval86), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval87), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval88), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval89), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval90), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval91), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval92), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval93), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval94), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval95), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval96), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval97), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval98), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval99), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval100), mload(0x60))
                    acc := addmod(acc, mulmod(v, mload(pL6), mload(0x60)), mload(0x60))
                    v := calldataload(pEval175)
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval176), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval177), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval178), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval179), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval180), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval181), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval182), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval183), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval184), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval185), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval186), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval187), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval188), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval189), mload(0x60))
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval190), mload(0x60))
                    acc := addmod(acc, mulmod(v, mload(add(pL6, 32)), mload(0x60)), mload(0x60))
                    mstore(pR7, acc)
                }
                // f8: stage 2, k = 2, offsets [-1, 0]
                {
                    let acc := 0
                    v := calldataload(pEval0)
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval1), mload(0x60))
                    acc := addmod(acc, mulmod(v, mload(pL7), mload(0x60)), mload(0x60))
                    v := calldataload(pEval156)
                    v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval169), mload(0x60))
                    acc := addmod(acc, mulmod(v, mload(add(pL7, 32)), mload(0x60)), mload(0x60))
                    mstore(pR8, acc)
                }
                // f9: stage 2, k = 16, offsets [0]
                v := calldataload(pEval157)
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval158), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval159), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval160), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval161), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval162), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval163), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval164), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval165), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval166), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval167), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval168), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval170), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval171), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval172), mload(0x60))
                v := addmod(mulmod(v, y, mload(0x60)), calldataload(pEval173), mload(0x60))
                mstore(pR9, v)
                // f10: stage 3, k = 1, offsets [0]
                v := mload(pQ)
                mstore(pR10, v)
            }

            // Whether a call to the precompile 0x06 or 0x07 succeeded and returned a point: 64 bytes.
            // snarkjs checks success only; on a chain where the precompile is missing, the call
            // succeeds and returns nothing.
            function checkPointResult(success) {
                if iszero(and(success, eq(returndatasize(), 64))) {
                    fail()
                }
            }

            // G1 function to accumulate a G1 value to an address
            function g1_acc(pR, pP) {
                let mIn := mload(0x40)
                mstore(mIn, mload(pR))
                mstore(add(mIn, 32), mload(add(pR, 32)))
                mstore(add(mIn, 64), mload(pP))
                mstore(add(mIn, 96), mload(add(pP, 32)))

                let success := staticcall(sub(gas(), 2000), 6, mIn, 128, pR, 64)

                checkPointResult(success)
            }

            // G1 function to multiply a G1 value (in the calldata at pP) to value in an address
            function g1_mulAcc(pR, pP, s) {
                let success
                let mIn := mload(0x40)
                mstore(mIn, calldataload(pP))
                mstore(add(mIn, 32), calldataload(add(pP, 32)))
                mstore(add(mIn, 64), s)

                success := staticcall(sub(gas(), 2000), 7, mIn, 96, mIn, 64)

                checkPointResult(success)

                mstore(add(mIn, 64), mload(pR))
                mstore(add(mIn, 96), mload(add(pR, 32)))

                success := staticcall(sub(gas(), 2000), 6, mIn, 128, pR, 64)

                checkPointResult(success)
            }

            // G1 function to multiply a G1 value (x, y) to value in an address
            function g1_mulAccC(pR, x, y, s) {
                let success
                let mIn := mload(0x40)
                mstore(mIn, x)
                mstore(add(mIn, 32), y)
                mstore(add(mIn, 64), s)

                success := staticcall(sub(gas(), 2000), 7, mIn, 96, mIn, 64)

                checkPointResult(success)

                mstore(add(mIn, 64), mload(pR))
                mstore(add(mIn, 96), mload(add(pR, 32)))

                success := staticcall(sub(gas(), 2000), 6, mIn, 128, pR, 64)

                checkPointResult(success)
            }

            // G1 function to write s·(x, y) to an address
            function g1_mulSetC(pR, x, y, s) {
                let mIn := mload(0x40)
                mstore(mIn, x)
                mstore(add(mIn, 32), y)
                mstore(add(mIn, 64), s)

                let success := staticcall(sub(gas(), 2000), 7, mIn, 96, pR, 64)

                checkPointResult(success)
            }

            // STEP 7: shplonk.js, computeQuotients, computeF, computeE and computeJ:
            // q_0 = Z_{T_0}(y), q_i = alpha^i·Z_{T_0}(y)/Z_{T_i}(y), F = [f_0] + sum_i q_i·[f_i],
            // E = (r_0(y) + sum_i q_i·r_i(y))·[1]_1, J = q_0·[W]; the fixed [f_i] are the vkey's.
            function computeFEJ() {
                let zt0 := mload(pZt0)
                let alpha := mload(pAlpha)
                let alphaPow := 1
                let quotient
                let e := mload(pR0)
                mstore(pF, f0x)
                mstore(add(pF, 32), f0y)
                // f1
                alphaPow := mulmod(alphaPow, alpha, mload(0x60))
                quotient := mulmod(alphaPow, mulmod(zt0, mload(pInvZt1), mload(0x60)), mload(0x60))
                g1_mulAccC(pF, f1x, f1y, quotient)
                e := addmod(e, mulmod(quotient, mload(pR1), mload(0x60)), mload(0x60))
                // f2
                alphaPow := mulmod(alphaPow, alpha, mload(0x60))
                quotient := mulmod(alphaPow, mulmod(zt0, mload(pInvZt2), mload(0x60)), mload(0x60))
                g1_mulAccC(pF, f2x, f2y, quotient)
                e := addmod(e, mulmod(quotient, mload(pR2), mload(0x60)), mload(0x60))
                // f3
                alphaPow := mulmod(alphaPow, alpha, mload(0x60))
                quotient := mulmod(alphaPow, mulmod(zt0, mload(pInvZt3), mload(0x60)), mload(0x60))
                g1_mulAccC(pF, f3x, f3y, quotient)
                e := addmod(e, mulmod(quotient, mload(pR3), mload(0x60)), mload(0x60))
                // f4
                alphaPow := mulmod(alphaPow, alpha, mload(0x60))
                quotient := mulmod(alphaPow, mulmod(zt0, mload(pInvZt4), mload(0x60)), mload(0x60))
                g1_mulAcc(pF, pF4, quotient)
                e := addmod(e, mulmod(quotient, mload(pR4), mload(0x60)), mload(0x60))
                // f5
                alphaPow := mulmod(alphaPow, alpha, mload(0x60))
                quotient := mulmod(alphaPow, mulmod(zt0, mload(pInvZt5), mload(0x60)), mload(0x60))
                g1_mulAcc(pF, pF5, quotient)
                e := addmod(e, mulmod(quotient, mload(pR5), mload(0x60)), mload(0x60))
                // f6
                alphaPow := mulmod(alphaPow, alpha, mload(0x60))
                quotient := mulmod(alphaPow, mulmod(zt0, mload(pInvZt6), mload(0x60)), mload(0x60))
                g1_mulAcc(pF, pF6, quotient)
                e := addmod(e, mulmod(quotient, mload(pR6), mload(0x60)), mload(0x60))
                // f7
                alphaPow := mulmod(alphaPow, alpha, mload(0x60))
                quotient := mulmod(alphaPow, mulmod(zt0, mload(pInvZt7), mload(0x60)), mload(0x60))
                g1_mulAcc(pF, pF7, quotient)
                e := addmod(e, mulmod(quotient, mload(pR7), mload(0x60)), mload(0x60))
                // f8
                alphaPow := mulmod(alphaPow, alpha, mload(0x60))
                quotient := mulmod(alphaPow, mulmod(zt0, mload(pInvZt8), mload(0x60)), mload(0x60))
                g1_mulAcc(pF, pF8, quotient)
                e := addmod(e, mulmod(quotient, mload(pR8), mload(0x60)), mload(0x60))
                // f9
                alphaPow := mulmod(alphaPow, alpha, mload(0x60))
                quotient := mulmod(alphaPow, mulmod(zt0, mload(pInvZt9), mload(0x60)), mload(0x60))
                g1_mulAcc(pF, pF9, quotient)
                e := addmod(e, mulmod(quotient, mload(pR9), mload(0x60)), mload(0x60))
                // f10
                alphaPow := mulmod(alphaPow, alpha, mload(0x60))
                quotient := mulmod(alphaPow, mulmod(zt0, mload(pInvZt10), mload(0x60)), mload(0x60))
                g1_mulAcc(pF, pF10, quotient)
                e := addmod(e, mulmod(quotient, mload(pR10), mload(0x60)), mload(0x60))

                // Compute group-encoded batch evaluation [E]_1
                g1_mulSetC(pE, G1x, G1y, e)

                // Compute the full difference [J]_1
                g1_mulSetC(pJ, calldataload(pW), calldataload(add(pW, 32)), zt0)
            }

            // STEP 7: shplonk.js, isValidPairing: e([F]_1 - [E]_1 - [J]_1 + y·[W']_1, [1]_2) ==
            // e([W']_1, [x]_2)
            function checkPairing() -> isOk {
                let mIn := mload(0x40)

                // First pairing value
                // Compute -E
                mstore(add(pE, 32), mod(sub(qf, mload(add(pE, 32))), qf))
                // Compute -J
                mstore(add(pJ, 32), mod(sub(qf, mload(add(pJ, 32))), qf))
                // F = F - E - J + y·W'
                g1_acc(pF, pE)
                g1_acc(pF, pJ)
                g1_mulAcc(pF, pWp, mload(pY))

                mstore(mIn, mload(pF))
                mstore(add(mIn, 32), mload(add(pF, 32)))

                // Second pairing value
                mstore(add(mIn, 64), G2x2)
                mstore(add(mIn, 96), G2x1)
                mstore(add(mIn, 128), G2y2)
                mstore(add(mIn, 160), G2y1)

                // Third pairing value
                // Compute -W'
                mstore(add(mIn, 192), calldataload(pWp))
                let s := calldataload(add(pWp, 32))
                s := mod(sub(qf, s), qf)
                mstore(add(mIn, 224), s)

                // Fourth pairing value
                mstore(add(mIn, 256), X2x2)
                mstore(add(mIn, 288), X2x1)
                mstore(add(mIn, 320), X2y2)
                mstore(add(mIn, 352), X2y1)

                let success := staticcall(sub(gas(), 2000), 8, mIn, 384, mIn, 0x20)

                // The precompile's answer, 32 bytes, is 1 (snarkjs takes any non-zero word).
                isOk := and(and(success, eq(returndatasize(), 0x20)), eq(mload(mIn), 1))
            }

            mstore(0x40, lastMem)

            // STEPS 1-3: the proof and the publics are well-formed
            checkInput()

            // STEP 4: the challenges
            computeChallenges(6756)

            // STEP 5: Z_H(xi), the Zi and Q(xi)
            computeZh()
            computeZi()
            computeQ(6756)

            // STEP 6: the z_s, and inv
            computeZ()
            computeInversions()

            // STEP 7: the SHPLONK opening, with a pairing
            computeR()
            computeFEJ()
            let isValid := checkPairing()

            mstore(0, isValid)
            return(0, 0x20)
        }
    }
}
