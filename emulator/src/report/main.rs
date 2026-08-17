use std::{env, fs, path::Path, process};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let html = match args.as_slice() {
        [a] => ziskemu::report::render_single(&read(a)),
        [a, b] => ziskemu::report::render_compare(&read(a), base(a), &read(b), base(b)),
        _ => {
            eprintln!("usage: report <stats.csv> [other.csv]");
            process::exit(1);
        }
    };

    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/report");
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("failed to create {dir}: {e}");
        process::exit(1);
    }
    let out = format!("{dir}/report.html");
    if let Err(e) = fs::write(&out, html) {
        eprintln!("failed to write {out}: {e}");
        process::exit(1);
    }
    println!("HTML report written to: {out}");
}

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        process::exit(1);
    })
}

fn base(path: &str) -> &str {
    Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path)
}
