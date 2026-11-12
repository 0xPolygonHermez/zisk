//! Asks for a prompt, generates a continuation by running the llama2 guest
//! on ziskemu, and prints it. Also runs the model natively and reports
//! whether both runs produced the same tokens.
//!
//! ```text
//! cargo run --release --bin llama2-zisk -- [OPTIONS] [--ziskemu PATH] [--save-input PATH]
//! ```
//!
//! `--save-input PATH` writes the guest input for the first prompt and exits.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use llama2_common::generate::generate;
use llama2_common::model::Model;
use llama2_common::params::GenParams;
use llama2_host::{
    Result, TextPrinter, gen_params, load, output_digest, parse_options, print_model_info, prompts,
};

/// Guest ELF, built by `build.rs`.
const GUEST_ELF: &str = env!("ZISK_ELF_llama2-guest");

/// Appends one stdin record: a `u64` length, then the data padded to 8 bytes.
fn push_record(input: &mut Vec<u8>, data: &[u8]) {
    input.extend_from_slice(&(data.len() as u64).to_le_bytes());
    input.extend_from_slice(data);
    input.resize(input.len().next_multiple_of(8), 0);
}

struct EmuRun {
    tokens: Vec<u32>,
    committed: Vec<u8>,
    steps: Option<u64>,
    secs: f64,
}

/// The guest's input file: the model blob, then the generation parameters.
fn guest_input(blob: &[u8], params: &GenParams) -> Vec<u8> {
    let mut input = Vec::with_capacity(blob.len() + 4096);
    push_record(&mut input, blob);
    push_record(&mut input, &params.to_bytes());
    input
}

fn run_ziskemu(ziskemu: &str, blob: &[u8], params: &GenParams, dir: &Path) -> Result<EmuRun> {
    let input_path = dir.join("input.bin");
    let output_path = dir.join("output.bin");
    std::fs::write(&input_path, guest_input(blob, params))?;

    let start = Instant::now();
    let out = Command::new(ziskemu)
        .arg("-e")
        .arg(GUEST_ELF)
        .arg("-i")
        .arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .arg("-m")
        .output()
        .map_err(|e| format!("cannot run {ziskemu}: {e} (is ziskemu installed? see --ziskemu)"))?;
    let secs = start.elapsed().as_secs_f64();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        return Err(format!("ziskemu failed ({}):\n{stdout}\n{stderr}", out.status).into());
    }

    let line = stdout
        .lines()
        .find_map(|l| l.strip_prefix("tokens:"))
        .ok_or_else(|| format!("the guest printed no tokens:\n{stdout}\n{stderr}"))?;
    let tokens = line.split_whitespace().map(str::parse).collect::<std::result::Result<_, _>>()?;
    let steps = parse_steps(&stdout).or_else(|| parse_steps(&stderr));
    let committed = std::fs::read(&output_path)?;
    Ok(EmuRun { tokens, committed, steps, secs })
}

/// Finds the step count in ziskemu's `-m` metrics.
fn parse_steps(text: &str) -> Option<u64> {
    text.lines().find_map(|l| {
        let l = l.to_lowercase();
        let rest = &l[l.find("steps")? + 5..];
        let digits: String = rest
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit() || *c == ',' || *c == '_')
            .filter(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    })
}

fn main() -> Result<()> {
    let o = parse_options(&["--ziskemu", "--save-input"])?;
    let extra = |name: &str| o.extra.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
    let ziskemu = extra("--ziskemu").unwrap_or_else(|| "ziskemu".to_string());
    let l = load(&o)?;

    // Write the guest input for the first prompt and exit, for running the
    // guest with cargo-zisk or ziskemu directly.
    if let Some(path) = extra("--save-input") {
        let text = prompts(&o).next().unwrap_or_default();
        std::fs::write(&path, guest_input(&l.blob, &gen_params(&o, &l.tokenizer, &text)))?;
        println!("Saved guest input to {path}");
        return Ok(());
    }

    print_model_info(&o, &l);
    println!("Guest ELF {GUEST_ELF}");
    let model = Model::parse(&l.blob);
    let dir: PathBuf = std::env::temp_dir().join(format!("llama2-zisk-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;

    for text in prompts(&o) {
        let params = gen_params(&o, &l.tokenizer, &text);
        println!("[running on ziskemu...]");
        let run = match run_ziskemu(&ziskemu, &l.blob, &params, &dir) {
            Ok(run) => run,
            Err(e) => {
                std::fs::remove_dir_all(&dir).ok();
                return Err(e);
            }
        };

        let mut printer = TextPrinter::start(&l.tokenizer, &text, &params.prompt);
        for &t in &run.tokens {
            printer.token(t);
        }
        println!();

        let digest_ok = run.committed.get(..32) == Some(&output_digest(&params, &run.tokens)[..]);
        let native = generate(&model, &params, |_| {});
        let steps = run.steps.map_or_else(|| "? steps".to_string(), |s| format!("{s} steps"));
        println!(
            "[ziskemu: {} prompt + {} generated tokens, {steps}, {:.2} s]",
            params.prompt.len(),
            run.tokens.len(),
            run.secs
        );
        println!(
            "[committed hash matches printed tokens: {}; same tokens as native: {}]",
            if digest_ok { "yes" } else { "NO" },
            if native == run.tokens { "yes" } else { "NO" }
        );
    }
    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}
