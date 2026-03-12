use precompiles_helpers::DmaInfo;

pub enum DmaPrePostRom {}

impl DmaPrePostRom {
    // Table generated from pil
    const TABLE_OFFSETS: [usize; 64] = [
        0, 32, 64, 96, 128, 160, 192, 224, 256, 284, 312, 340, 368, 396, 424, 452, 480, 504, 528,
        552, 576, 600, 624, 648, 672, 692, 712, 732, 752, 772, 792, 812, 832, 848, 864, 880, 896,
        912, 928, 944, 960, 972, 984, 996, 1008, 1020, 1032, 1044, 1056, 1064, 1072, 1080, 1088,
        1096, 1104, 1112, 1120, 1124, 1128, 1132, 1136, 1140, 1144, 1148,
    ];

    #[allow(dead_code)]
    pub fn get_row_from_encoded(
        encoded: u64,
        memcmp_result_nz: bool,
        memcmp_result_is_neg: bool,
        load_src: bool,
    ) -> usize {
        let src_offset = DmaInfo::get_src_offset(encoded);
        let dst_offset = DmaInfo::get_dst_offset(encoded);
        let count = DmaInfo::get_count(encoded);
        Self::get_row(
            dst_offset,
            src_offset,
            count,
            memcmp_result_nz,
            memcmp_result_is_neg,
            load_src,
        )
    }
    pub fn get_row(
        dst_offset: usize,
        src_offset: usize,
        count: usize,
        memcmp_result_nz: bool,
        memcmp_result_is_neg: bool,
        load_src: bool,
    ) -> usize {
        debug_assert!(!memcmp_result_is_neg || memcmp_result_nz);
        debug_assert!(load_src || (!memcmp_result_is_neg && !memcmp_result_nz));
        Self::TABLE_OFFSETS[dst_offset * 8 + src_offset]
            + (count - 1) * 4
            + if load_src { memcmp_result_is_neg as usize + memcmp_result_nz as usize } else { 3 }
    }
}
