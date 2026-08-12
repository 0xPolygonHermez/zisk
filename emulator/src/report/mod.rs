mod parser;
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
