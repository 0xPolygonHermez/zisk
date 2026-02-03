use crate::{
    command::create_command, utils::cargo_rerun_if_changed, BuildArgs, HELPER_TARGET_SUBDIR,
    ZISK_TARGET,
};
use cargo_metadata::camino::Utf8PathBuf;
use rom_setup::{assembly_files_exist, gen_assembly, get_output_path};
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{exit, Command, Stdio},
    thread,
};

use anyhow::{Context, Result};

// Helper for building a ZisK program.
pub(crate) fn build_program_internal(path: &str, args: Option<BuildArgs>) {
    // Get the root package name and metadata.
    let program_dir = std::path::Path::new(path);
    let metadata_file = program_dir.join("Cargo.toml");
    let mut metadata_cmd = cargo_metadata::MetadataCommand::new();
    let metadata = metadata_cmd.manifest_path(metadata_file).exec().unwrap();

    // Activate the build command if the dependencies change.
    cargo_rerun_if_changed(&metadata, program_dir);

    // Check if RUSTC_WORKSPACE_WRAPPER is set to clippy-driver (i.e. if `cargo clippy` is the
    // current compiler). If so, don't execute `cargo prove build` because it breaks
    // rust-analyzer's `cargo clippy` feature.
    let is_clippy_driver = std::env::var("RUSTC_WORKSPACE_WRAPPER")
        .map(|val| val.contains("clippy-driver"))
        .unwrap_or(false);
    if is_clippy_driver {
        // Still need to set ELF env vars even if build is skipped.
        let target_elf_paths = generate_elf_paths(&metadata, args.as_ref());
        let hints = args
            .as_ref()
            .and_then(|a| a.hints)
            .or_else(|| std::env::var("ZISK_HINTS").ok().and_then(|v| v.parse().ok()))
            .unwrap_or(false);
        print_elf_paths_cargo_directives(&target_elf_paths, hints);

        println!("cargo:warning=Skipping build due to clippy invocation.");
        return;
    }

    // Build the program with the given arguments.
    let path_output = if let Some(args) = &args {
        execute_build_program(args, Some(program_dir.to_path_buf()))
    } else {
        // Detect the host's build profile and use it for the guest program
        let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
        let default_args = BuildArgs { release: profile == "release", ..Default::default() };

        execute_build_program(&default_args, Some(program_dir.to_path_buf()))
    };
    if let Err(err) = path_output {
        panic!("Failed to build ZisK program: {err}.");
    }
}

pub fn execute_build_program(
    args: &BuildArgs,
    program_dir: Option<PathBuf>,
) -> Result<Vec<(String, Utf8PathBuf)>> {
    // If the program directory is not specified, use the current directory.
    let program_dir = program_dir
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory."));
    let program_dir: Utf8PathBuf =
        program_dir.try_into().expect("Failed to convert PathBuf to Utf8PathBuf");

    // Check for ZISK_PATH and ZISK_HINTS environment variables if not set in args
    let mut args = args.clone();
    if args.zisk_path.is_none() {
        if let Ok(env_path) = std::env::var("ZISK_PATH") {
            args.zisk_path = Some(env_path);
        }
    }
    if args.hints.is_none() {
        if let Ok(env_hints) = std::env::var("ZISK_HINTS") {
            args.hints = env_hints.parse().ok();
        }
    }

    // Get the program metadata.
    let program_metadata_file = program_dir.join("Cargo.toml");
    let mut program_metadata_cmd = cargo_metadata::MetadataCommand::new();
    let program_metadata = program_metadata_cmd.manifest_path(program_metadata_file).exec()?;

    // Get the command corresponding to Docker or local build.
    let cmd = create_command(&args, &program_dir, &program_metadata);

    let target_elf_paths = generate_elf_paths(&program_metadata, Some(&args));

    if target_elf_paths.len() > 1 && args.elf_name.is_some() {
        anyhow::bail!("--elf-name is not supported when --output-directory is used and multiple ELFs are built.");
    }

    execute_command(cmd)?;

    // Generate assembly for all ELF files (only if not already generated)
    let hints = args.hints.unwrap_or(false);
    println!("cargo:rerun-if-env-changed=ZISK_HINTS");

    let zisk_path_buf = args.zisk_path.as_ref().map(PathBuf::from);
    let output_path = get_output_path(&None)?;
    for (_, elf_path) in target_elf_paths.iter() {
        let elf_path_std = elf_path.as_std_path();

        let assembly_exists = assembly_files_exist(elf_path_std, &output_path)?;
        let hints_marker = output_path.join(format!(
            "{}.assembly_hints",
            elf_path_std.file_name().unwrap().to_string_lossy()
        ));
        let new_value = if hints { "on" } else { "off" };

        let hints_changed = match std::fs::read_to_string(&hints_marker) {
            Ok(prev) => prev != new_value,
            Err(_) => true,
        };

        if !assembly_exists || hints_changed {
            gen_assembly(elf_path_std, &zisk_path_buf, &None, hints, true)?;
            std::fs::write(&hints_marker, new_value)?;
        }
    }

    if let Some(output_directory) = &args.output_directory {
        // The path to the output directory, maybe relative or absolute.
        let output_directory = PathBuf::from(output_directory);

        // Ensure the output directory is a directory. If it doesnt exist, this is false.
        if output_directory.is_file() {
            anyhow::bail!("--output-directory is a file.");
        }

        // Ensure the output directory exists.
        std::fs::create_dir_all(&output_directory)?;

        // Copy the ELF file to the output directory.
        for (_, elf_path) in target_elf_paths.iter() {
            let elf_path = elf_path.to_path_buf();
            let elf_name = elf_path.file_name().expect("ELF path has a file name");
            let output_path = output_directory.join(args.elf_name.as_deref().unwrap_or(elf_name));

            std::fs::copy(&elf_path, &output_path)?;
        }
    }

    print_elf_paths_cargo_directives(&target_elf_paths, hints);

    Ok(target_elf_paths)
}

/// Execute the command and handle the output depending on the context.
pub(crate) fn execute_command(mut command: Command) -> Result<()> {
    // Add necessary tags for stdout and stderr from the command.
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn command")?;
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());

    // Add prefix to the output of the process depending on the context.
    let msg = "[ZisK] ";

    // Pipe stdout and stderr to the parent process with [docker] prefix
    let stdout_handle = thread::spawn(move || {
        stdout.lines().for_each(|line| {
            println!("{} {}", msg, line.unwrap());
        });
    });
    stderr.lines().for_each(|line| {
        eprintln!("{} {}", msg, line.unwrap());
    });
    stdout_handle.join().unwrap();

    // Wait for the child process to finish and check the result.
    let result = child.wait()?;
    if !result.success() {
        // Error message is already printed by cargo.
        exit(result.code().unwrap_or(1))
    }
    Ok(())
}

pub fn generate_elf_paths(
    metadata: &cargo_metadata::Metadata,
    args: Option<&BuildArgs>,
) -> Vec<(String, Utf8PathBuf)> {
    let profile = args.map(|v| if v.release { "release" } else { "debug" }).unwrap_or("debug");
    let root_package = metadata.root_package().expect("No root package found in metadata");
    let bin_target = root_package.targets.first().unwrap();
    let target_elf_path = metadata
        .target_directory
        .join(HELPER_TARGET_SUBDIR)
        .join(ZISK_TARGET)
        .join(profile)
        .join(&bin_target.name);

    vec![(bin_target.name.to_owned(), target_elf_path)]
}
fn print_elf_paths_cargo_directives(target_elf_paths: &[(String, Utf8PathBuf)], hints: bool) {
    println!("cargo:rerun-if-env-changed=ZISK_HINTS");

    for (target_name, elf_path) in target_elf_paths.iter() {
        println!("cargo:rustc-env=ZISK_ELF_{target_name}={elf_path}");
        if hints {
            println!("cargo:rustc-env=ZISK_ELF_{target_name}_WITH_HINTS=1");
        }
    }
}
