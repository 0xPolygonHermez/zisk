//! Structural codec for FloatDot minimal-trace data, without an outer operation-bus header.
//! Decoding does not prove arithmetic, mapped memory, or consistency of overlapping reads.

use crate::{bf16_to_fp32, fp16_to_fp32, fp32_add, fp32_mul, Fp32Result, FLOATDOT_MAX_LEN};

/// Operand encoding selected by descriptor word 4 bits 8..15. Accumulation is always FP32.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatDotFormat {
    /// Legacy contiguous packed binary32 operands, two per aligned word.
    Fp32 = 0,
    /// bfloat16 operands, exactly embedded as `bits << 16`.
    Bf16 = 1,
    /// IEEE binary16 operands, exactly widened to binary32.
    Fp16 = 2,
}

impl FloatDotFormat {
    /// Exact flag-free embedding of one 16-bit operand. Not defined for FP32 inputs.
    pub fn widen(self, h: u16) -> u32 {
        match self {
            Self::Fp32 => panic!("FP32 operands are not 16-bit lanes"),
            Self::Bf16 => bf16_to_fp32(h),
            Self::Fp16 => fp16_to_fp32(h),
        }
    }
}

/// The five descriptor words after validating address, length and initial-state encodings.
/// `x`/`y` are byte addresses. Strides are in elements and are zero for FP32 inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatDotDescriptor {
    pub x: u32,
    pub y: u32,
    pub n: usize,
    pub initial: Fp32Result,
    pub format: FloatDotFormat,
    pub stride_x: u16,
    pub stride_y: u16,
}

impl FloatDotDescriptor {
    /// Number of extended words per vector: packed pairs for FP32, one per 16-bit element.
    pub fn words_per_vector(&self) -> usize {
        match self.format {
            FloatDotFormat::Fp32 => self.n.div_ceil(2),
            _ => self.n,
        }
    }

    /// Byte address of 16-bit element `i`; validated decoding guarantees it does not wrap.
    pub fn element_address(&self, base: u32, stride: u16, i: usize) -> u32 {
        assert_ne!(self.format, FloatDotFormat::Fp32, "FP32 operands are packed pairs");
        base + 2 * u32::from(stride) * u32::try_from(i).expect("bounded FloatDot index")
    }

    /// Aligned addresses of the extended words of one vector, in payload order.
    pub fn word_addresses(&self, base: u32, stride: u16) -> Vec<u32> {
        (0..self.words_per_vector())
            .map(|i| match self.format {
                FloatDotFormat::Fp32 => base + 8 * u32::try_from(i).expect("bounded index"),
                _ => self.element_address(base, stride, i) & !7,
            })
            .collect()
    }

    /// The binary32 operand of term `i`, from that vector's extended words.
    pub fn operand(&self, words: &[u64], base: u32, stride: u16, i: usize) -> u32 {
        match self.format {
            FloatDotFormat::Fp32 => (words[i / 2] >> ((i % 2) * 32)) as u32,
            format => {
                let lane = (self.element_address(base, stride, i) >> 1) & 3;
                format.widen((words[i] >> (16 * lane)) as u16)
            }
        }
    }
}

/// One aligned 8-byte read. `step` is the unmodified VM step, not a memory-bus timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatDotMemoryRead {
    pub address: u32,
    pub step: u64,
    pub value: u64,
}

/// Validated fixed replay header. It is sufficient to derive the extended payload size.
#[derive(Clone, Copy, Debug)]
pub struct FloatDotCallHeader {
    descriptor_addr: u32,
    step: u64,
    descriptor: FloatDotDescriptor,
    result: Fp32Result,
    words: [u64; 6],
}

fn aligned_range(address: u64, words: usize) -> Result<u32, &'static str> {
    let addr = u32::try_from(address).map_err(|_| "FloatDot address exceeds 32 bits")?;
    if addr & 7 != 0 {
        return Err("FloatDot address must be 8-byte aligned");
    }
    let bytes = u64::try_from(words)
        .ok()
        .and_then(|words| words.checked_mul(8))
        .ok_or("FloatDot memory range overflows")?;
    if bytes > (1u64 << 32) - address {
        return Err("FloatDot memory range exceeds the 32-bit address space");
    }
    Ok(addr)
}

/// Decode `x | stride << 32` for a 16-bit vector: 2-byte aligned, stride below 2^16, and
/// every element of a nonempty vector, the last being highest, ends within 2^32.
fn strided_range(word: u64, n: usize) -> Result<(u32, u16), &'static str> {
    let stride = u16::try_from(word >> 32).map_err(|_| "floatdot stride exceeds 16 bits")?;
    let base = word & u64::from(u32::MAX);
    if base & 1 != 0 {
        return Err("floatdot 16-bit vector must be 2-byte aligned");
    }
    let last = u64::try_from(n.saturating_sub(1)).expect("bounded length");
    if n != 0 && base + 2 * u64::from(stride) * last + 2 > 1 << 32 {
        return Err("FloatDot memory range exceeds the 32-bit address space");
    }
    Ok((u32::try_from(base).expect("masked address"), stride))
}

impl FloatDotCallHeader {
    /// Parse exactly `[x, y, n, initial_bits, initial_flags, result_packed]`.
    /// `result_packed = bits | (flags << 32)` is witness data, not a sixth descriptor word.
    /// The caller must separately bind the outer opcode/type, reserved a=0, b and step.
    pub fn decode(descriptor_addr: u64, step: u64, payload: &[u64]) -> Result<Self, &'static str> {
        let descriptor_addr = aligned_range(descriptor_addr, 5)?;
        if step >= 1 << 36 {
            return Err("FloatDot VM step exceeds 36 bits");
        }
        let header: [u64; 6] =
            payload.try_into().map_err(|_| "FloatDot header requires six words")?;
        let n =
            usize::try_from(payload[2]).map_err(|_| "FloatDot length exceeds the host range")?;
        if n > FLOATDOT_MAX_LEN {
            return Err("FloatDot length exceeds the per-call bound");
        }
        let format = match payload[4] >> 8 {
            0 => FloatDotFormat::Fp32,
            1 => FloatDotFormat::Bf16,
            2 => FloatDotFormat::Fp16,
            _ => return Err("floatdot input format is unsupported"),
        };
        let (x, stride_x, y, stride_y) = if format == FloatDotFormat::Fp32 {
            let words = n.div_ceil(2);
            (aligned_range(payload[0], words)?, 0, aligned_range(payload[1], words)?, 0)
        } else {
            let (x, stride_x) = strided_range(payload[0], n)?;
            let (y, stride_y) = strided_range(payload[1], n)?;
            (x, stride_x, y, stride_y)
        };
        let bits = u32::try_from(payload[3]).map_err(|_| "FloatDot initial bits exceed 32 bits")?;
        if payload[4] & 0xe0 != 0 || payload[5] >> 37 != 0 {
            return Err("FloatDot flags contain reserved bits");
        }
        let flags = u8::try_from(payload[4] & 31).expect("validated five-bit flags");
        let result = Fp32Result {
            bits: u32::try_from(payload[5] & u64::from(u32::MAX)).expect("masked result bits"),
            flags: u8::try_from(payload[5] >> 32).expect("validated five-bit flags"),
        };
        Ok(Self {
            descriptor_addr,
            step,
            descriptor: FloatDotDescriptor {
                x,
                y,
                n,
                initial: Fp32Result { bits, flags },
                format,
                stride_x,
                stride_y,
            },
            result,
            words: header,
        })
    }

    pub fn descriptor_addr(&self) -> u32 {
        self.descriptor_addr
    }

    pub fn step(&self) -> u64 {
        self.step
    }

    pub fn descriptor(&self) -> FloatDotDescriptor {
        self.descriptor
    }

    pub fn result(&self) -> Fp32Result {
        self.result
    }

    pub fn data_ext_len(&self) -> usize {
        2 * self.descriptor.words_per_vector()
    }

    pub fn memory_read_count(&self) -> usize {
        5 + self.data_ext_len()
    }

    pub fn encode(&self) -> [u64; 6] {
        self.words
    }
}

/// A structurally validated, borrowed replay payload. No arithmetic or memory is authenticated.
#[derive(Debug)]
pub struct FloatDotCall<'a> {
    header: FloatDotCallHeader,
    data_ext: &'a [u64],
}

impl<'a> FloatDotCall<'a> {
    /// Parse a contiguous fixed header followed by all x words, then all y words.
    pub fn decode(
        descriptor_addr: u64,
        step: u64,
        payload: &'a [u64],
    ) -> Result<Self, &'static str> {
        if payload.len() < 6 {
            return Err("FloatDot replay payload is truncated");
        }
        Self::decode_parts(descriptor_addr, step, &payload[..6], &payload[6..])
    }

    /// Parse the fixed header and the separately dispatched vector words without copying them.
    pub fn decode_parts(
        descriptor_addr: u64,
        step: u64,
        header: &[u64],
        data_ext: &'a [u64],
    ) -> Result<Self, &'static str> {
        let header = FloatDotCallHeader::decode(descriptor_addr, step, header)?;
        if data_ext.len() != header.data_ext_len() {
            return Err("FloatDot replay payload has the wrong number of vector words");
        }
        Ok(Self { header, data_ext })
    }

    pub fn descriptor_addr(&self) -> u32 {
        self.header.descriptor_addr()
    }

    pub fn step(&self) -> u64 {
        self.header.step()
    }

    pub fn descriptor(&self) -> FloatDotDescriptor {
        self.header.descriptor()
    }

    pub fn result(&self) -> Fp32Result {
        self.header.result()
    }

    pub fn x_words(&self) -> &[u64] {
        &self.data_ext[..self.header.data_ext_len() / 2]
    }

    pub fn y_words(&self) -> &[u64] {
        &self.data_ext[self.header.data_ext_len() / 2..]
    }

    /// Compute the software reference result for witness generation or diagnostics only.
    /// This does not authenticate memory or replace the arithmetic AIR.
    pub fn reference_result(&self) -> Fp32Result {
        let d = self.descriptor();
        let mut result = d.initial;
        for i in 0..d.n {
            let x = d.operand(self.x_words(), d.x, d.stride_x, i);
            let y = d.operand(self.y_words(), d.y, d.stride_y, i);
            let product = fp32_mul(x, y);
            let sum = fp32_add(result.bits, product.bits);
            result = Fp32Result { bits: sum.bits, flags: result.flags | product.flags | sum.flags };
        }
        result
    }

    /// Reproduce the exact data payload, preserving odd-vector padding and the claimed result.
    pub fn encode(&self) -> Vec<u64> {
        let mut payload = Vec::with_capacity(6 + self.data_ext.len());
        payload.extend_from_slice(&self.header.words);
        payload.extend_from_slice(self.data_ext);
        payload
    }

    /// Five descriptor reads, then all x words, then all y words, at the same VM step.
    /// A 16-bit vector reads the enclosing aligned word of each element, in index order.
    /// Overlapping and repeated reads remain separate; the result is never a memory read.
    pub fn memory_reads(&self) -> Vec<FloatDotMemoryRead> {
        let mut reads = Vec::with_capacity(self.header.memory_read_count());
        let d = self.descriptor();
        let descriptor = (0..5).map(|i| self.descriptor_addr() + 8 * i);
        let addresses = descriptor
            .chain(d.word_addresses(d.x, d.stride_x))
            .chain(d.word_addresses(d.y, d.stride_y));
        let values = self.header.words[..5].iter().chain(self.data_ext);
        for (address, &value) in addresses.zip(values) {
            reads.push(FloatDotMemoryRead { address, step: self.step(), value });
        }
        reads
    }
}
