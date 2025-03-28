use core::arch::asm;

// fcall_get 0xFFE

pub fn ziskos_fcall_get() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, 0xFFE", out(reg) value);
    }
    value
}

#[macro_export]
macro_rules! ziskos_fcall_param {
    ($addr:expr) => {{
        unsafe {
            asm!(
                concat!("csrs 0x888, {value}"),
                value = in(reg) $addr
            );
        }
    }};
}

#[macro_export]
macro_rules! ziskos_fcall {
    ($csr_addr:expr, $addr:expr) => {{
        unsafe {
            asm!(
                "csrs {port}, {value}",
                value = in(reg) $addr,
                port = const $csr_addr
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
