use ziskos::zisklib::{
    add_twist_bls12_381, dbl_twist_bls12_381, is_on_curve_twist_bls12_381,
    is_on_subgroup_twist_bls12_381, neg_twist_bls12_381, scalar_mul_by_abs_x_twist_bls12_381,
};

use crate::constants::{G2, IDENTITY_G2};

pub fn twist_tests() {
    // Is on curve
    let p = IDENTITY_G2;
    let res = is_on_curve_twist_bls12_381(&p);
    assert_eq!(res, false);

    let p = [
        0x749452F6EA61448C,
        0x139A683D6C00B976,
        0x7ADE93A8282BAC81,
        0x1C58BA110FF338E7,
        0x251D40272ED844C7,
        0x4A1556AAAE189D2,
        0xB11CA9CCCB2CEDD6,
        0xF5A2DF75D69FEF21,
        0x53433D67918E397C,
        0xCE5C5DD68CB1FD58,
        0x303158F985FEB973,
        0x87A96163599DECF,
        0xA29ED42DA3F7AEAE,
        0xD20C801C77DF0DA8,
        0xBD56BCC1340191D2,
        0x8062D9139EF0F282,
        0x184BD175E1B3A4E5,
        0x7EECBE5774E3AB4,
        0x7CDBC7B998D523CB,
        0xEF5807F1DC96E725,
        0x258E5A75FC4F6F49,
        0x7926196E4996119C,
        0x8F134799C4B04ED6,
        0x18ABCDAF6A74EDB8,
    ];
    let res = is_on_curve_twist_bls12_381(&p);
    assert_eq!(res, true);

    // Is on subgroup
    let p = G2;
    let res = is_on_subgroup_twist_bls12_381(&p);
    assert_eq!(res, true);

    // Addition
    // (same point)
    let p1 = G2;
    let p2 = p1;
    let res = add_twist_bls12_381(&p1, &p2);
    let res_exp = [
        0xC952AACAB827A053,
        0x81F14B0BF3611B78,
        0xE1EA1E1E4D00DBAE,
        0x3BC0B995B8825E0E,
        0xD2370F17CC7ED586,
        0x1638533957D540A9,
        0x6178288C47C33577,
        0xC6C886F6B57EC72A,
        0x728114D1031E1572,
        0xD70662A904BA1074,
        0x9F520E47730A124F,
        0xA4EDEF9C1ED7F72,
        0x999D95D71E4C9899,
        0xE88DECE9764BF3BD,
        0xBFE6BD221E47AA8A,
        0x9A66DA69BF91009C,
        0xAEB8DCA2B525678,
        0x468FB440D82B063,
        0xACDEFD8B6E36CCF3,
        0x422E1AA0A59C8967,
        0x97003F7A13C308F5,
        0xA43253D9C66C4116,
        0x38B361543F887136,
        0xF6D4552FA65DD26,
    ];
    assert_eq!(res, res_exp);

    // (inverses of each other)
    let p1 = G2;
    let p2 = [
        0xD48056C8C121BDB8,
        0xBAC0326A805BBEF,
        0xB4510B647AE3D177,
        0xC6E47AD4FA403B02,
        0x260805272DC51051,
        0x24AA2B2F08F0A91,
        0xE5AC7D055D042B7E,
        0x334CF11213945D57,
        0xB5DA61BBDC7F5049,
        0x596BD0D09920B61A,
        0x7DACD3A088274F65,
        0x13E02B6052719F60,
        0xD86BAB79F74782AA,
        0x8C71363275A75D75,
        0xF9EE3837A55024F7,
        0xB679AFDA66C73F17,
        0xBE51D9EF691D77BC,
        0xD1B3CC2C7027888,
        0xF55F8A00FA030ED,
        0xDF74F2D75467E25E,
        0x40BC3FF59F825C78,
        0x993923066DDDAF10,
        0x186ED5061789213D,
        0x13FA4D4A0AD8B1CE,
    ];
    let res = add_twist_bls12_381(&p1, &p2);
    let res_exp = IDENTITY_G2;
    assert_eq!(res, res_exp);

    // (different)
    let p1 = G2;
    let p2 = [
        0xC952AACAB827A053,
        0x81F14B0BF3611B78,
        0xE1EA1E1E4D00DBAE,
        0x3BC0B995B8825E0E,
        0xD2370F17CC7ED586,
        0x1638533957D540A9,
        0x6178288C47C33577,
        0xC6C886F6B57EC72A,
        0x728114D1031E1572,
        0xD70662A904BA1074,
        0x9F520E47730A124F,
        0xA4EDEF9C1ED7F72,
        0x999D95D71E4C9899,
        0xE88DECE9764BF3BD,
        0xBFE6BD221E47AA8A,
        0x9A66DA69BF91009C,
        0xAEB8DCA2B525678,
        0x468FB440D82B063,
        0xACDEFD8B6E36CCF3,
        0x422E1AA0A59C8967,
        0x97003F7A13C308F5,
        0xA43253D9C66C4116,
        0x38B361543F887136,
        0xF6D4552FA65DD26,
    ];
    let res = add_twist_bls12_381(&p1, &p2);
    let res_exp = [
        0x16020EF82324AFAE,
        0x50A030FC866F09D5,
        0xA0C75DF1C04D6D7A,
        0x691AE54329781315,
        0x2EE414A3DCCB23AE,
        0x122915C824A0857E,
        0xD6A44AAA56CA66DC,
        0xEB480673937CC6D9,
        0x5062650F8D251C96,
        0x2AC480905396EDA5,
        0xEA7DC4DD7E0550FF,
        0x9380275BBC8E5DC,
        0x455E44813ECFD892,
        0x479DFD948B52FDF2,
        0x326AC738FEF5C721,
        0x36961D1E3B20B1A7,
        0x10C7A1ABC1A6F01,
        0xB21DA7955969E61,
        0xEA56D53F23A0E849,
        0xCF6B3B58B975B9ED,
        0x714150A166BFBD6B,
        0x62A7E42E0BF1C1ED,
        0xFE48D718A36CFE5F,
        0x8F239BA329B3967,
    ];
    assert_eq!(res, res_exp);

    // Doubling
    let p = G2;
    let res = dbl_twist_bls12_381(&p);
    let res_exp = [
        0xC952AACAB827A053,
        0x81F14B0BF3611B78,
        0xE1EA1E1E4D00DBAE,
        0x3BC0B995B8825E0E,
        0xD2370F17CC7ED586,
        0x1638533957D540A9,
        0x6178288C47C33577,
        0xC6C886F6B57EC72A,
        0x728114D1031E1572,
        0xD70662A904BA1074,
        0x9F520E47730A124F,
        0xA4EDEF9C1ED7F72,
        0x999D95D71E4C9899,
        0xE88DECE9764BF3BD,
        0xBFE6BD221E47AA8A,
        0x9A66DA69BF91009C,
        0xAEB8DCA2B525678,
        0x468FB440D82B063,
        0xACDEFD8B6E36CCF3,
        0x422E1AA0A59C8967,
        0x97003F7A13C308F5,
        0xA43253D9C66C4116,
        0x38B361543F887136,
        0xF6D4552FA65DD26,
    ];
    assert_eq!(res, res_exp);

    // Negation
    let p = G2;
    let res = neg_twist_bls12_381(&p);
    let res_exp = [
        0xD48056C8C121BDB8,
        0xBAC0326A805BBEF,
        0xB4510B647AE3D177,
        0xC6E47AD4FA403B02,
        0x260805272DC51051,
        0x24AA2B2F08F0A91,
        0xE5AC7D055D042B7E,
        0x334CF11213945D57,
        0xB5DA61BBDC7F5049,
        0x596BD0D09920B61A,
        0x7DACD3A088274F65,
        0x13E02B6052719F60,
        0xD86BAB79F74782AA,
        0x8C71363275A75D75,
        0xF9EE3837A55024F7,
        0xB679AFDA66C73F17,
        0xBE51D9EF691D77BC,
        0xD1B3CC2C7027888,
        0xF55F8A00FA030ED,
        0xDF74F2D75467E25E,
        0x40BC3FF59F825C78,
        0x993923066DDDAF10,
        0x186ED5061789213D,
        0x13FA4D4A0AD8B1CE,
    ];
    assert_eq!(res, res_exp);

    // Scalar multiplication by x
    let p = G2;
    let res = scalar_mul_by_abs_x_twist_bls12_381(&p);
    let res_exp = [
        0xD928C716F9C1BC67,
        0x4A85F5B285F02508,
        0x171FD41F31B7EC5C,
        0xC41C801F2EB23FD9,
        0xC86F8673946935A6,
        0x149EE6D25A1C8648,
        0xFD2B694040D19479,
        0xB18D535B71CF70DA,
        0x2AA7CF28B80AA054,
        0xAB79697E45ABF504,
        0xBA6F6E15257A5CD4,
        0x6E63A1561FA8259,
        0x91AC308465A849C6,
        0x947457947CD89177,
        0x2083DF7086215506,
        0x6F664B92422160A5,
        0xED89A6B0FF12B4DA,
        0xFCF61319E51E41E,
        0x8EA0ECFDC3DE9B2F,
        0x1A3D8B6A55E5881F,
        0x6DF99C2649D3F49B,
        0x8A49E53D8EF016C7,
        0xCB1EB0DFBB4910E9,
        0x14C164377443F5AC,
    ];
    assert_eq!(res, res_exp);
}
