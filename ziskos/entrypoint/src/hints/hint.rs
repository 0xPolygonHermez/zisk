use std::collections::VecDeque;
use std::io::{self};

#[cfg(zisk_hints_reference)]
use std::io::Read;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Condvar, Mutex,
};

use crate::hints::HintModExp;

pub const MAX_SLICE_U64_LEN: usize = 192;

#[derive(Clone, Debug)]
pub struct HintSliceU64 {
    pub header: u64,
    pub data: [u64; MAX_SLICE_U64_LEN],
    pub len: usize,
}

impl HintSliceU64 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = unsafe {
            core::slice::from_raw_parts(self.data.as_ptr() as *const u8, self.len * 8)
        };
        (self.header.to_le_bytes(), bytes)
    }

    #[cfg(zisk_hints_metrics)]
    #[inline(always)]
    fn hint_id(&self) -> u32 {
        (self.header >> 32) as u32
    }
}

#[derive(Clone, Debug)]
pub enum Hint {
    HintSliceU64(HintSliceU64),
    HintModExp(HintModExp),
}

impl Hint {
    #[cfg(zisk_hints_metrics)]
    #[inline(always)]
    pub fn hint_id(&self) -> u32 {
        match self {
            Hint::HintSliceU64(hint) => hint.hint_id(),
            Hint::HintModExp(hint) => hint.hint_id(),
        }
    }

    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        match self {
            Hint::HintSliceU64(hint) => hint.header_and_payload(),
            Hint::HintModExp(hint) => hint.header_and_payload(),
        }
    }

    #[cfg(zisk_hints_reference)]
    #[inline(always)]
    pub fn read_from(&self, file: &mut std::fs::File, disable_prefix: bool) -> Result<(), String> {
        let id = self.hint_id();
        let (expected_header, expected_payload) = self.header_and_payload();

        if !disable_prefix {
            let mut header = [0u8; 8];
            if let Err(e) = file.read_exact(&mut header) {
                return Err(format!("Failed to read {:?} header, error: {}", id, e));
            }

            if header != expected_header {
                return Err(format!("Unexpected {:?} header: expected {:?}, got {:?}", id, expected_header, header));
            }
        }

        let mut payload = vec![0u8; expected_payload.len()];
        if let Err(e) = file.read_exact(&mut payload) {
            return Err(format!("Failed to read {:?} payload, error: {}", id, e));
        }

        if payload.as_slice() != expected_payload {
            return Err(format!("{:?} value mismatch", id));
        }

        Ok(())
    }

     #[inline(always)]
    pub fn write_to<W: std::io::Write>(&self, w: &mut W, disable_prefix: bool) -> io::Result<()> {
        debug_assert!(cfg!(target_endian = "little"));

        let (header, payload) = self.header_and_payload();

        if !disable_prefix {
            w.write_all(&header)?;
        }

        w.write_all(payload)?;

        Ok(())
    }
}
#[derive(Debug)]
pub struct HintQueue {
    states: Mutex<VecDeque<Hint>>,
    condvar: Condvar,
    closed: AtomicBool,
    paused: AtomicBool,
}

impl HintQueue {
    pub const fn new() -> Self {
        Self {
            states: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            closed: AtomicBool::new(true),
            paused: AtomicBool::new(false),
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

     #[inline(always)]
    pub fn is_open(&self) -> bool {
        !self.closed.load(Ordering::SeqCst)
    }

     #[inline(always)]
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    #[inline(always)]
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    #[inline(always)]
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }
}