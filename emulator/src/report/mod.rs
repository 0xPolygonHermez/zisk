mod parser;
mod pdf;
mod render;

pub fn render_single(csv: &str) -> String {
    let report = parser::parse(csv);
    render::single(&report)
}

pub fn render_compare(csv_a: &str, name_a: &str, csv_b: &str, name_b: &str) -> String {
    let a = parser::parse(csv_a);
    let b = parser::parse(csv_b);
    render::compare(&a, &b, name_a, name_b)
}

pub fn render_single_pdf(csv: &str) -> Vec<u8> {
    let report = parser::parse(csv);
    pdf::single(&report)
}

pub fn render_compare_pdf(csv_a: &str, name_a: &str, csv_b: &str, name_b: &str) -> Vec<u8> {
    let a = parser::parse(csv_a);
    let b = parser::parse(csv_b);
    pdf::compare(&a, &b, name_a, name_b)
}
