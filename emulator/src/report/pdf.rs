use super::parser::{FropRow, MemRow, OpRow, Report};
use super::render::fmt_num;
use std::collections::{HashMap, HashSet};

const MARGIN: f64 = 40.0;
const TOP_MARGIN: f64 = 40.0;
const BOTTOM_MARGIN: f64 = 40.0;
const A4_SHORT: f64 = 595.0;
const A4_LONG: f64 = 842.0;

const BODY: f64 = 8.0;
const SECTION: f64 = 10.0;
const TITLE: f64 = 15.0;
const LINE_FACTOR: f64 = 1.4;
const RULE_H: f64 = 9.0;

const BLACK: [f64; 3] = [0.0, 0.0, 0.0];
const GREEN_TITLE: [f64; 3] = [0.16, 0.40, 0.06];
const GOOD: [f64; 3] = [0.0, 0.45, 0.0];
const BAD: [f64; 3] = [0.72, 0.0, 0.0];
const ZEBRA: [f64; 3] = [0.955, 0.955, 0.955];
const HEADER_BG: [f64; 3] = [0.88, 0.94, 0.82];
const RULE_COLOR: [f64; 3] = [0.70, 0.70, 0.70];

#[derive(Clone, Copy)]
enum FontKind {
    Body,
    BodyBold,
    Title,
}

struct Seg {
    text: String,
    col: usize,
    color: [f64; 3],
}

struct Line {
    segs: Vec<Seg>,
    font: FontKind,
    size: f64,
    band: Option<[f64; 3]>,
    band_cols: usize,
    col_seps: Vec<f64>,
    is_header: bool,
}

enum Item {
    Line(Line),
    Rule,
    Gap(f64),
    KeepStart(f64),
}

#[derive(Clone, Copy)]
struct RowStyle {
    bold: bool,
    tail: Option<[f64; 3]>,
}

fn seg(text: String, col: usize, color: [f64; 3]) -> Seg {
    Seg { text, col, color }
}

fn line(
    segs: Vec<Seg>,
    font: FontKind,
    size: f64,
    band: Option<[f64; 3]>,
    band_cols: usize,
) -> Item {
    Item::Line(Line { segs, font, size, band, band_cols, col_seps: Vec::new(), is_header: false })
}

fn body_line(s: String) -> Item {
    line(vec![seg(s, 0, BLACK)], FontKind::Body, BODY, None, 0)
}

pub fn single(r: &Report) -> Vec<u8> {
    let mut l: Vec<Item> = Vec::new();
    heading(&mut l, "ZisK stats report");

    l.push(body_line(kv("STEPS", &fmt_num(r.steps))));
    if let Some(c) = find(r, "TOTAL") {
        l.push(body_line(kv("TOTAL COST", &fmt_num(c))));
    }
    if let Some(c) = r.cost.iter().find(|c| c.label == "FROPS") {
        l.push(body_line(format!("{} ({:.2}%)", kv("FROPS", &fmt_num(c.cost)), c.pct)));
    }
    if r.ram_usage.used > 0 {
        l.push(body_line(format!(
            "{} ({:.2}%)",
            kv("RAM USAGE", &fmt_num(r.ram_usage.used)),
            r.ram_usage.pct
        )));
    }
    l.push(body_line(format!(
        "{} ({:.2}%)",
        kv("ROM USAGE", &fmt_num(r.rom_usage.used)),
        r.rom_usage.pct
    )));

    let cost_rows: Vec<Vec<String>> = r
        .cost
        .iter()
        .filter(|c| c.label != "FROPS")
        .map(|c| vec![c.label.clone(), fmt_num(c.cost), format!("{:.2}%", c.pct)])
        .collect();
    let cost_styles: Vec<RowStyle> = r
        .cost
        .iter()
        .filter(|c| c.label != "FROPS")
        .map(|c| RowStyle { bold: matches!(c.label.as_str(), "TOTAL" | "VARIABLE"), tail: None })
        .collect();
    table(
        &mut l,
        "COST DISTRIBUTION",
        &["", "COST", "%"],
        &[false, true, true],
        &cost_rows,
        Some(&cost_styles),
        None,
    );

    section_op(&mut l, "COST BY BASE OPCODE", &r.op_base);
    section_op(&mut l, "COST BY PRECOMPILED OPCODE", &r.precompiles);
    section_frop(&mut l, "FROPS BY OPCODE", &r.frop);
    section_mem(&mut l, "MEM COST BY TYPE", &r.mem_by_type);
    section_mem(&mut l, "MEM TOTALS", &r.mem_totals);
    section_mem(&mut l, "DETAILED MEM", &r.detailed_mem);
    section_mem(&mut l, "DETAILED MEM (FULL)", &r.detailed_mem_full);

    let mut sections: Vec<(Vec<Item>, f64, f64, bool)> = vec![(l, A4_SHORT, A4_LONG, false)];

    if !r.mem_offsets.rows.is_empty() {
        let mut ml: Vec<Item> = Vec::new();
        let mut headers: Vec<String> = vec![String::new()];
        headers.extend(r.mem_offsets.cols.iter().cloned());
        let hdr: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
        let mut aligns = vec![false];
        aligns.extend(std::iter::repeat(true).take(r.mem_offsets.cols.len()));
        let rows: Vec<Vec<String>> = r
            .mem_offsets
            .rows
            .iter()
            .map(|(name, vals)| {
                let mut row = vec![name.clone()];
                row.extend(vals.iter().map(|v| fmt_num(*v)));
                row
            })
            .collect();
        table(&mut ml, "MEM OFFSETS", &hdr, &aligns, &rows, None, None);
        sections.push((ml, A4_LONG, A4_SHORT, true));
    }

    render_sections(&sections)
}

fn section_op(l: &mut Vec<Item>, title: &str, rows: &[OpRow]) {
    if rows.is_empty() {
        return;
    }
    let mut v: Vec<&OpRow> = rows.iter().collect();
    v.sort_by_key(|r| std::cmp::Reverse(r.count));
    let out: Vec<Vec<String>> = v
        .iter()
        .map(|r| {
            vec![
                r.name.clone(),
                fmt_num(r.count),
                format!("{:.2}%", r.count_pct),
                fmt_num(r.cost),
                format!("{:.2}%", r.cost_pct),
            ]
        })
        .collect();
    table(
        l,
        title,
        &["OPCODE", "COUNT", "%", "COST", "%"],
        &[false, true, true, true, true],
        &out,
        None,
        None,
    );
}

fn section_frop(l: &mut Vec<Item>, title: &str, rows: &[FropRow]) {
    if rows.is_empty() {
        return;
    }
    let mut v: Vec<&FropRow> = rows.iter().collect();
    v.sort_by_key(|r| std::cmp::Reverse(r.count));
    let out: Vec<Vec<String>> = v
        .iter()
        .map(|r| {
            vec![
                r.name.clone(),
                fmt_num(r.count),
                format!("{:.2}%", r.hit_pct),
                fmt_num(r.cost),
                format!("{:.2}%", r.cost_pct),
            ]
        })
        .collect();
    table(
        l,
        title,
        &["FROP", "COUNT", "HIT%", "COST", "%"],
        &[false, true, true, true, true],
        &out,
        None,
        None,
    );
}

fn section_mem(l: &mut Vec<Item>, title: &str, rows: &[MemRow]) {
    if rows.is_empty() {
        return;
    }
    let mut v: Vec<&MemRow> = rows.iter().collect();
    v.sort_by_key(|r| std::cmp::Reverse(r.count));
    let out: Vec<Vec<String>> = v
        .iter()
        .map(|r| {
            vec![
                r.label.clone(),
                fmt_num(r.count),
                format!("{:.2}%", r.count_pct),
                fmt_num(r.cost),
                format!("{:.2}%", r.cost_pct),
            ]
        })
        .collect();
    table(
        l,
        title,
        &["TYPE", "COUNT", "%", "COST", "%"],
        &[false, true, true, true, true],
        &out,
        None,
        None,
    );
}

pub fn compare(a: &Report, b: &Report, name_a: &str, name_b: &str) -> Vec<u8> {
    let mut l: Vec<Item> = Vec::new();
    heading(&mut l, "ZisK stats report - comparison");
    l.push(body_line(format!("A = {} (baseline)   B = {}   Change = B - A", name_a, name_b)));
    l.push(line(
        vec![
            seg("green = lower cost   ".to_string(), 0, GOOD),
            seg("red = higher".to_string(), 21, BAD),
        ],
        FontKind::Body,
        BODY,
        None,
        0,
    ));
    l.push(Item::Gap(6.0));

    cmp_kv(&mut l, "STEPS", Some(a.steps), Some(b.steps));
    cmp_kv(&mut l, "TOTAL COST", find(a, "TOTAL"), find(b, "TOTAL"));
    cmp_kv(&mut l, "FROPS", find(a, "FROPS"), find(b, "FROPS"));

    cmp_table(
        &mut l,
        "COST DISTRIBUTION",
        a.cost.iter().filter(|c| c.label != "FROPS").map(|c| (c.label.clone(), c.cost)).collect(),
        b.cost.iter().filter(|c| c.label != "FROPS").map(|c| (c.label.clone(), c.cost)).collect(),
        false,
    );
    cmp_table(&mut l, "COST BY BASE OPCODE", op_cost(&a.op_base), op_cost(&b.op_base), true);
    cmp_table(
        &mut l,
        "COST BY PRECOMPILED OPCODE",
        op_cost(&a.precompiles),
        op_cost(&b.precompiles),
        true,
    );
    cmp_table(&mut l, "FROPS BY OPCODE", frop_cost(&a.frop), frop_cost(&b.frop), true);
    cmp_table(&mut l, "MEM COST BY TYPE", mem_cost(&a.mem_by_type), mem_cost(&b.mem_by_type), true);
    cmp_table(&mut l, "MEM TOTALS", mem_cost(&a.mem_totals), mem_cost(&b.mem_totals), true);
    cmp_table(&mut l, "DETAILED MEM", mem_cost(&a.detailed_mem), mem_cost(&b.detailed_mem), true);
    cmp_table(
        &mut l,
        "DETAILED MEM (FULL)",
        mem_cost(&a.detailed_mem_full),
        mem_cost(&b.detailed_mem_full),
        true,
    );

    render_sections(&[(l, A4_LONG, A4_SHORT, false)])
}

fn op_cost(rows: &[OpRow]) -> Vec<(String, u64)> {
    rows.iter().map(|r| (r.name.clone(), r.cost)).collect()
}
fn frop_cost(rows: &[FropRow]) -> Vec<(String, u64)> {
    rows.iter().map(|r| (r.name.clone(), r.cost)).collect()
}
fn mem_cost(rows: &[MemRow]) -> Vec<(String, u64)> {
    rows.iter().map(|r| (r.label.clone(), r.cost)).collect()
}

fn cmp_table(
    l: &mut Vec<Item>,
    title: &str,
    a: Vec<(String, u64)>,
    b: Vec<(String, u64)>,
    sort_desc: bool,
) {
    let am: HashMap<String, u64> = a.iter().cloned().collect();
    let bm: HashMap<String, u64> = b.iter().cloned().collect();
    let mut order: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (n, _) in a.iter().chain(b.iter()) {
        if seen.insert(n.clone()) {
            order.push(n.clone());
        }
    }
    if order.is_empty() {
        return;
    }
    if sort_desc {
        order.sort_by_key(|n| {
            std::cmp::Reverse(am.get(n).copied().unwrap_or(0).max(bm.get(n).copied().unwrap_or(0)))
        });
    }
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(order.len());
    let mut styles: Vec<RowStyle> = Vec::with_capacity(order.len());
    for n in &order {
        let av = am.get(n).copied();
        let bv = bm.get(n).copied();
        let (d, pct) = delta(av, bv);
        rows.push(vec![
            n.clone(),
            av.map(fmt_num).unwrap_or_else(|| "-".to_string()),
            bv.map(fmt_num).unwrap_or_else(|| "-".to_string()),
            d,
            pct,
        ]);
        styles.push(RowStyle {
            bold: matches!(n.as_str(), "TOTAL" | "VARIABLE"),
            tail: cmp_dir(av, bv),
        });
    }
    table(
        l,
        title,
        &["", "A", "B", "CHANGE", "%"],
        &[false, true, true, true, true],
        &rows,
        Some(&styles),
        Some(3),
    );
}

fn cmp_kv(l: &mut Vec<Item>, label: &str, a: Option<u64>, b: Option<u64>) {
    let (d, pct) = delta(a, b);
    let prefix = format!(
        "{:<14}A {:>16}   B {:>16}   ",
        label,
        a.map(fmt_num).unwrap_or_else(|| "-".to_string()),
        b.map(fmt_num).unwrap_or_else(|| "-".to_string()),
    );
    let tail = format!("{} ({})", d, pct);
    let col = prefix.chars().count();
    let color = cmp_dir(a, b).unwrap_or(BLACK);
    l.push(line(vec![seg(prefix, 0, BLACK), seg(tail, col, color)], FontKind::Body, BODY, None, 0));
}

fn cmp_dir(a: Option<u64>, b: Option<u64>) -> Option<[f64; 3]> {
    let av = a.unwrap_or(0);
    let bv = b.unwrap_or(0);
    if bv < av {
        Some(GOOD)
    } else if bv > av {
        Some(BAD)
    } else {
        None
    }
}

fn delta(a: Option<u64>, b: Option<u64>) -> (String, String) {
    let av = a.unwrap_or(0) as i128;
    let bv = b.unwrap_or(0) as i128;
    let d = bv - av;
    let sign = if d > 0 {
        "+"
    } else if d < 0 {
        "-"
    } else {
        ""
    };
    let delta = format!("{}{}", sign, fmt_num(d.unsigned_abs() as u64));
    let pct = match (a.unwrap_or(0), b.unwrap_or(0)) {
        (0, 0) => "0.00%".to_string(),
        (0, _) => "new".to_string(),
        (av, _) => {
            let p = d as f64 / av as f64 * 100.0;
            let s = if p > 0.0 {
                "+"
            } else if p < 0.0 {
                "-"
            } else {
                ""
            };
            format!("{}{:.2}%", s, p.abs())
        }
    };
    (delta, pct)
}

fn find(r: &Report, label: &str) -> Option<u64> {
    r.cost.iter().find(|c| c.label == label).map(|c| c.cost)
}

fn heading(l: &mut Vec<Item>, title: &str) {
    l.push(line(vec![seg(title.to_string(), 0, GREEN_TITLE)], FontKind::Title, TITLE, None, 0));
    l.push(Item::Rule);
    l.push(Item::Gap(6.0));
}

fn kv(label: &str, value: &str) -> String {
    format!("{:<14}{:>18}", label, value)
}

fn table(
    l: &mut Vec<Item>,
    title: &str,
    headers: &[&str],
    aligns: &[bool],
    rows: &[Vec<String>],
    row_styles: Option<&[RowStyle]>,
    delta_col: Option<usize>,
) {
    if rows.is_empty() {
        return;
    }
    let ncol = headers.len();
    let mut w: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (c, wc) in w.iter_mut().enumerate() {
            let len = row.get(c).map(|s| s.chars().count()).unwrap_or(0);
            *wc = (*wc).max(len);
        }
    }
    let table_cols = w.iter().sum::<usize>() + 2 * ncol.saturating_sub(1);
    let seps: Vec<f64> =
        (1..ncol).map(|c| (w[..c].iter().sum::<usize>() + 2 * c) as f64 - 1.0).collect();
    let row_h = BODY * LINE_FACTOR;
    let reserve = 6.0 + RULE_H + SECTION * LINE_FACTOR + row_h + rows.len().min(2) as f64 * row_h;
    l.push(Item::KeepStart(reserve));
    l.push(Item::Gap(6.0));
    l.push(Item::Rule);
    l.push(line(vec![seg(title.to_string(), 0, GREEN_TITLE)], FontKind::Title, SECTION, None, 0));

    let hdr = render_row(headers.iter().copied(), &w, aligns);
    l.push(Item::Line(Line {
        segs: vec![seg(hdr, 0, BLACK)],
        font: FontKind::BodyBold,
        size: BODY,
        band: Some(HEADER_BG),
        band_cols: table_cols,
        col_seps: seps.clone(),
        is_header: true,
    }));

    for (i, row) in rows.iter().enumerate() {
        let rs = row_styles
            .and_then(|s| s.get(i))
            .copied()
            .unwrap_or(RowStyle { bold: false, tail: None });
        let full =
            render_row((0..ncol).map(|c| row.get(c).map(|s| s.as_str()).unwrap_or("")), &w, aligns);
        let font = if rs.bold { FontKind::BodyBold } else { FontKind::Body };
        let band = if i % 2 == 1 { Some(ZEBRA) } else { None };
        let segs = match (delta_col, rs.tail) {
            (Some(k), Some(color)) if k < ncol => {
                let split = w[..k].iter().sum::<usize>() + 2 * k;
                let left: String = full.chars().take(split).collect();
                let right: String = full.chars().skip(split).collect();
                vec![seg(left, 0, BLACK), seg(right, split, color)]
            }
            _ => vec![seg(full, 0, BLACK)],
        };
        l.push(Item::Line(Line {
            segs,
            font,
            size: BODY,
            band,
            band_cols: table_cols,
            col_seps: seps.clone(),
            is_header: false,
        }));
    }
}

fn render_row<'a, I: Iterator<Item = &'a str>>(cells: I, w: &[usize], aligns: &[bool]) -> String {
    let mut s = String::new();
    for (c, cell) in cells.enumerate() {
        if c > 0 {
            s.push_str("  ");
        }
        s.push_str(&pad(cell, w[c], aligns[c]));
    }
    s
}

fn pad(s: &str, w: usize, right: bool) -> String {
    let len = s.len();
    if len >= w {
        return s.to_string();
    }
    let sp = " ".repeat(w - len);
    if right {
        format!("{sp}{s}")
    } else {
        format!("{s}{sp}")
    }
}

fn font_id(f: FontKind) -> &'static str {
    match f {
        FontKind::Body => "F1",
        FontKind::BodyBold => "F2",
        FontKind::Title => "F3",
    }
}

fn charw(size: f64) -> f64 {
    0.6 * size
}

fn item_height(it: &Item) -> f64 {
    match it {
        Item::Line(l) => l.size * LINE_FACTOR,
        Item::Rule => RULE_H,
        Item::Gap(h) => *h,
        Item::KeepStart(_) => 0.0,
    }
}

fn paginate(items: &[Item], page_h: f64) -> Vec<Vec<&Item>> {
    let start_y = page_h - TOP_MARGIN;
    let mut pages: Vec<Vec<&Item>> = Vec::new();
    let mut cur: Vec<&Item> = Vec::new();
    let mut y = start_y;
    let mut header: Option<&Item> = None;
    for it in items {
        if let Item::KeepStart(min_h) = it {
            header = None;
            if y - *min_h < BOTTOM_MARGIN && !cur.is_empty() {
                pages.push(std::mem::take(&mut cur));
                y = start_y;
            }
            continue;
        }
        let h = item_height(it);
        if y - h < BOTTOM_MARGIN && !cur.is_empty() {
            pages.push(std::mem::take(&mut cur));
            y = start_y;
            if let Some(hdr) = header {
                cur.push(hdr);
                y -= item_height(hdr);
            }
        }
        cur.push(it);
        y -= h;
        match it {
            Item::Line(ln) if ln.is_header => header = Some(it),
            Item::Rule | Item::Gap(_) => header = None,
            _ => {}
        }
    }
    if pages.is_empty() || !cur.is_empty() {
        pages.push(cur);
    }
    pages
}

fn render_sections(sections: &[(Vec<Item>, f64, f64, bool)]) -> Vec<u8> {
    let mut pages: Vec<(Vec<&Item>, f64, f64, bool)> = Vec::new();
    for (items, page_w, page_h, center) in sections {
        for pg in paginate(items, *page_h) {
            pages.push((pg, *page_w, *page_h, *center));
        }
    }

    let n_pages = pages.len();
    let n_objs = 5 + 2 * n_pages;
    let mut off = vec![0usize; n_objs + 1];
    let mut out: Vec<u8> = Vec::new();

    fn wr(out: &mut Vec<u8>, s: &str) {
        out.extend_from_slice(s.as_bytes());
    }

    wr(&mut out, "%PDF-1.4\n");

    off[1] = out.len();
    wr(&mut out, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    off[2] = out.len();
    let mut kids = String::new();
    for k in 0..n_pages {
        kids.push_str(&format!("{} 0 R ", 6 + 2 * k));
    }
    wr(
        &mut out,
        &format!("2 0 obj\n<< /Type /Pages /Kids [ {}] /Count {} >>\nendobj\n", kids, n_pages),
    );

    off[3] = out.len();
    wr(&mut out, "3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>\nendobj\n");
    off[4] = out.len();
    wr(&mut out, "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier-Bold >>\nendobj\n");
    off[5] = out.len();
    wr(&mut out, "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n");

    let cw = charw(BODY);
    for (k, (page, page_w, page_h, center)) in pages.iter().enumerate() {
        let (page_w, page_h, center) = (*page_w, *page_h, *center);
        let page_id = 6 + 2 * k;
        let content_id = 7 + 2 * k;
        let start_y = page_h - TOP_MARGIN;
        let max_cols = ((page_w - 2.0 * MARGIN) / cw) as usize;
        let table_w = if center {
            page.iter()
                .filter_map(|it| match it {
                    Item::Line(ln) => Some(ln.band_cols),
                    _ => None,
                })
                .max()
                .unwrap_or(0) as f64
                * cw
        } else {
            0.0
        };
        let dx = if center { ((page_w - table_w) / 2.0 - MARGIN).max(0.0) } else { 0.0 };

        let mut gfx = String::new();
        let mut txt = String::from("BT\n");
        let mut y = start_y;
        for it in page.iter() {
            let h = item_height(it);
            let top = y;
            match it {
                Item::Line(ln) => {
                    if let Some(bg) = ln.band {
                        let bx = MARGIN + dx - 2.0;
                        let bw = ln.band_cols as f64 * cw + 4.0;
                        gfx.push_str(&format!(
                            "{:.3} {:.3} {:.3} rg\n{:.2} {:.2} {:.2} {:.2} re f\n",
                            bg[0],
                            bg[1],
                            bg[2],
                            bx,
                            top - h,
                            bw,
                            h
                        ));
                    }
                    for &cs in &ln.col_seps {
                        let x = MARGIN + dx + cs * cw;
                        gfx.push_str(&format!(
                            "{:.3} {:.3} {:.3} RG\n0.4 w\n{:.2} {:.2} m {:.2} {:.2} l S\n",
                            RULE_COLOR[0],
                            RULE_COLOR[1],
                            RULE_COLOR[2],
                            x,
                            top - h,
                            x,
                            top
                        ));
                    }
                    let base = top - ln.size;
                    let fid = font_id(ln.font);
                    for s in &ln.segs {
                        let x = MARGIN + dx + s.col as f64 * cw;
                        txt.push_str(&format!(
                            "/{} {} Tf\n{:.3} {:.3} {:.3} rg\n1 0 0 1 {:.2} {:.2} Tm\n({}) Tj\n",
                            fid,
                            ln.size,
                            s.color[0],
                            s.color[1],
                            s.color[2],
                            x,
                            base,
                            pdf_escape(&s.text, max_cols.saturating_sub(s.col)),
                        ));
                    }
                }
                Item::Rule => {
                    let ry = top - h * 0.5;
                    let (x1, x2) = if center {
                        (MARGIN + dx, MARGIN + dx + table_w)
                    } else {
                        (MARGIN, page_w - MARGIN)
                    };
                    gfx.push_str(&format!(
                        "{:.3} {:.3} {:.3} RG\n0.6 w\n{:.1} {:.2} m {:.1} {:.2} l S\n",
                        RULE_COLOR[0], RULE_COLOR[1], RULE_COLOR[2], x1, ry, x2, ry
                    ));
                }
                Item::Gap(_) => {}
                Item::KeepStart(_) => {}
            }
            y -= h;
        }
        txt.push_str("ET");
        let content = format!("{}{}", gfx, txt);

        off[page_id] = out.len();
        wr(
            &mut out,
            &format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.0} {:.0}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> /Contents {} 0 R >>\nendobj\n",
                page_id, page_w, page_h, content_id
            ),
        );

        off[content_id] = out.len();
        wr(&mut out, &format!("{} 0 obj\n<< /Length {} >>\nstream\n", content_id, content.len()));
        wr(&mut out, &content);
        wr(&mut out, "\nendstream\nendobj\n");
    }

    let xref = out.len();
    wr(&mut out, &format!("xref\n0 {}\n", n_objs + 1));
    wr(&mut out, "0000000000 65535 f \n");
    for o in &off[1..=n_objs] {
        wr(&mut out, &format!("{:010} 00000 n \n", o));
    }
    wr(
        &mut out,
        &format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", n_objs + 1, xref),
    );

    out
}

fn pdf_escape(s: &str, max_cols: usize) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for (n, ch) in s.chars().enumerate() {
        if n >= max_cols {
            out.push_str("...");
            break;
        }
        let c = if ch.is_ascii() && !ch.is_ascii_control() { ch } else { '?' };
        match c {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            _ => out.push(c),
        }
    }
    out
}
