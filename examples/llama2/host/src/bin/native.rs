//! Asks for a prompt, generates a continuation natively, and prints it.
//!
//! ```text
//! cargo run --release --bin llama2-native -- [OPTIONS]
//! ```

use std::time::Instant;

use llama2_common::forward::{MatmulMode, dot_soft};
use llama2_common::generate::generate_with;
use llama2_common::model::Model;
use llama2_host::{
    Result, TextPrinter, gen_params, load, parse_options, print_model_info, prompts,
};

fn main() -> Result<()> {
    // Command-line options (see `USAGE` in the host library): which model,
    // the prompt, how many tokens to generate, and how to pick them.
    let o = parse_options(&[])?;

    // Load the model and its tokenizer.
    //
    // A model is a long list of numbers, its weights, learned during
    // training. The llama2.c checkpoint stores them as 32-bit floats; `load`
    // converts them to 8-bit integers plus one float scale per group of
    // values ("quantization"), which is what the ZisK guest uses. The result
    // is a single byte buffer, the model blob.
    //
    // The tokenizer converts between text and tokens. A token is a piece of
    // text, a word or part of one, identified by a number: the model only
    // reads and writes such numbers.
    let l = load(&o)?;
    print_model_info(&o, &l);

    // Interpret the blob as a model: its sizes and its weight matrices. The
    // int8 weights are not copied, only referenced.
    let model = Model::parse(&l.blob);

    // One prompt from `--prompt`, or each line the user types until an empty
    // one.
    for text in prompts(&o) {
        // The request: the prompt as tokens (starting with the BOS token,
        // "beginning of a story"), how many tokens to add, and the sampling
        // settings (temperature 0 always picks the most likely next token).
        let params = gen_params(&o, &l.tokenizer, &text);

        // Prints the prompt now, and then each new token as text as soon as
        // it is generated.
        let mut printer = TextPrinter::start(&l.tokenizer, &text, &params.prompt);
        let start = Instant::now();

        // How the matrix multiplications, almost all of the work, are
        // computed: the plain-Rust integer dot product, and the scales in
        // `f32` or fixed point (`--scales`). The ZisK guest can use the
        // `dot_i8` precompile instead of `dot_soft`; natively there is none.
        let mode = MatmulMode { dot: dot_soft, scales: o.scales };

        // The generation loop. For each position it runs the model once
        // ("forward pass") on the current token, which gives a score for
        // every token of the vocabulary as the possible next one. During the
        // prompt the next token is simply the next prompt token; after it,
        // the next token is chosen from those scores and fed back in. It
        // stops after `--max-new` tokens, at a BOS (a new story would start)
        // or when the model's context is full. Each generated token is passed
        // to the closure, which prints it, and all of them are returned.
        let tokens = generate_with(&model, &params, mode, |t| printer.token(t));
        let secs = start.elapsed().as_secs_f64();
        println!();
        println!(
            "[native: {} prompt + {} generated tokens in {:.3} s, {:.1} tokens/s]",
            params.prompt.len(),
            tokens.len(),
            secs,
            (params.prompt.len() + tokens.len()) as f64 / secs
        );
    }
    Ok(())
}
