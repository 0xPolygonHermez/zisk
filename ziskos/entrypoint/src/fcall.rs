#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

// fcall_get 0xFFE

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub fn ziskos_fcall_get() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, 0xFFE", out(reg) value);
    }
    value
}

#[macro_export]
macro_rules! ziskos_fcall_param {
    ( $addr:expr, $words:literal) => {{
        // Calcula l'índex basat en log2 del nombre de words
        const fn words_to_port(words: usize) -> usize {
            match words {
                1 => 0,     /* direct value */
                2 => 1,     /* 2 x 8 = 16 */
                4 => 2,     /* 4 x 8 = 32 */
                8 => 3,     /* 8 x 8 = 64 */
                12 => 4,    /* 12 x 8 = 96 */
                16 => 5,    /* 16 x 8 = 128 */
                20 => 6,    /* 20 x 8 = 160 */
                24 => 7,    /* 24 x 8 = 192 */
                28 => 8,    /* 28 x 8 = 224 */
                32 => 9,    /* 32 x 8 = 256 */
                48 => 10,   /* 48 x 8 = 384 */
                64 => 11,   /* 64 x 8 = 512 */
                80 => 12,   /* 80 x 8 = 640 */
                96 => 13,   /* 256 x 8 = 2048 */
                128 => 14,  /* 128 x 8 = 1024 */
                256 => 15,  /* 256 x 8 = 2048 */
                _ => panic!("number of words no supported, must be 2, 4, 8, 12, 16, 20, 24, 28, 32, 48, 64, 80, 96, 128 or 256"),
            }
        }

        unsafe {
            asm!(
                "csrs {port}, {value}",
                port = const 0x8F0 + words_to_port($words),
                value = in(reg) $addr
            );
        }
    }};
}

#[macro_export]
macro_rules! ziskos_fcall {
    ($func_id:expr) => {{
        const _: () = assert!($func_id < 1024, "func_id must be less than 1024");
        unsafe {
            asm!(
                "csrwi {port}, {imm}",
                port = const 0x8C0 + ($func_id >> 5),
                imm = const $func_id & 0x1f
            );
        }
    }};
}

#[macro_export]
macro_rules! ziskos_fcall_mget {
    () => {
        read_csr_ffe()
    };
    (1) => {
        read_csr_ffe()
    };
    (2) => {
        [read_csr_ffe(), read_csr_ffe()]
    };
    (3) => {
        [read_csr_ffe(), read_csr_ffe(), read_csr_ffe()]
    };
    (4) => {
        [
            $crate::read_csr_ffe(),
            $crate::read_csr_ffe(),
            $crate::read_csr_ffe(),
            $crate::read_csr_ffe(),
        ]
    };
    (8) => {
        [
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
        ]
    }; // afegeix més si cal
    (12) => {
        [
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
        ]
    }; // afegeix més si cal
    (16) => {
        [
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
            read_csr_ffe(),
        ]
    }; // afegeix més si cal
    ($len:expr) => {{
        let mut arr = [0u64; $len];
        let mut i = 0;
        while i < $len {
            arr[i] = read_csr_ffe();
            i += 1;
        }
        arr
    }};
}
