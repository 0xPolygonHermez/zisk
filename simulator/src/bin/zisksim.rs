use riscv2zisk::Riscv2zisk;
use std::{
    env, fs,
    fs::metadata,
    path::{Path, PathBuf},
    process,
};
use zisksim::{Sim, SimOptions};

fn _list_files(vec: &mut Vec<PathBuf>, path: &Path) {
    if metadata(path).unwrap().is_dir() {
        let paths = fs::read_dir(path).unwrap();
        for path_result in paths {
            let full_path = path_result.unwrap().path();
            if metadata(&full_path).unwrap().is_dir() {
                _list_files(vec, &full_path);
            } else {
                vec.push(full_path);
            }
        }
    }
}

fn list_files(path: &Path) -> Vec<PathBuf> {
    let mut vec = Vec::new();
    _list_files(&mut vec, path);
    vec
}

fn main() {
    //println!("zisk_tester converts an ELF RISCV file into a ZISK ASM program, simulates it, and
    // copies the output to console");

    // Get program arguments
    let args: Vec<String> = env::args().collect();

    // Check program arguments length
    if args.len() != 2 {
        eprintln!("Error parsing arguments: number of arguments should be 1.  Usage: zisk_tester <elf_riscv_file>");
        process::exit(1);
    }

    let argument = &args[1];
    let mut multiple_files = false;
    let md = metadata(argument).unwrap();
    let mut elf_files: Vec<String> = Vec::new();
    if md.is_file() {
        elf_files.push(argument.clone());
    } else if md.is_dir() {
        multiple_files = true;
        let path = Path::new(argument);
        let files = list_files(path);
        for file in files {
            let file_name = file.display().to_string();
            if file_name.contains("dut") && file_name.ends_with(".elf") {
                elf_files.push(file_name.to_string().clone());
                println!("found DUT ELF file: {}", file_name);
            }
        }
    }

    let elf_files_len = elf_files.len();

    if multiple_files {
        println!("Going to process {} ELF files", elf_files_len);
    }

    //const FIRST_ELF_FILE: u64 = 0;

    for (elf_file_counter, elf_file) in elf_files.into_iter().enumerate() {
        // Get the input parameters: ELF (RISCV) file name (input data)
        //let elf_file = args[1].clone();
        let zisk_file = String::new();

        if multiple_files {
            println!("ELF file {}/{}: {}", elf_file_counter, elf_files_len, elf_file);
        }
        /*if (FIRST_ELF_FILE > 0) && (elf_file_counter < FIRST_ELF_FILE) {
            println!("Skipping file {}", elf_file);
            continue;
        }*/

        // Create an instance of the program converter
        let rv2zk = Riscv2zisk::new(elf_file, zisk_file);

        // Convert program to rom
        let result = rv2zk.run();
        if result.is_err() {
            println!("Application error: {}", result.err().unwrap());
            process::exit(1);
        }
        let rom = result.unwrap();

        // Create an empty input
        let input: Vec<u8> = Vec::new();

        // Create a simulator instance with this rom and input
        let mut sim = Sim::new(rom, input);

        // Create a simulator options instance with the default values
        let sim_options = SimOptions::new();

        // Run the simulations
        sim.run(sim_options);
        if !sim.terminated() {
            println!("Simulation did not complete");
            process::exit(1);
        }

        if !multiple_files {
            // Get the simulation outpus as a u32 vector
            let output = sim.get_output_32();

            // Log the output in console
            for o in &output {
                println!("{:08x}", o);
            }
        }
    }

    // Return successfully
    process::exit(0);
}
