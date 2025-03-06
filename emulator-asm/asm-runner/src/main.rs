//extern crate errno;
extern crate libc;
//use errno::{errno, set_errno, Errno};
use libc::{
    ftruncate, memcpy, mmap, munmap, pipe, shm_open, shm_unlink, MAP_SHARED, O_CREAT, PROT_READ,
    PROT_WRITE, S_IRUSR, S_IWUSR, S_IXUSR,
};
use std::borrow::Cow;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;
use std::{fs, time};
use std::{slice, thread};

use libc::{c_char, c_void, off_t, size_t};

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
    let shmem_prefix = "SHM001";
    const shmem_input_name: *const c_char = b"/SHM001_input\0".as_ptr() as *const c_char;
    const shmem_output_name: *const c_char = b"/SHM001_output\0".as_ptr() as *const c_char;

    let mut inputs = Vec::new();
    // Read inputs data from the provided inputs path
    let path = PathBuf::from(/*options.inputs.clone().unwrap()*/ "pessimistic-proof.bin");
    inputs = fs::read(path).expect("Could not read inputs file");

    // Time
    let start = Instant::now();

    let shmem_input_size = ((inputs.len() + 32 + 1) >> 3) << 3;
    let mut shmem_input_data: Vec<u8> = Vec::with_capacity(shmem_input_size);
    //shmem_input_data[0..8] = u64::to_le_bytes(1024*1024);
    shmem_input_data.extend_from_slice((1u64 << 20).to_le_bytes().as_slice()); // Chunk size
    shmem_input_data.extend_from_slice((1u64 << 32).to_le_bytes().as_slice()); // Max steps
    shmem_input_data.extend_from_slice((1u64 << 30).to_le_bytes().as_slice()); // Initial trace size
    shmem_input_data.extend_from_slice((inputs.len() as u64).to_le_bytes().as_slice()); // Input data size
    shmem_input_data.extend_from_slice(inputs.as_slice());

    // Input data

    // Step 4: Open the shared memory object
    unsafe { shm_unlink(shmem_input_name) };
    let shm_fd = unsafe {
        shm_open(
            shmem_input_name,
            libc::O_RDWR | O_CREAT,
            /*0666*/ S_IRUSR | S_IWUSR | S_IXUSR,
        )
    };
    if shm_fd == -1 {
        //let e = errno();
        panic!(
            "Rust: shm_open() failed errno={}={}",
            std::io::Error::last_os_error().raw_os_error().unwrap(),
            //e.0,
            std::io::Error::last_os_error()
        );
    }

    let _res = unsafe { ftruncate(shm_fd, shmem_input_size as i64) };

    // Step 5: Map the shared memory into Rust
    let mapped_ptr = unsafe {
        mmap(std::ptr::null_mut(), shmem_input_size, PROT_READ | PROT_WRITE, MAP_SHARED, shm_fd, 0)
    };
    if mapped_ptr == libc::MAP_FAILED {
        panic!("Rust: mmap failed");
    }

    unsafe {
        memcpy(mapped_ptr, shmem_input_data.as_ptr() as *const c_void, shmem_input_size);
    }
    unsafe { munmap(mapped_ptr, shmem_input_size) };

    // Step 2: Spawn C++ Child Process, passing the pipe file descriptors
    let mut child = Command::new("emulator-asm/build/ziskemuasm")
        .arg(shmem_prefix.to_string()) // Send write FD to child
        .stdin(Stdio::piped()) // For sending "ACK"
        .spawn()
        .expect("Failed to start child process");

    child.wait();

    // println!("Sleeping...");
    // let ten_seconds = time::Duration::from_secs(5);

    // thread::sleep(ten_seconds);
    // println!("Sleeping... done!");

    let shm_fd = unsafe {
        shm_open(shmem_output_name, libc::O_RDONLY, /*0666*/ S_IRUSR | S_IWUSR | S_IXUSR)
    };
    if shm_fd == -1 {
        //let e = errno();
        panic!(
            "Rust: shm_open(output) failed errno={}={}",
            std::io::Error::last_os_error().raw_os_error().unwrap(),
            //e.0,
            std::io::Error::last_os_error()
        );
    }

    let mapped_ptr = unsafe {
        mmap(std::ptr::null_mut(), 32, PROT_READ /*| PROT_WRITE*/, MAP_SHARED, shm_fd, 0)
    };
    if mapped_ptr == libc::MAP_FAILED {
        panic!("Rust: mmap failed");
    }
    let mut output_header: [u64; 4] = [0; 4];
    unsafe {
        memcpy(output_header.as_ptr() as *mut c_void, mapped_ptr, 32);
    }
    unsafe { munmap(mapped_ptr, 32) };

    println!("ziskemuasm version = 0x{:06x}", output_header[0]);
    println!("ziskemuasm exit code = {}", output_header[1]);
    println!("ziskemuasm allocated size = {}", output_header[2]);
    println!("ziskemuasm trace size = {}", output_header[3]);
    let output_allocated_size = output_header.get(2).unwrap();
    println!("ziskemuasm allocated size = {}", output_header[2]);
    println!("ziskemuasm output_allocated_size = {}", *output_allocated_size);
    assert!(*output_allocated_size > 0);
    let output_trace_size = output_header.get(3).unwrap();
    println!("ziskemuasm output_trace_size = {}", *output_trace_size);
    assert!(*output_trace_size > 0);

    let mapped_ptr = unsafe {
        mmap(
            std::ptr::null_mut(),
            //output_allocated_size as usize,
            *output_trace_size as usize + 32,
            PROT_READ, /*| PROT_WRITE*/
            MAP_SHARED,
            shm_fd,
            0,
        )
    };
    if mapped_ptr == libc::MAP_FAILED {
        panic!("Rust: mmap failed");
    }

    // Store the duration of the emulation process as a difference vs. the start time
    let duration = start.elapsed();
    println!("Duration = {} ns", duration.as_nanos());

    // Read / decode minimal trace from mapped_ptr + 32 onwards

    unsafe { munmap(mapped_ptr, *output_trace_size as usize + 32) };

    unsafe { shm_unlink(shmem_output_name) };

    println!("Done!");

    // let input = AsmRunnerInput {
    //     chunk_size: 1,
    //     max_steps: 2,
    //     initial_trace_size: 3,
    //     input_data: vec![4, 5, 6],
    // };

    // let mut data = vec![100u64, 200, 300, 400];

    // let output_c = AsmRunnerOutputC {
    //     version: 7,
    //     exit_code: 8,
    //     mt_address: data.as_ptr() as u64,
    //     mt_allocated_size: data.len() as u64,
    //     mt_used_size: data.len() as u64,
    // };

    // let input_c = input.to_c();

    // let output = unsafe { AsmRunnerOutput::from_c(&output_c) };

    // println!("{:?}", input);
    // println!("{:?}", output);

    // data[0] = 1000;

    // println!("{:?}", output);
}
