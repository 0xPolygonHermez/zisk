//! Code shared by `llama2-native` and `llama2-zisk`: command-line options,
//! loading the model and tokenizer, reading prompts, and printing text.

use std::error::Error;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use llama2_common::checkpoint::{Config, Quantization, quantize_checkpoint_with, read_config};
use llama2_common::formats::Format;
use llama2_common::forward::Scales;
use llama2_common::params::GenParams;
use llama2_common::tokenizer::Tokenizer;
use sha2::{Digest, Sha256};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub const USAGE: &str = "\
Options:
  --model PATH         llama2.c checkpoint (default: models/stories260K.bin)
  --tokenizer PATH     tokenizer file (default: tok512.bin for a 512-token
                       vocabulary, else tokenizer.bin, next to the model)
  --prompt TEXT        generate once for TEXT instead of asking
  --max-new N          tokens to generate (default: 128)
  --temperature T      0 = always the most likely token (default: 0)
  --seed S             sampler seed when temperature > 0 (default: 42)
  --scales float|fixed how matrix multiplications apply the group scales:
                       f32 as llama2.c, or fixed point (default: float)
  --group-size N       quantize in groups of at most N values, a multiple of 8
                       (16 with e2m1) that divides the row (default:
                       llama2.c's, a power of two up to 64)
  --weights FORMAT     format of the weights: int8 (default, as llama2.c),
                       e4m3 (OCP FP8), e4m3fnuz (AMD FP8), e2m1 (FP4), bf16,
                       mxfp4 or nvfp4 (FP4 with block scales)
  --activations FORMAT format the activations are quantized to before each
                       matrix multiplication (default: the weights' format,
                       bf16 for mxfp4 and nvfp4; bf16 goes only with bf16,
                       mxfp4 and nvfp4)";

pub struct Options {
    pub model: PathBuf,
    pub tokenizer: Option<PathBuf>,
    pub prompt: Option<String>,
    pub max_new: u32,
    pub temperature: f32,
    pub seed: u64,
    pub scales: Scales,
    pub max_group: Option<usize>,
    pub weights: Format,
    pub activations: Option<Format>,
    /// Options this program knows and the shared parser does not.
    pub extra: Vec<(String, String)>,
}

/// Parses the shared options. `extra` lists additional `--name VALUE`
/// options the calling program accepts.
pub fn parse_options(extra: &[&str]) -> Result<Options> {
    let models = Path::new(env!("CARGO_MANIFEST_DIR")).join("../models");
    let mut o = Options {
        model: models.join("stories260K.bin"),
        tokenizer: None,
        prompt: None,
        max_new: 128,
        temperature: 0.0,
        seed: 42,
        scales: Scales::Float,
        max_group: None,
        weights: Format::Int8,
        activations: None,
        extra: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(name) = args.next() {
        if name == "--help" || name == "-h" {
            println!("{USAGE}");
            std::process::exit(0);
        }
        let value = args.next().ok_or_else(|| format!("{name} needs a value\n{USAGE}"))?;
        match name.as_str() {
            "--model" => o.model = value.into(),
            "--tokenizer" => o.tokenizer = Some(value.into()),
            "--prompt" => o.prompt = Some(value),
            "--max-new" => o.max_new = value.parse()?,
            "--temperature" => o.temperature = value.parse()?,
            "--seed" => o.seed = value.parse()?,
            "--group-size" => o.max_group = Some(value.parse()?),
            "--weights" => o.weights = parse_format(&name, &value)?,
            "--activations" => o.activations = Some(parse_format(&name, &value)?),
            "--scales" => {
                o.scales = match value.as_str() {
                    "float" => Scales::Float,
                    "fixed" => Scales::Fixed,
                    v => return Err(format!("--scales must be float or fixed, not {v}").into()),
                }
            }
            n if extra.contains(&n) => o.extra.push((name, value)),
            _ => return Err(format!("unknown option {name}\n{USAGE}").into()),
        }
    }
    if !(o.temperature >= 0.0) {
        return Err("--temperature must be 0 or more".into());
    }
    Ok(o)
}

fn parse_format(name: &str, value: &str) -> Result<Format> {
    Format::from_name(value).ok_or_else(|| {
        format!("{name} must be int8, e4m3, e4m3fnuz, e2m1, bf16, mxfp4 or nvfp4, not {value}")
            .into()
    })
}

impl Options {
    /// The model's quantization: formats and group sizes.
    pub fn quantization(&self) -> Quantization {
        Quantization {
            weights: self.weights,
            activations: self.activations.unwrap_or(if self.weights.uses_bf16() {
                Format::Bf16
            } else {
                self.weights
            }),
            max_group: self.max_group,
        }
    }
}

/// The quantized model blob and its tokenizer.
pub struct Loaded {
    pub blob: Vec<u8>,
    pub config: Config,
    pub tokenizer: Tokenizer,
}

pub fn load(o: &Options) -> Result<Loaded> {
    let read = |p: &Path| {
        std::fs::read(p).map_err(|e| {
            format!("cannot read {}: {e} (see README.md to download the models)", p.display())
        })
    };
    let checkpoint = read(&o.model)?;
    let (config, _) = read_config(&checkpoint)?;
    let blob = quantize_checkpoint_with(&checkpoint, o.quantization())?;
    let tokenizer_path = o.tokenizer.clone().unwrap_or_else(|| {
        let name = if config.vocab_size == 512 { "tok512.bin" } else { "tokenizer.bin" };
        o.model.with_file_name(name)
    });
    let tokenizer = Tokenizer::parse(&read(&tokenizer_path)?, config.vocab_size)?;
    Ok(Loaded { blob, config, tokenizer })
}

/// The prompts to run: `--prompt` once, or lines typed by the user until an
/// empty line or end of input.
pub fn prompts(o: &Options) -> impl Iterator<Item = String> {
    let once = o.prompt.clone();
    let interactive = once.is_none();
    let mut done = false;
    std::iter::from_fn(move || {
        if done {
            return None;
        }
        if !interactive {
            done = true;
            return once.clone();
        }
        print!("\nPrompt (empty to quit)> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        let read = std::io::stdin().lock().read_line(&mut line).unwrap_or(0);
        let line = line.trim_end_matches(['\n', '\r']).to_string();
        if read == 0 || line.is_empty() {
            done = true;
            return None;
        }
        Some(line)
    })
}

pub fn gen_params(o: &Options, tokenizer: &Tokenizer, text: &str) -> GenParams {
    GenParams {
        prompt: tokenizer.encode(text),
        max_new: o.max_new,
        temperature: o.temperature,
        seed: o.seed,
    }
}

/// Prints generated tokens as text, as they arrive.
pub struct TextPrinter<'a> {
    tokenizer: &'a Tokenizer,
    prev: u32,
}

impl<'a> TextPrinter<'a> {
    /// Prints the prompt; generated tokens follow it.
    pub fn start(tokenizer: &'a Tokenizer, text: &str, prompt: &[u32]) -> Self {
        print!("{text}");
        std::io::stdout().flush().ok();
        TextPrinter { tokenizer, prev: *prompt.last().unwrap() }
    }

    pub fn token(&mut self, token: u32) {
        let mut out = std::io::stdout().lock();
        out.write_all(&self.tokenizer.decode(self.prev, token)).ok();
        out.flush().ok();
        self.prev = token;
    }
}

pub fn print_model_info(o: &Options, l: &Loaded) {
    let c = &l.config;
    let model = llama2_common::model::Model::parse(&l.blob);
    println!(
        "Model {}: dim {}, {} layers, {} heads, vocabulary {}, context {}",
        o.model.display(),
        c.dim,
        c.n_layers,
        c.n_heads,
        c.vocab_size,
        c.seq_len
    );
    println!(
        "  {} {} weights, {} activations, {} MACs per token in matrix multiplications, \
         groups of {} (dim) and {} (hidden_dim)",
        model.params(),
        model.weights().name(),
        model.activations.name(),
        model.matmul_macs_per_token(),
        model.wq.gs,
        model.w2.gs
    );
}

/// The digest the guest commits: `sha256(params || generated tokens)`.
pub fn output_digest(params: &GenParams, tokens: &[u32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(params.to_bytes());
    for t in tokens {
        h.update(t.to_le_bytes());
    }
    h.finalize().into()
}
