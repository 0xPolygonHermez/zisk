//! Replay the saved transcript, feeding guest activations into private copies of stage inputs.
use crate::asm::AsmServer;
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, BufReader, Read, Seek, SeekFrom, Write},
    path::Path,
    process::Command,
};

const DIM: usize = 2560;
const VOCAB: usize = 151936;
const LAYERS: usize = 36;
const SHARD_LOG2: u32 = 11;
const SHARD_ROWS: usize = 1 << SHARD_LOG2;

fn frame(path: &Path) -> Result<(File, u64)> {
    let mut file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut prefix = [0; 8];
    file.read_exact(&mut prefix)?;
    let size = u64::from_le_bytes(prefix);
    let padded = size.checked_next_multiple_of(8).and_then(|n| n.checked_add(8));
    ensure!(padded == Some(file.metadata()?.len()), "invalid frame length in {}", path.display());
    Ok((file, size))
}

fn embedding_ids(path: &Path, req: &crate::Request, positions: u32) -> Result<[u8; 96]> {
    let (mut input, size) = frame(path)?;
    ensure!(size >= 108 + 4 * u64::from(positions), "embedding token IDs exceed payload");
    let mut magic = [0; 8];
    let mut ids = [0; 96];
    let mut word = [0; 4];
    input.read_exact(&mut magic)?;
    input.read_exact(&mut ids)?;
    input.read_exact(&mut word)?;
    ensure!(
        &magic == b"PCIEMBD1" && u32::from_le_bytes(word) == positions,
        "invalid embedding header"
    );
    ensure!(ids[64..] == req.request_id(&ids[..32], &ids[32..64]), "embedding request ID mismatch");
    for token in req.prompt_ids.iter().chain(req.reply_ids.iter()).take(positions as usize) {
        input.read_exact(&mut word)?;
        ensure!(u32::from_le_bytes(word) == *token, "embedding transcript mismatch");
    }
    Ok(ids)
}

fn with_activations(
    source: &Path,
    dest: &Path,
    header_size: usize,
    prefix: &[u8],
    boundary: &[u8],
) -> Result<()> {
    let (mut input, payload) = frame(source)?;
    let offset = header_size as u64 + boundary.len() as u64;
    ensure!(offset <= payload, "activation exceeds payload in {}", source.display());
    let mut header = vec![0; header_size];
    input.read_exact(&mut header)?;
    ensure!(
        header.starts_with(prefix),
        "wrong stage, request or dimensions in {}",
        source.display()
    );
    let mut out = File::create(dest)?;
    out.write_all(&payload.to_le_bytes())?;
    out.write_all(&header)?;
    out.write_all(boundary)?;
    input.seek(SeekFrom::Start(8 + offset))?;
    io::copy(&mut input, &mut out)?;
    Ok(())
}

/// The steps of one or more executions, with their cost on ziskemu, or their emulation time on
/// the assembly emulator.
#[derive(Serialize)]
struct Counts {
    steps: u64,
    cost: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emulation_us: Option<u64>,
}

fn add(a: Option<u64>, b: Option<u64>, what: &str) -> Result<Option<u64>> {
    match (a, b) {
        (Some(a), Some(b)) => {
            Ok(Some(a.checked_add(b).with_context(|| format!("{what} overflow"))?))
        }
        _ => Ok(None),
    }
}

impl Counts {
    fn zero(asm: bool) -> Self {
        Self { steps: 0, cost: (!asm).then_some(0), emulation_us: asm.then_some(0) }
    }

    fn add(&mut self, counts: &Self) -> Result<()> {
        self.steps = self.steps.checked_add(counts.steps).context("step total overflow")?;
        self.cost = add(self.cost, counts.cost, "cost total")?;
        self.emulation_us = add(self.emulation_us, counts.emulation_us, "time total")?;
        Ok(())
    }

    fn describe(&self) -> String {
        let mut text = format!("{} steps", self.steps);
        if let Some(cost) = self.cost {
            text += &format!(", {cost} cost");
        }
        if let Some(us) = self.emulation_us {
            let rate = if us == 0 { 0.0 } else { self.steps as f64 / us as f64 };
            let time = if us < 1_000_000 {
                format!("{:.1} ms", us as f64 / 1e3)
            } else {
                format!("{:.2} s", us as f64 / 1e6)
            };
            text += &format!(", emulated in {time} ({rate:.1} Msteps/s)");
        }
        text
    }
}

fn stats(path: &Path) -> Result<Counts> {
    let (mut steps, mut cost) = (None, None);
    for line in fs::read_to_string(path)?.lines() {
        let mut fields = line.split(',');
        let slot = match fields.next() {
            Some("STEPS") => &mut steps,
            Some("COST") if fields.next() == Some("TOTAL") => &mut cost,
            _ => continue,
        };
        ensure!(slot.is_none(), "duplicate statistic in {}", path.display());
        *slot = Some(fields.next().context("missing statistic value")?.trim().parse()?);
    }
    Ok(Counts {
        steps: steps.context("missing STEPS")?,
        cost: Some(cost.context("missing COST,TOTAL")?),
        emulation_us: None,
    })
}

#[derive(Serialize)]
struct Measurement {
    stage: String,
    /// The size of the input file the emulator read: the framed stage input.
    input_bytes: u64,
    #[serde(flatten)]
    counts: Counts,
}

/// Runs stage `name`, number `of.0` (from 1) of the `of.1` stages of its kind.
fn execute(
    args: &crate::Run,
    asm: Option<&mut AsmServer>,
    name: &str,
    of: (usize, usize),
    input: &Path,
    bytes: usize,
    measurements: &mut Vec<Measurement>,
) -> Result<()> {
    let base = args.output.join(name);
    let capture = base.with_extension("bin");
    let log = base.with_extension("log");
    let input_bytes = fs::metadata(input)?.len();
    let counts = match asm {
        Some(server) => {
            let metrics = server.execute(name, input, &capture, &log)?;
            Counts { steps: metrics.steps, cost: None, emulation_us: Some(metrics.duration_us) }
        }
        None => ziskemu(args, name, input, &base)?,
    };
    ensure!(
        fs::metadata(&capture)?.len() == bytes as u64,
        "{name}: expected {bytes} capture bytes"
    );
    let size = format!("input {input_bytes} bytes ({:.1} MiB)", input_bytes as f64 / 1048576.0);
    let label = format!("{name} ({}/{})", of.0, of.1);
    writeln!(fs::OpenOptions::new().append(true).open(&log)?, "{label}: {size}")?;
    println!("{label}: {size}, {}", counts.describe());
    measurements.push(Measurement { stage: name.into(), input_bytes, counts });
    Ok(())
}

fn ziskemu(args: &crate::Run, name: &str, input: &Path, base: &Path) -> Result<Counts> {
    let capture = base.with_extension("bin");
    let csv = base.with_extension("csv");
    let log = File::create(base.with_extension("log"))?;
    let elf = args.elf.as_ref().context("--elf is required to run on ziskemu")?;
    let status = Command::new(&args.emulator)
        .args(["-X", "-m", "--max-input-mem", "268435456", "-n", "1000000000000", "-e"])
        .arg(elf)
        .arg("-i")
        .arg(input)
        .arg("-o")
        .arg(base.with_extension("public"))
        .arg("--save-stats")
        .arg(&csv)
        .arg("--dump-guest-buffer")
        .arg(&capture)
        .stdout(log.try_clone()?)
        .stderr(log)
        .status()
        .with_context(|| format!("starting emulator for {name}"))?;
    ensure!(
        status.success(),
        "{name} exited with {status}; see {}",
        base.with_extension("log").display()
    );
    stats(&csv).with_context(|| format!("reading {name} statistics"))
}

fn stage_prefix(magic: &[u8; 8], ids: &[u8; 96], fields: &[u32]) -> Vec<u8> {
    let mut prefix = magic.to_vec();
    prefix.extend(ids);
    for field in fields {
        prefix.extend(field.to_le_bytes());
    }
    prefix
}

// Visit positions first, then vocabulary shards, exactly as in the contiguous FP32 logits matrix.
fn summarize_logits(args: &crate::Run, req: &crate::Request) -> Result<serde_json::Value> {
    let count = req.reply_ids.len() * VOCAB;
    let mut reference = args.reference_logits.as_ref().map(File::open).transpose()?;
    if let Some(file) = &reference {
        ensure!(file.metadata()?.len() == count as u64 * 4, "wrong reference logits size");
    }
    let mut shards: Vec<_> = (0..VOCAB.div_ceil(SHARD_ROWS))
        .map(|i| File::open(args.output.join(format!("head-{i:05}.bin"))).map(BufReader::new))
        .collect::<io::Result<_>>()?;
    let mut row = vec![0; SHARD_ROWS * 4];
    let mut pinned = vec![0; row.len()];
    let mut hash = Sha256::new();
    let mut greedy = Vec::new();
    let mut identical = 0usize;
    let mut max_diff = 0f64;
    for _ in &req.reply_ids {
        let (mut best, mut winner) = (f32::NEG_INFINITY, 0);
        for (shard, file) in shards.iter_mut().enumerate() {
            let first = shard * SHARD_ROWS;
            let bytes = 4 * SHARD_ROWS.min(VOCAB - first);
            file.read_exact(&mut row[..bytes])?;
            hash.update(&row[..bytes]);
            if let Some(reference) = &mut reference {
                reference.read_exact(&mut pinned[..bytes])?;
            }
            for (i, word) in row[..bytes].chunks_exact(4).enumerate() {
                let value = f32::from_le_bytes(word.try_into()?);
                ensure!(value.is_finite(), "non-finite logit");
                if value > best {
                    best = value;
                    winner = (first + i) as u32;
                }
                if reference.is_some() {
                    let target = f32::from_le_bytes(pinned[i * 4..i * 4 + 4].try_into()?);
                    ensure!(target.is_finite(), "non-finite reference logit");
                    identical += usize::from(value.to_bits() == target.to_bits());
                    max_diff = max_diff.max((f64::from(value) - f64::from(target)).abs());
                }
            }
        }
        greedy.push(winner);
    }
    let matching = greedy.iter().zip(&req.reply_ids).filter(|(a, b)| a == b).count();
    let mut result = json!({
        "greedy_ids": greedy, "matching_ids": matching, "logits_sha256": hex::encode(hash.finalize()),
        "total_logits": count, "pinned_ids": req.reply_ids, "all_positions_use_saved_transcript": true
    });
    if reference.is_some() {
        result["identical_logits"] = json!(identical);
        result["max_abs_logit_difference"] = json!(max_diff);
    }
    Ok(result)
}

pub fn run(args: &crate::Run, req: &crate::Request) -> anyhow::Result<()> {
    let predictions = req.reply_ids.len();
    let positions = (req.prompt_ids.len() + predictions - 1) as u32;
    let boundary_size = positions as usize * DIM * 4;
    let embed = args.records.join("embed.stdin");
    let ids = embedding_ids(&embed, req, positions)?;
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&args.output)
        .with_context(|| format!("creating fresh output {}", args.output.display()))?;
    let mut asm =
        args.asm.as_ref().map(|asm| AsmServer::start(asm, &args.output, &embed)).transpose()?;
    let mut measurements = Vec::new();
    execute(args, asm.as_mut(), "embed", (1, 1), &embed, boundary_size, &mut measurements)?;
    let mut boundary = fs::read(args.output.join("embed.bin"))?;
    let tmp = tempfile::tempdir_in(&args.output)?;
    let input = tmp.path().join("stage.stdin");
    for index in 0..LAYERS {
        let name = format!("layer{index:02}");
        let prefix = stage_prefix(b"PCILAYR1", &ids, &[index as u32, positions]);
        with_activations(
            &args.records.join(format!("{name}.v1.stdin")),
            &input,
            176,
            &prefix,
            &boundary,
        )?;
        execute(
            args,
            asm.as_mut(),
            &name,
            (index + 1, LAYERS),
            &input,
            boundary_size,
            &mut measurements,
        )?;
        boundary = fs::read(args.output.join(name).with_extension("bin"))?;
    }
    for (shard, first) in (0..VOCAB).step_by(SHARD_ROWS).enumerate() {
        let name = format!("head-{shard:05}");
        let rows = SHARD_ROWS.min(VOCAB - first);
        let prefix = stage_prefix(
            b"PCIHEAD1",
            &ids,
            &[SHARD_LOG2, shard as u32, positions, predictions as u32],
        );
        with_activations(
            &args.records.join(format!("{name}.stdin")),
            &input,
            120,
            &prefix,
            &boundary,
        )?;
        execute(
            args,
            asm.as_mut(),
            &name,
            (shard + 1, VOCAB.div_ceil(SHARD_ROWS)),
            &input,
            predictions * rows * 4,
            &mut measurements,
        )?;
    }
    // Shut the server down before summarizing
    drop(asm);
    let mut result = summarize_logits(args, req)?;
    let asm = args.asm.is_some();
    let mut total = Counts::zero(asm);
    for group in ["embed", "layer", "head"] {
        let mut part = Counts::zero(asm);
        for item in measurements.iter().filter(|item| item.stage.starts_with(group)) {
            part.add(&item.counts)?;
        }
        total.add(&part)?;
        result["parts"][group] = json!(part);
    }
    result["stages"] = json!(measurements);
    result["total_steps"] = json!(total.steps);
    result["total_cost"] = json!(total.cost);
    if let Some(us) = total.emulation_us {
        result["total_emulation_us"] = json!(us);
    }
    let mut summary = serde_json::to_string_pretty(&result)?;
    summary.push('\n');
    fs::write(args.output.join("summary.json"), summary)?;
    println!("Full request: {}", total.describe());
    println!(
        "Greedy predictions matching the saved reply: {}/{predictions}",
        result["matching_ids"]
    );
    Ok(())
}
