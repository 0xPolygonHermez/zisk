//! Prepare and replay the saved Qwen3-4B transcript on ziskemu or the assembly emulator.
mod asm;
mod prepare;
mod run;
mod tile;

use anyhow::{ensure, Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verify the original BF16 checkpoint and prepare one input record per job.
    Prepare {
        checkpoint: PathBuf,
        records: PathBuf,
        #[arg(long)]
        request: Option<PathBuf>,
    },
    /// Replay the embedding, decoder layers and vocabulary head in order.
    Run(Run),
    /// Compare the two projection kernels on the frozen 64-row tile.
    Tile {
        checkpoint: PathBuf,
        output: PathBuf,
        #[arg(long, default_value = "ziskemu")]
        emulator: PathBuf,
        #[arg(long)]
        elf: PathBuf,
    },
}

#[derive(Args)]
struct Run {
    #[arg(long)]
    records: PathBuf,
    /// The guest ELF, run by ziskemu (`--emulator`).
    #[arg(long, required_unless_present = "asm", conflicts_with = "asm")]
    elf: Option<PathBuf>,
    #[arg(long, default_value = "ziskemu")]
    emulator: PathBuf,
    /// Run on this assembly emulator (`ziskemuasm`, built for the guest ELF in fast mode) instead
    /// of ziskemu: much faster, but it reports steps only, no cost.
    #[arg(long)]
    asm: Option<PathBuf>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    request: Option<PathBuf>,
    /// Optional reference logits in position-major little-endian FP32 order.
    #[arg(long)]
    reference_logits: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Request {
    prompt_ids: Vec<u32>,
    reply_ids: Vec<u32>,
    max_reply: u32,
    nonce: String,
}

fn tagged(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"PCI/v1/");
    hasher.update(domain);
    hasher.update([0]);
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn token_hash(domain: &[u8], ids: &[u32]) -> [u8; 32] {
    let bytes: Vec<_> = ids.iter().flat_map(|id| id.to_le_bytes()).collect();
    tagged(domain, &[&(ids.len() as u64).to_le_bytes(), &bytes])
}

impl Request {
    fn load(path: Option<&PathBuf>) -> Result<Self> {
        let bytes = match path {
            Some(path) => fs::read(path).with_context(|| format!("read {}", path.display()))?,
            None => include_bytes!("../../saved_request.json").to_vec(),
        };
        let request: Self = serde_json::from_slice(&bytes)?;
        for ids in [&request.prompt_ids, &request.reply_ids] {
            ensure!(!ids.is_empty() && ids.iter().all(|&id| id < 151936), "invalid token IDs");
        }
        ensure!(
            request.reply_ids.len() <= request.max_reply as usize
                && request.prompt_ids.len() + request.max_reply as usize <= 40960,
            "invalid reply limit"
        );
        ensure!(hex::decode(&request.nonce)?.len() == 32, "expected a 32-byte nonce");
        Ok(request)
    }

    fn request_id(&self, model: &[u8], suite: &[u8]) -> [u8; 32] {
        let nonce = hex::decode(&self.nonce).expect("validated nonce");
        tagged(
            b"request/greedy-lowest-id",
            &[
                model,
                suite,
                &nonce,
                &token_hash(b"prompt", &self.prompt_ids),
                &self.max_reply.to_le_bytes(),
                &40960u32.to_le_bytes(),
                &token_hash(b"stop-ids", &[151643, 151645]),
            ],
        )
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Prepare { checkpoint, records, request } => {
            prepare::prepare(&checkpoint, &records, &Request::load(request.as_ref())?)
        }
        Command::Run(args) => run::run(&args, &Request::load(args.request.as_ref())?),
        Command::Tile { checkpoint, output, emulator, elf } => {
            tile::compare(&checkpoint, &output, &emulator, &elf)
        }
    }
}
