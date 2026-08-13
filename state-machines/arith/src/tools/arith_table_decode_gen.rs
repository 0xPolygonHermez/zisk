//! Writes the canonical decoding of `ArithTable` to `docs/arith_table.txt`, or decodes a table dumped
//! from another revision to stdout.
//!
//! ```text
//! # regenerate the committed decoding (do this whenever the table changes)
//! cargo run --release --bin arith_table_decode_gen
//!
//! # compare against another revision
//! git show <rev>:state-machines/arith/src/arith_table_data.rs > /tmp/old.rs
//! cargo run --release --bin arith_table_decode_gen -- /tmp/old.rs --legacy         > /tmp/old.txt
//! cargo run --release --bin arith_table_decode_gen -- --stdout                     > /tmp/new.txt
//! diff /tmp/old.txt /tmp/new.txt
//!
//! # the same comparison with the two is-zero columns projected out. Empty means the change was
//! # purely notational: same states, same range type for every constrained chunk.
//! cargo run --release --bin arith_table_decode_gen -- /tmp/old.rs --legacy --coarse > /tmp/old_c.txt
//! cargo run --release --bin arith_table_decode_gen -- --stdout --coarse             > /tmp/new_c.txt
//! diff /tmp/old_c.txt /tmp/new_c.txt
//! ```
//!
//! The decoding itself lives in `arith_table_decode.rs` next to this file and is shared with the test that guards the
//! committed file, so the two cannot drift apart.

use std::{fs, path::Path, process::ExitCode};

use zisk_sm_arith::{decode_current_table, decode_table, parse_rows, LEGACY_LAYOUT};

const OUTPUT: &str = "state-machines/arith/docs/arith_table.txt";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let coarse = args.iter().any(|a| a == "--coarse");
    let legacy = args.iter().any(|a| a == "--legacy");
    let stdout = args.iter().any(|a| a == "--stdout");
    let input = args.iter().find(|a| !a.starts_with("--"));

    let decoded = match input {
        Some(path) => {
            let text = match fs::read_to_string(path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("cannot read {path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let rows = parse_rows(&text);
            if rows.is_empty() {
                eprintln!("no [op, flags, range_ab, range_cd] rows found in {path}");
                return ExitCode::FAILURE;
            }
            eprintln!("decoded {} rows from {path}", rows.len());
            if legacy {
                decode_table(&rows, &LEGACY_LAYOUT, coarse)
            } else {
                decode_table(&rows, &zisk_sm_arith::CURRENT_LAYOUT, coarse)
            }
        }
        None => {
            if legacy {
                eprintln!("--legacy only makes sense together with an input file");
                return ExitCode::FAILURE;
            }
            decode_current_table(coarse)
        }
    };

    if stdout || input.is_some() || coarse {
        print!("{decoded}");
        return ExitCode::SUCCESS;
    }

    // Default: refresh the committed decoding. Run from the repository root.
    let path = Path::new(OUTPUT);
    if let Some(dir) = path.parent() {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("cannot create {}: {e}", dir.display());
            return ExitCode::FAILURE;
        }
    }
    match fs::write(path, &decoded) {
        Ok(()) => {
            println!("wrote {OUTPUT}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("cannot write {OUTPUT}: {e} (run from the repository root)");
            ExitCode::FAILURE
        }
    }
}
