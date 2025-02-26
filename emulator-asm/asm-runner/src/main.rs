use std::borrow::Cow;
use std::slice;

#[repr(C)]
#[derive(Debug)]
pub struct AsmRunnerInputC {
    pub chunk_size: u64,
    pub max_steps: u64,
    pub initial_trace_size: u64,
    pub input_data_size: u64,
    pub input_data: *const u64,
}

#[derive(Debug)]
pub struct AsmRunnerInput {
    pub chunk_size: u64,
    pub max_steps: u64,
    pub initial_trace_size: u64,
    pub input_data: Vec<u64>,
}

impl AsmRunnerInput {
    pub fn to_c(&self) -> AsmRunnerInputC {
        AsmRunnerInputC {
            chunk_size: self.chunk_size,
            max_steps: self.max_steps,
            initial_trace_size: self.initial_trace_size,
            input_data_size: self.input_data.len() as u64,
            input_data: self.input_data.as_ptr(),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct AsmRunnerOutputC {
    pub version: u64,
    pub exit_code: u64,
    pub mt_address: u64, // Pointer to memory trace
    pub mt_allocated_size: u64,
    pub mt_used_size: u64,
}

#[derive(Debug)]
pub struct AsmRunnerOutput<'a> {
    pub version: u64,
    pub exit_code: u64,
    pub mt_address: u64,
    pub mt: Cow<'a, [u64]>, // Borrowed if possible, owned if needed
}

impl<'a> AsmRunnerOutput<'a> {
    /// Convert Rust structure to C-compatible structure
    pub fn to_c(&self) -> AsmRunnerOutputC {
        AsmRunnerOutputC {
            version: self.version,
            exit_code: self.exit_code,
            mt_address: self.mt_address,
            mt_allocated_size: self.mt.len() as u64,
            mt_used_size: self.mt.len() as u64,
        }
    }

    /// Convert from `AsmRunnerOutputC` (C) to `AsmRunnerOutput<'a>` (Rust)
    pub unsafe fn from_c(output: &'a AsmRunnerOutputC) -> Self {
        let mt = if output.mt_address != 0 {
            unsafe {
                Cow::Borrowed(slice::from_raw_parts(
                    output.mt_address as *const u64,
                    output.mt_used_size as usize,
                ))
            }
        } else {
            Cow::Owned(Vec::new()) // In case it's empty, provide an owned empty Vec
        };

        Self {
            version: output.version,
            exit_code: output.exit_code,
            mt_address: output.mt_address,
            mt,
        }
    }
}

fn main() {
    let input = AsmRunnerInput {
        chunk_size: 1,
        max_steps: 2,
        initial_trace_size: 3,
        input_data: vec![4, 5, 6],
    };

    let mut data = vec![100u64, 200, 300, 400];

    let output_c = AsmRunnerOutputC {
        version: 7,
        exit_code: 8,
        mt_address: data.as_ptr() as u64,
        mt_allocated_size: data.len() as u64,
        mt_used_size: data.len() as u64,
    };

    let input_c = input.to_c();

    let output = unsafe { AsmRunnerOutput::from_c(&output_c) };

    println!("{:?}", input);
    println!("{:?}", output);

    data[0] = 1000;

    println!("{:?}", output);
}
