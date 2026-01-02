use crate::hints::bigint256::AddMod256;
use crate::hints::bigint256::DivRem256;
use crate::hints::bigint256::MulMod256;
use crate::hints::bigint256::OMul256;
use crate::hints::bigint256::RedMod256;
use crate::hints::bigint256::WMul256;
use crate::hints::bigint256::WPow256;
use crate::hints::modexp::ModExp;
use crate::hints::secp256k1::ECRecover;

use crate::hints::keccakf::*;
use crate::hints::sha256f::*;
use crate::hints::types::HintData;

use std::collections::VecDeque;
use std::io::{self, Read};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Condvar, Mutex,
};

#[derive(Copy, Clone, Debug)]
pub enum HintKind {
    KeccakF,
    Sha2,
    ECRecover,
    // ModExp,
    RedMod256,
    AddMod256,
    MulMod256,
    DivRem256,
    WPow256,
    OMul256,
    WMul256,
    ModExp,
}

#[derive(Clone, Debug)]
pub enum Hint {
    KeccakF(KeccakF),
    SHA2(Sha2),
    ECRecover(ECRecover),
    // ModExp(Vec<u8>),
    RedMod256(RedMod256),
    AddMod256(AddMod256),
    MulMod256(MulMod256),
    DivRem256(DivRem256),
    WPow256(WPow256),
    OMul256(OMul256),
    WMul256(WMul256),
    ModExp(ModExp),
}

impl Hint {
    #[inline]
    #[allow(unused)]
    pub fn kind(&self) -> HintKind {
        match self {
            Hint::KeccakF(_) => HintKind::KeccakF,
            Hint::SHA2(_) => HintKind::Sha2,
            Hint::ECRecover(_) => HintKind::ECRecover,
            // Hint::ModExp(_) => HintKind::ModExp,
            Hint::RedMod256(_) => HintKind::RedMod256,
            Hint::AddMod256(_) => HintKind::AddMod256,
            Hint::MulMod256(_) => HintKind::MulMod256,
            Hint::DivRem256(_) => HintKind::DivRem256,
            Hint::WPow256(_) => HintKind::WPow256,
            Hint::OMul256(_) => HintKind::OMul256,
            Hint::WMul256(_) => HintKind::WMul256,
            Hint::ModExp(_) => HintKind::ModExp,
        }
    }

    #[allow(unused)]
    pub fn read_from(&self, file: &mut std::fs::File, disable_prefix: bool) -> Result<(), String> {
        let kind = self.kind();
        let (expected_header, expected_payload) = self.header_and_payload();

        if !disable_prefix {
            let mut header = [0u8; 8];
            if let Err(e) = file.read_exact(&mut header) {
                return Err(format!("Failed to read {:?} header, error: {}", kind, e));
            }

            if header != expected_header {
                return Err(format!("Unexpected {:?} header: expected {:?}, got {:?}", kind, expected_header, header));
            }
        }

        let mut payload = vec![0u8; expected_payload.len()];
        if let Err(e) = file.read_exact(&mut payload) {
            return Err(format!("Failed to read {:?} payload, error: {}", kind, e));
        }

        if payload.as_slice() != expected_payload {
            return Err(format!("{:?} value mismatch", kind));
        }

        Ok(())
    }

    #[inline]
    pub fn write_to<W: std::io::Write>(&self, w: &mut W, disable_prefix: bool) -> io::Result<()> {
        debug_assert!(cfg!(target_endian = "little"));

        let (header, payload) = self.header_and_payload();

        if !disable_prefix {
            w.write_all(&header)?;
        }

        w.write_all(payload)?;

        Ok(())
    }

    #[inline]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        match self {
            Hint::KeccakF(keccakf) => keccakf.header_and_payload(),
            Hint::SHA2(sha2) => sha2.header_and_payload(),
            Hint::ECRecover(ecrecover) => ecrecover.header_and_payload(),
            Hint::RedMod256(redmod256) => redmod256.header_and_payload(),
            Hint::AddMod256(addmod256) => addmod256.header_and_payload(),
            Hint::MulMod256(mulmod256) => mulmod256.header_and_payload(),
            Hint::DivRem256(divrem256) => divrem256.header_and_payload(),
            Hint::WPow256(wpow256) => wpow256.header_and_payload(),
            Hint::OMul256(omul256) => omul256.header_and_payload(),
            Hint::WMul256(wmul256) => wmul256.header_and_payload(),
            Hint::ModExp(modexp) => modexp.header_and_payload(),
        }
    }
}

#[derive(Debug)]
pub struct HintQueue {
    states: Mutex<VecDeque<Hint>>,
    condvar: Condvar,
    closed: AtomicBool,
}

impl HintQueue {
    pub const fn new() -> Self {
        Self {
            states: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            closed: AtomicBool::new(true),
        }
    }

    pub fn reset(&self) {
        let mut states = self.states.lock().unwrap();
        states.clear();
        self.closed.store(false, Ordering::SeqCst);
    }

    #[inline(always)]
    pub fn push(&self, hint: Hint) {
        let mut states = self.states.lock().unwrap();
        states.push_back(hint);
        self.condvar.notify_one();
    }

    pub fn pop_batch(&self, out: &mut Vec<Hint>, max_batch: usize) -> bool {
        let mut states = self.states.lock().unwrap();
        loop {
            if !states.is_empty() {
                let take = max_batch.min(states.len());
                for _ in 0..take {
                    if let Some(hint) = states.pop_front() {
                        out.push(hint);
                    }
                }
                return true;
            }

            if self.closed.load(Ordering::SeqCst) {
                return false;
            }

            states = self.condvar.wait(states).unwrap();
        }
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.condvar.notify_all();
    }

    pub fn is_open(&self) -> bool {
        !self.closed.load(Ordering::SeqCst)
    }
}