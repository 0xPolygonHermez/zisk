//! Build framed stage inputs from the original Qwen3-4B-Instruct-2507 BF16 shards.
use crate::{tagged, Request};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

type Hash = [u8; 32];
const DIM: usize = 2560;
const VOCAB: usize = 151936;
const LAYERS: usize = 36;
const DEPTH: usize = 18;
const SHARD_LOG2: usize = 11;
const SHARD_ROWS: usize = 1 << SHARD_LOG2;
const ROW_BYTES: usize = DIM * 2;
const TIED: &str = "model.embed_tokens.weight";
const NORM: &str = "model.norm.weight";
const CHECKPOINT_ID: &str = "b3a575752a697efff539af25aa7cab9127e13857cbf0e5e66fc08cd6edf0f87e";
const ROW_ROOT: &str = "97a9569bbf996b61e0973f4f171d6dfb00499c2e0abcd76f73ebe58920922571";
const SUITE: &[u8] = concat!(
    "qwen3-4b token-ids greedy-lowest-id; layer-record v2; child-receipt v3; ",
    "mac-arm64-bf16-sdpa-v1; torch cf30153c; transformers 5.15.1; libm 0.2.16; ",
    "rows-v1 depth 18; embed-record v1; head-shard-record v1 receipt v1; ",
    "zisk leaf proofs, publics = 32-byte digest then zeros; bundle v1; head shard log2 11"
)
.as_bytes();

// Lexicographic enrollment order is also the wire order of each layer's weights.
const LAYER_TENSORS: [&str; 11] = [
    "input_layernorm",
    "mlp.down_proj",
    "mlp.gate_proj",
    "mlp.up_proj",
    "post_attention_layernorm",
    "self_attn.k_norm",
    "self_attn.k_proj",
    "self_attn.o_proj",
    "self_attn.q_norm",
    "self_attn.q_proj",
    "self_attn.v_proj",
];

#[derive(Serialize)]
struct Stage {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<u32>,
    tensors: Vec<String>,
}

fn stages() -> Vec<Stage> {
    let mut stages = vec![Stage { kind: "embedding", index: None, tensors: vec![TIED.into()] }];
    for i in 0..LAYERS {
        let tensors =
            LAYER_TENSORS.iter().map(|s| format!("model.layers.{i}.{s}.weight")).collect();
        stages.push(Stage { kind: "layer", index: Some(i as u32), tensors });
    }
    stages.push(Stage { kind: "head", index: None, tensors: vec![NORM.into(), TIED.into()] });
    stages
}

fn profile(t: u32, r: u32) -> [u8; 64] {
    let fields = [
        1,
        DIM as u32,
        9728,
        32,
        8,
        128,
        VOCAB as u32,
        t,
        5_000_000f32.to_bits(),
        1e-6f32.to_bits(),
        32,
        r,
    ];
    let mut out = [0; 64];
    for (slot, field) in out.chunks_exact_mut(4).zip(fields) {
        slot.copy_from_slice(&field.to_le_bytes());
    }
    out
}

struct Tensor {
    path: PathBuf,
    offset: u64,
    len: u64,
    digest: Hash,
}

impl Tensor {
    fn reader(&self, start: u64) -> Result<File> {
        let mut file = File::open(&self.path).with_context(|| self.path.display().to_string())?;
        file.seek(SeekFrom::Start(self.offset + start))?;
        Ok(file)
    }

    fn copy_range(&self, out: &mut File, start: u64, len: u64) -> Result<()> {
        ensure!(start <= self.len && len <= self.len - start, "range outside tensor");
        let copied = io::copy(&mut self.reader(start)?.take(len), out)?;
        ensure!(copied == len, "truncated tensor in {}", self.path.display());
        Ok(())
    }
}

#[derive(Deserialize)]
struct Index {
    weight_map: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Entry {
    dtype: String,
    shape: Vec<u64>,
    data_offsets: [u64; 2],
}

struct Enrollment {
    tensors: BTreeMap<String, Tensor>,
    model: Hash,
}

fn enroll(checkpoint: &Path, stages: &[Stage]) -> Result<Enrollment> {
    let config = fs::read(checkpoint.join("config.json"))?;
    let index = fs::read(checkpoint.join("model.safetensors.index.json"))?;
    let mut files = BTreeMap::from([
        ("config.json".to_owned(), hex::encode(Sha256::digest(config))),
        ("model.safetensors.index.json".to_owned(), hex::encode(Sha256::digest(&index))),
    ]);
    let Index { weight_map } = serde_json::from_slice(&index)?;
    let mut tensors = BTreeMap::new();
    let mut inventory = BTreeMap::new();
    let mut buf = vec![0; 1 << 20];
    for n in 1..=3 {
        let shard = format!("model-{n:05}-of-00003.safetensors");
        let path = checkpoint.join(&shard);
        let mut file = File::open(&path).with_context(|| path.display().to_string())?;
        let mut prefix = [0; 8];
        file.read_exact(&mut prefix)?;
        let size = u64::from_le_bytes(prefix);
        ensure!(size <= 16 << 20, "oversize safetensors header in {shard}");
        let mut header = vec![0; size as usize];
        file.read_exact(&mut header)?;
        let mut whole = Sha256::new();
        whole.update(prefix);
        whole.update(&header);
        let mut raw: BTreeMap<String, Value> = serde_json::from_slice(&header)?;
        raw.remove("__metadata__");
        let mut entries = Vec::with_capacity(raw.len());
        for (name, value) in raw {
            ensure!(weight_map.get(&name) == Some(&shard), "unexpected tensor {name} in {shard}");
            let entry: Entry = serde_json::from_value(value).with_context(|| name.clone())?;
            entries.push((name, entry));
        }
        entries.sort_by_key(|(_, e)| e.data_offsets);
        let mut cursor = 0;
        for (name, entry) in entries {
            let [start, end] = entry.data_offsets;
            let expected = entry.shape.iter().try_fold(2u64, |n, &dim| n.checked_mul(dim));
            ensure!(
                entry.dtype == "BF16"
                    && start == cursor
                    && end >= start
                    && expected == Some(end - start),
                "invalid tensor layout for {name}"
            );
            let len = end - start;
            let mut remaining = len;
            let mut digest = Sha256::new();
            while remaining > 0 {
                let count = remaining.min(buf.len() as u64) as usize;
                file.read_exact(&mut buf[..count]).with_context(|| format!("reading {name}"))?;
                whole.update(&buf[..count]);
                digest.update(&buf[..count]);
                remaining -= count as u64;
            }
            let digest: Hash = digest.finalize().into();
            inventory.insert(
                name.clone(),
                json!({
                    "dtype": entry.dtype, "shape": entry.shape, "sha256": hex::encode(digest)
                }),
            );
            tensors
                .insert(name, Tensor { path: path.clone(), offset: 8 + size + start, len, digest });
            cursor = end;
        }
        ensure!(file.read(&mut buf[..1])? == 0, "trailing data in {shard}");
        files.insert(shard, hex::encode(whole.finalize()));
    }
    ensure!(tensors.keys().eq(weight_map.keys()), "checkpoint index and shards disagree");
    let mut payload = json!({"version": 1, "files": files, "stages": stages, "tensors": inventory});
    // The checkpoint inventory hashes compact JSON with recursively sorted object keys.
    payload.sort_all_objects();
    let mut digest = Sha256::new();
    digest.update(b"PCI/checkpoint-inventory/v1\0");
    digest.update(serde_json::to_vec(&payload)?);
    let checkpoint_id: Hash = digest.finalize().into();
    ensure!(hex::encode(checkpoint_id) == CHECKPOINT_ID, "unexpected publisher checkpoint");

    // Model identity binds the whole-tensor digests, including the tied tensor for both roles.
    let mut roots = Vec::with_capacity(stages.len() * 32);
    for (i, stage) in stages.iter().enumerate() {
        let kind = if i == 0 {
            0
        } else if stage.index.is_some() {
            1
        } else {
            2
        };
        let mut fields = vec![kind];
        fields.extend(stage.index.unwrap_or(0).to_le_bytes());
        fields.extend((stage.tensors.len() as u32).to_le_bytes());
        for name in &stage.tensors {
            fields.extend(tensors[name].digest);
        }
        roots.extend(tagged(b"stage-weights", &[&fields]));
    }
    let model = tagged(
        b"model/qwen3-4b/mac-arm64-bf16-sdpa-v1",
        &[&checkpoint_id, &profile(0, 0), &(stages.len() as u32).to_le_bytes(), &roots],
    );
    Ok(Enrollment { tensors, model })
}

struct RowTree(Vec<Vec<Hash>>);

impl RowTree {
    fn build(tied: &Tensor) -> Result<Self> {
        let mut file = BufReader::new(tied.reader(0)?);
        let mut leaves = Vec::with_capacity(1 << DEPTH);
        let mut row = [0; ROW_BYTES];
        for i in 0..VOCAB as u32 {
            file.read_exact(&mut row)?;
            leaves.push(tagged(b"row", &[&i.to_le_bytes(), &row]));
        }
        leaves.resize(1 << DEPTH, [0; 32]);
        let mut levels = vec![leaves];
        for level in 0..DEPTH {
            let parents = levels[level]
                .chunks_exact(2)
                .map(|pair| tagged(b"row-node", &[&pair[0], &pair[1]]))
                .collect();
            levels.push(parents);
        }
        let root = tagged(
            b"rows-root",
            &[
                &(VOCAB as u32).to_le_bytes(),
                &(ROW_BYTES as u32).to_le_bytes(),
                &(DEPTH as u32).to_le_bytes(),
                &levels[DEPTH][0],
            ],
        );
        ensure!(hex::encode(root) == ROW_ROOT, "unexpected tied embedding row commitment");
        Ok(Self(levels))
    }

    fn write_opening(&self, file: &mut File, level: usize, mut index: usize) -> Result<()> {
        for nodes in &self.0[level..DEPTH] {
            file.write_all(&nodes[index ^ 1])?;
            index >>= 1;
        }
        Ok(())
    }
}

fn frame(
    path: PathBuf,
    magic: &[u8; 8],
    ids: &[Hash; 3],
    body: impl FnOnce(&mut File) -> Result<()>,
) -> Result<()> {
    let mut f = File::create_new(&path).with_context(|| path.display().to_string())?;
    f.write_all(&[0; 8])?;
    f.write_all(magic)?;
    for id in ids {
        f.write_all(id)?;
    }
    body(&mut f).with_context(|| format!("writing {}", path.display()))?;
    let size = f.stream_position()? - 8;
    f.write_all(&[0; 8][..(size.wrapping_neg() % 8) as usize])?;
    f.rewind()?;
    f.write_all(&size.to_le_bytes())?;
    Ok(())
}

/// Prepare the 112 stage templates, publishing the directory only after every frame is complete.
pub fn prepare(checkpoint: &Path, records: &Path, req: &Request) -> Result<()> {
    ensure!(!records.try_exists()?, "choose a fresh records directory: {}", records.display());
    ensure!(!req.prompt_ids.is_empty() && !req.reply_ids.is_empty(), "empty prompt or reply");
    ensure!(
        req.prompt_ids.iter().chain(&req.reply_ids).all(|&id| id < VOCAB as u32),
        "invalid token ID"
    );
    ensure!(
        req.reply_ids.len() <= req.max_reply as usize
            && req.prompt_ids.len() <= 40960usize.saturating_sub(req.max_reply as usize),
        "invalid reply limit"
    );
    ensure!(hex::decode(&req.nonce)?.len() == 32, "expected a 32-byte nonce");

    let stages = stages();
    let Enrollment { tensors, model } = enroll(checkpoint, &stages)?;
    let tied = &tensors[TIED];
    let tree = RowTree::build(tied)?;
    let suite = tagged(b"suite", &[SUITE]);
    let ids = [model, suite, req.request_id(&model, &suite)];
    let consumed = req.prompt_ids.iter().chain(&req.reply_ids[..req.reply_ids.len() - 1]);
    let t = (req.prompt_ids.len() + req.reply_ids.len() - 1) as u32;
    let r = req.reply_ids.len() as u32;
    let activation_bytes = i64::from(t) * DIM as i64 * 4;

    let parent = records.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let scratch = tempfile::Builder::new().prefix(".qwen-prepare-").tempdir_in(parent)?;
    let dir = scratch.path().join("records");
    fs::create_dir(&dir)?;
    frame(dir.join("embed.stdin"), b"PCIEMBD1", &ids, |f| {
        f.write_all(&t.to_le_bytes())?;
        for id in consumed.clone() {
            f.write_all(&id.to_le_bytes())?;
        }
        for &id in consumed {
            tied.copy_range(f, u64::from(id) * ROW_BYTES as u64, ROW_BYTES as u64)?;
            tree.write_opening(f, 0, id as usize)?;
        }
        Ok(())
    })?;
    for (i, stage) in stages[1..=LAYERS].iter().enumerate() {
        frame(dir.join(format!("layer{i:02}.v1.stdin")), b"PCILAYR1", &ids, |f| {
            f.write_all(&(i as u32).to_le_bytes())?;
            f.write_all(&t.to_le_bytes())?;
            f.write_all(&profile(t, r))?;
            f.seek(SeekFrom::Current(activation_bytes))?;
            for name in &stage.tensors {
                let tensor = &tensors[name];
                tensor.copy_range(f, 0, tensor.len)?;
            }
            Ok(())
        })?;
    }
    let norm = &tensors[NORM];
    for (i, first) in (0..VOCAB).step_by(SHARD_ROWS).enumerate() {
        frame(dir.join(format!("head-{i:05}.stdin")), b"PCIHEAD1", &ids, |f| {
            for field in [SHARD_LOG2 as u32, i as u32, t, r] {
                f.write_all(&field.to_le_bytes())?;
            }
            f.seek(SeekFrom::Current(activation_bytes))?;
            norm.copy_range(f, 0, norm.len)?;
            let rows = SHARD_ROWS.min(VOCAB - first);
            tied.copy_range(f, (first * ROW_BYTES) as u64, (rows * ROW_BYTES) as u64)?;
            tree.write_opening(f, SHARD_LOG2, i)
        })?;
    }
    fs::rename(dir, records).with_context(|| format!("publishing {}", records.display()))
}
