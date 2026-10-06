use std::{env, fs, path::Path, process};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let (html, pdf) = match args.as_slice() {
        [a] => {
            let csv = read(a);
            (ziskemu::report::render_single(&csv), ziskemu::report::render_single_pdf(&csv))
        }
        [a, b] => {
            let (ca, cb) = (read(a), read(b));
            (
                ziskemu::report::render_compare(&ca, base(a), &cb, base(b)),
                ziskemu::report::render_compare_pdf(&ca, base(a), &cb, base(b)),
            )
        }
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

    let html_out = format!("{dir}/report.html");
    if let Err(e) = fs::write(&html_out, html) {
        eprintln!("failed to write {html_out}: {e}");
        process::exit(1);
    }
    println!("HTML report written to: {html_out}");

    let pdf_out = format!("{dir}/report.pdf");
    if let Err(e) = fs::write(&pdf_out, pdf) {
        eprintln!("failed to write {pdf_out}: {e}");
        process::exit(1);
    }
    println!("PDF report written to: {pdf_out}");
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
