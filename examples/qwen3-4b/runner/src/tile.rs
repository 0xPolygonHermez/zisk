use anyhow::{ensure, Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;

const WIDTH: usize = 2560;
const POSITIONS: usize = 39;
const ROWS: usize = 64;
const GOLD: &str = "f7c1ceb9b4258f70eee60ef741e0a18e56aabe8ce331e25054aeb8ffd7d7cbf3";
const FRAMES: [(u32, &str); 2] = [
    (5, "bef989bb43574b95f36c681b270648e03b2780dfdb7831109a66fb845f92a47f"),
    (6, "c0ed3f1d0dcd7260c224ff0c32037b678403239f7b1a66894a95b995bbb7ab8a"),
];

struct Tensor {
    shard: File,
    start: u64,
    rows: u64,
}

impl Tensor {
    fn open(checkpoint: &Path, index: &Value, name: &str, shape: &[u64]) -> Result<Self> {
        let filename = index["weight_map"][name].as_str().context("missing tensor shard")?;
        let mut shard =
            File::open(checkpoint.join(filename)).with_context(|| filename.to_owned())?;
        let mut prefix = [0; 8];
        shard.read_exact(&mut prefix)?;
        let size = u64::from_le_bytes(prefix);
        ensure!(size <= 16 << 20, "oversized safetensors header");
        let mut buf = vec![0; size as usize];
        shard.read_exact(&mut buf)?;
        let header: Value = serde_json::from_slice(&buf)?;
        let entry = &header[name];
        ensure!(
            entry["dtype"] == "BF16" && entry["shape"] == serde_json::json!(shape),
            "unexpected dtype or shape for {name}"
        );
        let begin = entry["data_offsets"][0].as_u64().context("missing tensor offset")?;
        let end = entry["data_offsets"][1].as_u64().context("missing tensor end")?;
        let bytes = shape.iter().product::<u64>() * 2;
        ensure!(
            end.checked_sub(begin) == Some(bytes) && end <= shard.metadata()?.len() - 8 - size,
            "invalid tensor range for {name}"
        );
        Ok(Self { shard, start: 8 + size + begin, rows: bytes / (2 * WIDTH as u64) })
    }

    fn read_rows(&mut self, row: u64, count: usize) -> Result<Vec<u8>> {
        ensure!(row <= self.rows && count as u64 <= self.rows - row, "tile row outside tensor");
        self.shard.seek(SeekFrom::Start(self.start + row * (2 * WIDTH as u64)))?;
        let mut buf = vec![0; count * 2 * WIDTH];
        self.shard.read_exact(&mut buf)?;
        Ok(buf)
    }
}

fn widen(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(2)
        .map(|b| f32::from_bits(u32::from(u16::from_le_bytes([b[0], b[1]])) << 16))
        .collect()
}

fn bf16(x: f32) -> f32 {
    let bits = x.to_bits();
    f32::from_bits(bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) & 0xffff0000)
}

fn append_activation(data: &mut Vec<u8>, x: &[f32], norm: &[f32]) {
    // The pinned arm64 RMSNorm uses sixteen partial sums per group of 256.
    let mut lanes = [0f32; 16];
    for group in x.chunks_exact(256) {
        let mut partial = [0f32; 16];
        for row in group.chunks_exact(16) {
            for (sum, &v) in partial.iter_mut().zip(row) {
                *sum += v * v;
            }
        }
        for (sum, p) in lanes.iter_mut().zip(partial) {
            *sum += p;
        }
    }
    let sum: f32 = (0..4).map(|i| lanes[i] + lanes[i + 4] + lanes[i + 8] + lanes[i + 12]).sum();
    let inv = 1.0 / (sum / WIDTH as f32 + 1e-6).sqrt();
    for (&v, &w) in x.iter().zip(norm) {
        data.extend_from_slice(&bf16(bf16(v * inv) * w).to_le_bytes());
    }
}

fn operands(checkpoint: &Path) -> Result<Vec<u8>> {
    let index =
        serde_json::from_reader(File::open(checkpoint.join("model.safetensors.index.json"))?)?;
    let open = |name, shape: &[u64]| Tensor::open(checkpoint, &index, name, shape);
    let mut embedding = open("model.embed_tokens.weight", &[151936, WIDTH as u64])?;
    let norm =
        widen(&open("model.layers.0.input_layernorm.weight", &[WIDTH as u64])?.read_rows(0, 1)?);
    let mut weights = open("model.layers.0.self_attn.k_proj.weight", &[1024, WIDTH as u64])?;
    let request = crate::Request::load(None)?;
    let (_, consumed) = request.reply_ids.split_last().context("empty reply")?;
    ensure!(request.prompt_ids.len() + consumed.len() == POSITIONS, "unexpected position count");
    let mut data = Vec::with_capacity((4 * POSITIONS + 2 * ROWS) * WIDTH);
    for &id in request.prompt_ids.iter().chain(consumed) {
        let row = embedding.read_rows(u64::from(id), 1)?;
        append_activation(&mut data, &widen(&row), &norm);
    }
    data.extend(weights.read_rows(0, ROWS)?);
    Ok(data)
}

fn stat(csv: &str, prefix: &str) -> Result<u64> {
    let mut matches = csv.lines().filter_map(|line| line.strip_prefix(prefix));
    let field = matches.next().with_context(|| format!("missing {prefix} statistic"))?;
    ensure!(matches.next().is_none(), "duplicate {prefix} statistic");
    Ok(field.split(',').next().unwrap_or_default().parse()?)
}

pub fn compare(checkpoint: &Path, output: &Path, emulator: &Path, elf: &Path) -> Result<()> {
    let data = operands(checkpoint)?;
    let mut gold = hex::decode(GOLD)?;
    gold.resize(256, 0);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(output)
        .with_context(|| format!("expected a fresh directory at {}", output.display()))?;
    for (mode, expected) in FRAMES {
        let mut frame = Vec::with_capacity(32 + data.len());
        frame.extend_from_slice(&(24 + data.len() as u64).to_le_bytes());
        frame.extend_from_slice(b"PCISMPL1");
        for word in [mode, POSITIONS as u32, ROWS as u32, WIDTH as u32] {
            frame.extend_from_slice(&word.to_le_bytes());
        }
        frame.extend_from_slice(&data);
        ensure!(
            hex::encode(Sha256::digest(&frame)) == expected,
            "mode {mode} input differs from the measured tile"
        );
        let base = output.join(format!("mode{mode}"));
        let stdin = base.with_extension("stdin");
        let public = base.with_extension("public");
        let stats = base.with_extension("csv");
        let log_path = base.with_extension("log");
        fs::write(&stdin, frame)?;
        let mut cmd = Command::new(emulator);
        cmd.arg("-X")
            .arg("-e")
            .arg(elf)
            .arg("-i")
            .arg(stdin)
            .arg("-o")
            .arg(&public)
            .arg("--save-stats")
            .arg(&stats);
        let mut log = File::create(&log_path)?;
        writeln!(log, "{cmd:?}")?;
        println!("{cmd:?}");
        let status = cmd
            .stdout(log.try_clone()?)
            .stderr(log.try_clone()?)
            .status()
            .with_context(|| format!("starting {}", emulator.display()))?;
        ensure!(status.success(), "emulator exited with {status}, see {}", log_path.display());
        ensure!(fs::read(public)? == gold, "mode {mode} output differs from the frozen reference");
        let csv = fs::read_to_string(stats)?;
        let steps = stat(&csv, "STEPS,")?;
        let cost = stat(&csv, "COST,TOTAL,")?;
        let result = format!("mode {mode}: {steps} steps, {cost} cost, 2,496/2,496 outputs match");
        println!("{result}");
        writeln!(log, "{result}")?;
    }
    Ok(())
}
