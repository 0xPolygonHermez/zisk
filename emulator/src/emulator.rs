use crate::{
    Emu, EmuOptions, EmuStartingPoints, EmuTrace, EmuTraceStart, ErrWrongArguments, ParEmuOptions,
    ZiskEmulatorErr,
};
use p3_field::PrimeField;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Instant,
};
use sysinfo::System;
use zisk_core::{
    Riscv2zisk, ZiskOperationType, ZiskRequiredOperation, ZiskRom, ZISK_OPERATION_TYPE_VARIANTS,
};

pub trait Emulator {
    fn emulate(
        &self,
        options: &EmuOptions,
        callback: Option<impl Fn(EmuTrace)>,
    ) -> Result<Vec<u8>, ZiskEmulatorErr>;
}
use rayon::prelude::*;

pub struct ZiskEmulator;

/*
ziskemu.main()
\
 emulate()
 \
  process_directory()
  \
   process_elf_file()
   \
    Riscv2zisk::run()
    process_rom()
    \
     Emu::run()
*/

impl ZiskEmulator {
    fn process_directory(
        directory: String,
        inputs: &[u8],
        options: &EmuOptions,
    ) -> Result<Vec<u8>, ZiskEmulatorErr> {
        if options.verbose {
            println!("process_directory() directory={}", directory);
        }

        let files = Self::list_files(&directory).unwrap();
        for file in files {
            if file.contains("dut") && file.ends_with(".elf") {
                Self::process_elf_file(file, inputs, options, None::<Box<dyn Fn(EmuTrace)>>)?;
            }
        }

        Ok(Vec::new())
    }

    fn process_elf_file(
        elf_filename: String,
        inputs: &[u8],
        options: &EmuOptions,
        callback: Option<impl Fn(EmuTrace)>,
    ) -> Result<Vec<u8>, ZiskEmulatorErr> {
        if options.verbose {
            println!("process_elf_file() elf_file={}", elf_filename);
        }

        // Convert the ELF file to ZisK ROM
        // Create an instance of the RISCV -> ZisK program converter
        let riscv2zisk = Riscv2zisk::new(elf_filename, String::new(), String::new(), String::new());

        // Convert program to rom
        let zisk_rom = riscv2zisk.run();
        if zisk_rom.is_err() {
            return Err(ZiskEmulatorErr::Unknown(zisk_rom.err().unwrap().to_string()));
        }

        Self::process_rom(&zisk_rom.unwrap(), inputs, options, callback)
    }

    fn process_rom_file(
        rom_filename: String,
        inputs: &[u8],
        options: &EmuOptions,
        callback: Option<impl Fn(EmuTrace)>,
    ) -> Result<Vec<u8>, ZiskEmulatorErr> {
        if options.verbose {
            println!("process_rom_file() rom_file={}", rom_filename);
        }

        // TODO: load from file
        let rom: ZiskRom = ZiskRom::new();
        Self::process_rom(&rom, inputs, options, callback)
    }

    pub fn process_rom(
        rom: &ZiskRom,
        inputs: &[u8],
        options: &EmuOptions,
        callback: Option<impl Fn(EmuTrace)>,
    ) -> Result<Vec<u8>, ZiskEmulatorErr> {
        if options.verbose {
            println!("process_rom() rom size={} inputs size={}", rom.insts.len(), inputs.len());
        }

        // Create a emulator instance with this rom and inputs
        let mut emu = Emu::new(rom);
        let start = Instant::now();

        // Run the emulation
        emu.run(inputs.to_owned(), options, callback);

        if !emu.terminated() {
            return Err(ZiskEmulatorErr::EmulationNoCompleted);
        }

        let duration = start.elapsed();

        // Log performance metrics
        if options.log_metrics {
            let secs = duration.as_secs_f64();
            let steps = emu.number_of_steps();
            let tp = steps as f64 / secs / 1_000_000.0;

            let system = System::new_all();
            let cpu = &system.cpus()[0];
            let cpu_frequency = cpu.frequency() as f64;

            let clocks_per_step = cpu_frequency / tp;
            println!(
                "process_rom() steps={} duration={:.4} tp={:.4} Msteps/s freq={:.4} {:.4} clocks/step",
                steps, secs, tp, cpu_frequency, clocks_per_step
            );
        }

        // Get the emulation output
        let output = emu.get_output_8();

        // OUTPUT:
        // Save output to a file if requested
        if options.output.is_some() {
            fs::write(options.output.as_ref().unwrap(), &output)
                .map_err(|e| ZiskEmulatorErr::Unknown(e.to_string()))?
        }

        // Log output to console if requested
        if options.log_output {
            // Get the emulation output as a u32 vector
            let output = emu.get_output_32();

            // Log the output to console
            for o in &output {
                println!("{:08x}", o);
            }
        }

        Ok(output)
    }

    pub fn par_process_rom<F: PrimeField>(
        rom: &ZiskRom,
        inputs: &[u8],
        options: &EmuOptions,
        num_threads: usize,
        op_sizes: [u64; ZISK_OPERATION_TYPE_VARIANTS],
    ) -> Result<(Vec<EmuTrace>, EmuStartingPoints), ZiskEmulatorErr> {
        let mut emu_traces = vec![Vec::new(); num_threads];
        let emu_slices = Mutex::new(EmuStartingPoints::default());

        emu_traces.par_iter_mut().enumerate().for_each(|(thread_id, emu_trace)| {
            let par_emu_options = ParEmuOptions::new(
                num_threads,
                thread_id,
                op_sizes[ZiskOperationType::None as usize] as usize,
                op_sizes,
            );

            // Run the emulation
            let mut emu = Emu::new(rom);
            let result = emu.par_run::<F>(inputs.to_owned(), options, &par_emu_options);

            if !emu.terminated() {
                panic!("Emulation did not complete");
                // TODO!
                // return Err(ZiskEmulatorErr::EmulationNoCompleted);
            }

            *emu_trace = result.0;

            if thread_id == 0 {
                *emu_slices.lock().unwrap() = result.1;
            }
        });

        let capacity = emu_traces.iter().map(|trace| trace.len()).sum::<usize>();
        let mut vec_traces = Vec::with_capacity(capacity);
        for i in 0..capacity {
            let x = i % num_threads;
            let y = i / num_threads;

            vec_traces.push(std::mem::take(&mut emu_traces[x][y]));
        }

        // For performance reasons we didn't collect main operation starting points because we
        // can generate them from the execution trace.
        let mut emu_slices = emu_slices.into_inner().unwrap();
        emu_slices.total_steps[ZiskOperationType::None as usize] =
            vec_traces.iter().map(|trace| trace.steps.len()).sum::<usize>() as u64;
        for vec_trace in &vec_traces {
            emu_slices.add(
                ZiskOperationType::None,
                vec_trace.start.pc,
                vec_trace.start.sp,
                vec_trace.start.c,
                vec_trace.start.step,
            );
        }

        Ok((vec_traces, emu_slices))
    }

    #[inline]
    pub fn process_slice_required<F: PrimeField>(
        rom: &ZiskRom,
        vec_traces: &[EmuTrace],
        op_type: ZiskOperationType,
        emu_trace_start: &EmuTraceStart,
        num_rows: usize,
    ) -> Vec<ZiskRequiredOperation> {
        // Create a emulator instance with this rom
        let mut emu = Emu::new(rom);
        // Run the emulation
        emu.run_slice_required::<F>(vec_traces, op_type, emu_trace_start, num_rows)
    }

    fn list_files(directory: &str) -> std::io::Result<Vec<String>> {
        fn _list_files(vec: &mut Vec<PathBuf>, path: &Path) -> std::io::Result<()> {
            if path.is_dir() {
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    let full_path = entry.path();
                    if full_path.is_dir() {
                        _list_files(vec, &full_path)?;
                    } else {
                        vec.push(full_path);
                    }
                }
            }
            Ok(())
        }

        let mut paths = Vec::new();
        _list_files(&mut paths, Path::new(directory))?;
        Ok(paths.into_iter().map(|p| p.display().to_string()).collect())
    }
}

impl Emulator for ZiskEmulator {
    fn emulate(
        &self,
        options: &EmuOptions,
        callback: Option<impl Fn(EmuTrace)>,
    ) -> Result<Vec<u8>, ZiskEmulatorErr> {
        // Log this call
        if options.verbose {
            println!("emulate()\n{}", options);
        }

        // Check options are valid
        if options.rom.is_some() && options.elf.is_some() {
            return Err(ZiskEmulatorErr::WrongArguments(ErrWrongArguments::new(
                "ROM file and ELF file are incompatible; use only one of them",
            )));
        } else if options.rom.is_none() && options.elf.is_none() {
            return Err(ZiskEmulatorErr::WrongArguments(ErrWrongArguments::new(
                "ROM file or ELF file must be provided",
            )));
        }

        // INPUTs:
        // build inputs data either from the provided inputs path, or leave it empty (default
        // inputs)
        let mut inputs = Vec::new();
        if options.inputs.is_some() {
            // Read inputs data from the provided inputs path
            let path = PathBuf::from(options.inputs.clone().unwrap());
            inputs = fs::read(path).expect("Could not read inputs file");
        }

        if options.rom.is_some() {
            let rom_filename = options.rom.clone().unwrap();

            let metadata = fs::metadata(&rom_filename);
            if metadata.is_err() {
                return Err(ZiskEmulatorErr::WrongArguments(ErrWrongArguments::new(
                    "ROM file does not exist",
                )));
            }

            let metadata = metadata.unwrap();
            if metadata.is_dir() {
                return Err(ZiskEmulatorErr::WrongArguments(ErrWrongArguments::new(
                    "ROM file must be a file",
                )));
            }

            Self::process_rom_file(rom_filename, &inputs, options, callback)
        } else {
            let elf_filename = options.elf.clone().unwrap();

            let metadata = fs::metadata(&elf_filename);
            if metadata.is_err() {
                return Err(ZiskEmulatorErr::WrongArguments(ErrWrongArguments::new(
                    "ELF file does not exist",
                )));
            }

            let metadata = metadata.unwrap();
            if metadata.is_dir() {
                Self::process_directory(elf_filename, &inputs, options)
            } else {
                Self::process_elf_file(elf_filename, &inputs, options, callback)
            }
        }
    }
}
