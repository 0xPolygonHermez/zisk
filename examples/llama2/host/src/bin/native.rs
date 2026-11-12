//! Asks for a prompt, generates a continuation natively, and prints it.
//!
//! ```text
//! cargo run --release --bin llama2-native -- [OPTIONS]
//! ```

use std::time::Instant;

use llama2_common::generate::generate;
use llama2_common::model::Model;
use llama2_host::{
    Result, TextPrinter, gen_params, load, parse_options, print_model_info, prompts,
};

fn main() -> Result<()> {
    let o = parse_options(&[])?;
    let l = load(&o)?;
    print_model_info(&o, &l);
    let model = Model::parse(&l.blob);

    for text in prompts(&o) {
        let params = gen_params(&o, &l.tokenizer, &text);
        let mut printer = TextPrinter::start(&l.tokenizer, &text, &params.prompt);
        let start = Instant::now();
        let tokens = generate(&model, &params, |t| printer.token(t));
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
