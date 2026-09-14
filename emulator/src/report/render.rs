use super::parser::{
    CostRow, FropRow, MemFnAlignRow, MemFnCostRow, MemFnRatioRow, MemRow, Offsets, OpRow, Report,
};
use std::collections::HashMap;

const CSS: &str = r#"
:root {
  color-scheme: dark;
  --color-primary-dark: #16302b;
  --color-accent-text: #A3E635;
  --color-chart-grid: #1E3D38;
  --color-surface: #1a3630;
  --color-hover: #234742;
  --color-text: #e6f2ec;
  --color-text-muted: #8fb3a5;
  --color-bar: #A3E635;
  --color-up: #f87171;
  --donut-0: #4d7c0f; --donut-1: #65a30d; --donut-2: #84cc16; --donut-3: #a3e635; --donut-4: #d9f99d;
}
:root:has(#theme-light:checked) {
  color-scheme: light;
  --color-primary-dark: #f5f8f1;
  --color-accent-text: #17a601;
  --color-chart-grid: #dde5d6;
  --color-surface: #ffffff;
  --color-hover: #eef3ea;
  --color-text: #000000;
  --color-text-muted: #61756a;
  --color-bar: #84cc16;
  --color-up: #dc2626;
  --donut-0: #365314; --donut-1: #4d7c0f; --donut-2: #65a30d; --donut-3: #84cc16; --donut-4: #bef264;
}
* { box-sizing: border-box; }
body {
  font-family: -apple-system, system-ui, "Segoe UI", sans-serif;
  margin: 0;
  padding: 2.5rem 1rem 4rem;
  background: var(--color-primary-dark);
  color: var(--color-text);
  line-height: 1.5;
}
.wrap { max-width: 1680px; margin: 0 auto; }
.toc { display: flex; flex-wrap: wrap; gap: .4rem; justify-content: center; max-width: 1100px; margin: 0 auto 1.8rem; }
.toc a { font-size: .68rem; font-weight: 600; letter-spacing: .04em; text-transform: uppercase; color: var(--color-text-muted); text-decoration: none; padding: .3rem .7rem; border: 1px solid var(--color-chart-grid); border-radius: 999px; background: var(--color-surface); transition: background .12s ease, color .12s ease, border-color .12s ease; }
.toc a:hover { background: var(--color-hover); color: var(--color-text); border-color: var(--color-bar); }
.anchor { display: block; height: 0; scroll-margin-top: 3.4rem; }
.theme-toggle { position: absolute; width: 1px; height: 1px; opacity: 0; pointer-events: none; }
.controls { position: fixed; top: .9rem; right: .9rem; z-index: 10; display: flex; align-items: center; gap: .45rem; }
.theme-switch { display: inline-flex; gap: 2px; padding: 3px; border-radius: 999px; background: var(--color-surface); border: 1px solid var(--color-chart-grid); cursor: pointer; user-select: none; box-shadow: 0 1px 2px rgba(0,0,0,.08); }
.toc .reset-view { display: inline-flex; align-items: center; }
.toc .reset-view svg { width: 12px; height: 12px; display: block; }
.theme-switch .opt { display: flex; align-items: center; justify-content: center; padding: .32rem .5rem; border-radius: 999px; color: var(--color-text-muted); transition: background .12s ease, color .12s ease; }
.theme-switch .opt svg { width: 14px; height: 14px; display: block; }
.theme-switch .opt-zisk { background: var(--color-bar); color: #16320a; }
:root:has(#theme-light:checked) .theme-switch .opt-zisk { background: transparent; color: var(--color-text-muted); }
:root:has(#theme-light:checked) .theme-switch .opt-light { background: var(--color-bar); color: #16320a; }
.theme-toggle:focus-visible + .theme-switch { outline: 2px solid var(--color-bar); outline-offset: 2px; }
.d0 { --c: var(--donut-0); } .d1 { --c: var(--donut-1); } .d2 { --c: var(--donut-2); } .d3 { --c: var(--donut-3); } .d4 { --c: var(--donut-4); }
.donut circle { stroke: var(--c); }
.donut-legend .sw { background: var(--c); }
h1 {
  font-size: 1.65rem; font-weight: 700; letter-spacing: -.01em;
  color: var(--color-text); margin: 0 0 1.75rem;
  display: flex; align-items: center; justify-content: center; gap: .55rem;
}
h1 svg { height: 1.9em; width: auto; display: block; }
.cards { display: flex; gap: 1rem; flex-wrap: wrap; justify-content: center; margin: 0 0 2rem; }
.card {
  background: var(--color-surface); border: 1px solid var(--color-chart-grid);
  border-radius: 14px; padding: 1rem 1.35rem; min-width: 165px; flex: 0 1 auto;
  box-shadow: 0 1px 2px rgba(0,0,0,.06);
}
.card .label { font-size: .66rem; color: var(--color-text-muted); text-transform: uppercase; letter-spacing: .09em; margin-bottom: .35rem; }
.card .value { font-size: 1.35rem; font-weight: 600; color: var(--color-text); font-variant-numeric: tabular-nums; }
.card.stat-card { display: flex; flex-direction: column; }
.card.stat-card .value { margin: auto 0; }
.card.gauge-card { display: flex; flex-direction: column; align-items: center; gap: .25rem; }
.card.gauge-card .label { align-self: flex-start; }
.card.gauge-card, .card.stat-card { padding-top: .7rem; padding-bottom: .7rem; }
.gauge { width: 116px; height: auto; display: block; }
.gauge .track { stroke: var(--color-chart-grid); }
.gauge .val { stroke: var(--color-bar); }
.gauge .gval { fill: var(--color-text); font-size: 18px; font-weight: 700; text-anchor: middle; dominant-baseline: central; }
.card .sub { font-size: .92rem; color: var(--color-text-muted); font-variant-numeric: tabular-nums; }
.cost-layout { display: flex; align-items: center; justify-content: center; gap: 2.5rem; flex-wrap: wrap; margin: .3rem auto 1.3rem; }
.cost-layout .cost-table { flex: 1 1 440px; max-width: 640px; }
.cost-layout .cost-table table { margin: 0; }
.cost-viz { display: flex; align-items: center; justify-content: center; gap: 1.4rem; flex: 0 0 auto; }
.donut { width: 180px; height: 180px; flex: 0 0 auto; }
.donut-c1 { fill: var(--color-text-muted); font-size: 7px; text-anchor: middle; letter-spacing: .5px; }
.donut-c2 { fill: var(--color-text); font-size: 11px; font-weight: 700; text-anchor: middle; font-variant-numeric: tabular-nums; }
.donut-legend { list-style: none; margin: 0; padding: 0; font-size: .86rem; }
.donut-legend li { display: flex; align-items: center; gap: .55rem; padding: .18rem 0; }
.donut-legend .sw { width: 11px; height: 11px; border-radius: 3px; flex: 0 0 auto; }
.donut-legend .nm { min-width: 6.5em; }
.donut-legend b { font-variant-numeric: tabular-nums; min-width: 4.3em; text-align: right; }
.donut-legend .v { color: var(--color-text-muted); font-variant-numeric: tabular-nums; min-width: 8.5em; text-align: right; }
details {
  background: var(--color-surface); border: 1px solid var(--color-chart-grid);
  border-radius: 12px; margin-bottom: 1rem; padding: 0 1.35rem;
  box-shadow: 0 1px 2px rgba(0,0,0,.05);
}
summary {
  cursor: pointer; user-select: none; list-style: none; display: flex; align-items: center;
  font-size: .78rem; font-weight: 600; letter-spacing: .06em; text-transform: uppercase;
  color: var(--color-text); padding: 1rem 0;
}
summary::-webkit-details-marker { display: none; }
summary::before {
  content: "\25B8"; display: inline-block; width: 1em; margin-right: .5rem;
  color: var(--color-text-muted); transition: transform .15s ease;
}
details[open] > summary::before { transform: rotate(90deg); }
summary:hover { filter: brightness(1.15); }
details[open] > summary { border-bottom: 1px solid var(--color-chart-grid); margin-bottom: .8rem; }
details > table, details > .scroll { margin: 0 auto 1.15rem; }
table { border-collapse: collapse; width: 100%; max-width: 900px; margin: 0 auto; font-size: .92rem; }
thead th { color: var(--color-text-muted); font-weight: 600; font-size: .7rem; text-transform: uppercase; letter-spacing: .05em; text-align: left; padding: .5rem .6rem; border-bottom: 1px solid var(--color-chart-grid); }
td { padding: .4rem .6rem; border-bottom: 1px solid var(--color-chart-grid); }
tbody tr:last-child td { border-bottom: none; }
tbody tr:hover { background: var(--color-hover); }
th.num, td.num { text-align: right; font-variant-numeric: tabular-nums; }
svg.bar { width: 120px; height: 8px; vertical-align: middle; }
.bar-bg { fill: var(--color-chart-grid); }
.bar-fg { fill: var(--color-bar); }
.scroll { overflow-x: auto; max-width: 100%; }
table.ops { max-width: none; margin: 0 auto; font-size: .84rem; }
table.ops th, table.ops td { white-space: nowrap; padding: .35rem .6rem; }
table.ops thead th.grp { text-align: center; letter-spacing: .08em; border-bottom: 1px solid var(--color-chart-grid); }
.legend { text-align: center; color: var(--color-text-muted); font-size: .82rem; margin: -1rem 0 1.75rem; }
.legend .up { color: var(--color-up); }
.legend .down { color: var(--color-accent-text); }
.cmp { font-size: .82rem; color: var(--color-text-muted); margin-top: .35rem; font-variant-numeric: tabular-nums; }
.delta.down { color: var(--color-accent-text); }
.delta.up { color: var(--color-up); }
.delta.zero { color: var(--color-text-muted); }
thead th .fname { text-transform: none; font-weight: 400; letter-spacing: 0; opacity: .8; }
tr.lvl2 td:first-child { padding-left: 1.9rem; color: var(--color-text-muted); }
tr.subtotal td { font-weight: 600; border-top: 1px solid var(--color-chart-grid); }
tr.total td { font-weight: 700; border-top: 2px solid var(--color-text); }
tr.aside td { color: var(--color-text-muted); border-top: 2px dashed var(--color-chart-grid); }
.note { max-width: 900px; margin: .2rem auto 1.15rem; text-align: center; color: var(--color-text-muted); font-size: .78rem; }
.sortable { position: relative; }
.sortable input.srt { position: absolute; width: 1px; height: 1px; opacity: 0; pointer-events: none; }
.sortable .by-cost { display: none; }
.sortable input.cost:checked ~ .by-cost { display: block; }
.sortable input.cost:checked ~ .by-count { display: none; }
th label.sortlab { cursor: pointer; user-select: none; display: inline-flex; align-items: center; gap: .3rem; padding: .12rem .55rem; border: 1px solid var(--color-chart-grid); border-radius: 999px; color: var(--color-text-muted); transition: background .12s ease, border-color .12s ease, color .12s ease; }
th label.sortlab::after { content: "\2195"; opacity: .45; font-size: .95em; }
th label.sortlab:hover { background: var(--color-hover); color: var(--color-text); }
.sortable input.count:checked ~ .by-count .lab-count,
.sortable input.cost:checked ~ .by-cost .lab-cost { background: var(--color-bar); border-color: var(--color-bar); color: #16320a; font-weight: 700; }
.sortable input.count:checked ~ .by-count .lab-count::after,
.sortable input.cost:checked ~ .by-cost .lab-cost::after { content: "\25BE"; opacity: 1; }
@media (prefers-reduced-motion: reduce) { th label.sortlab { transition: none; } }
"#;

const LOGO: &str = include_str!("zisk.svg");

pub fn single(r: &Report) -> String {
    let mut h = String::new();
    h.push_str(&head());
    h.push_str(&header_cards(r));
    h.push_str(&section("COST DISTRIBUTION", true, &cost_distribution(&r.cost)));
    h.push_str(&section(
        "COST BY BASE OPCODE",
        false,
        &sort_table("sort-base", "%", op_sort(&r.op_base)),
    ));
    h.push_str(&section(
        "COST BY PRECOMPILED OPCODE",
        false,
        &sort_table("sort-precomp", "%", op_sort(&r.precompiles)),
    ));
    h.push_str(&section(
        "FROPS BY OPCODE",
        false,
        &sort_table("sort-frops", "HIT", frop_sort(&r.frop)),
    ));
    h.push_str(&section(
        "MEM COST BY TYPE",
        false,
        &sort_table("sort-memtype", "%", mem_sort(&r.mem_by_type)),
    ));
    h.push_str(&section(
        "MEM TOTALS",
        false,
        &sort_table("sort-memtot", "%", mem_sort(&r.mem_totals)),
    ));
    h.push_str(&section(
        "DETAILED MEM",
        false,
        &sort_table("sort-detmem", "%", mem_sort(&r.detailed_mem)),
    ));
    h.push_str(&section(
        "DETAILED MEM (FULL)",
        false,
        &sort_table("sort-detmemfull", "%", mem_sort(&r.detailed_mem_full)),
    ));
    h.push_str(&section("MEM OFFSETS", false, &offsets_table(&r.mem_offsets)));
    h.push_str(&section("TOP MEMORY COST FUNCTIONS", false, &mem_top_cost_table(&r.mem_top_cost)));
    h.push_str(&section(
        "TOP UNALIGNED MEMORY FUNCTIONS",
        false,
        &mem_top_unaligned_table(&r.mem_top_unaligned),
    ));
    h.push_str(&section(
        "TOP UNALIGNED/STEP RATIO FUNCTIONS",
        false,
        &mem_top_ratio_table(&r.mem_top_ratio),
    ));
    h.push_str(&foot());
    with_toc(h)
}

fn mem_top_cost_table(rows: &[MemFnCostRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut h = String::from(
        "<div class=\"scroll\">\n<table class=\"ops\">\n<thead><tr><th></th>\
         <th class=\"num\">MEM COST</th><th class=\"num\">%</th>\
         <th class=\"num\">CALLS</th><th class=\"num\">COST/CALL</th><th></th></tr></thead>\n<tbody>\n",
    );
    for r in rows {
        h.push_str(&format!(
            "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{:.2}%</td>\
             <td class=\"num\">{}</td><td class=\"num\">{}</td><td>{}</td></tr>\n",
            esc(&r.name),
            fmt_num(r.cost),
            r.cost_pct,
            fmt_num(r.calls),
            fmt_num(r.cost_per_call),
            bar(r.cost_pct),
        ));
    }
    h.push_str("</tbody>\n</table>\n</div>\n");
    h
}

fn mem_top_unaligned_table(rows: &[MemFnAlignRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut h = String::from(
        "<div class=\"scroll\">\n<table class=\"ops\">\n<thead><tr><th></th>\
         <th class=\"num\">UNALIGNED</th><th class=\"num\">ALIGNED</th>\
         <th class=\"num\">% UNALIGNED</th><th class=\"num\">CALLS</th><th></th></tr></thead>\n<tbody>\n",
    );
    for r in rows {
        h.push_str(&format!(
            "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td>\
             <td class=\"num\">{:.2}%</td><td class=\"num\">{}</td><td>{}</td></tr>\n",
            esc(&r.name),
            fmt_num(r.unaligned),
            fmt_num(r.aligned),
            r.unaligned_pct,
            fmt_num(r.calls),
            bar(r.unaligned_pct),
        ));
    }
    h.push_str("</tbody>\n</table>\n</div>\n");
    h
}

fn mem_top_ratio_table(rows: &[MemFnRatioRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut h = String::from(
        "<div class=\"scroll\">\n<table class=\"ops\">\n<thead><tr><th></th>\
         <th class=\"num\">RATIO</th><th class=\"num\">UNALIGNED</th><th class=\"num\">% UNALIGNED</th>\
         <th class=\"num\">UNALIGNED ACC./CALL</th><th class=\"num\">CALLS</th><th></th></tr></thead>\n<tbody>\n",
    );
    for r in rows {
        h.push_str(&format!(
            "<tr><td>{}</td><td class=\"num\">{:.2}x</td><td class=\"num\">{}</td>\
             <td class=\"num\">{:.2}%</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td>{}</td></tr>\n",
            esc(&r.name),
            r.ratio,
            fmt_num(r.unaligned),
            r.unaligned_pct,
            fmt_num(r.accesses_per_call),
            fmt_num(r.calls),
            bar(r.ratio * 20.0),
        ));
    }
    h.push_str("</tbody>\n</table>\n</div>\n");
    h
}

fn head() -> String {
    let mut h = String::new();
    h.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    h.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    h.push_str("<title>ZisK stats report</title>\n<style>");
    h.push_str(CSS);
    h.push_str("</style>\n</head>\n<body>\n");
    h.push_str(
        "<div class=\"controls\">\n\
         <input type=\"checkbox\" id=\"theme-light\" class=\"theme-toggle\" autocomplete=\"off\">\n\
         <label for=\"theme-light\" class=\"theme-switch\" title=\"Switch colour theme\" aria-label=\"Switch colour theme\">\
         <span class=\"opt opt-zisk\" title=\"ZisK (dark)\">\
         <svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\">\
         <path d=\"M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z\"/></svg></span>\
         <span class=\"opt opt-light\" title=\"Light\">\
         <svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\">\
         <circle cx=\"12\" cy=\"12\" r=\"4\"/>\
         <path d=\"M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4\"/></svg></span>\
         </label>\n\
         </div>\n",
    );
    h.push_str("<div class=\"wrap\">\n<h1>");
    h.push_str(LOGO);
    h.push_str("stats report</h1>\n");
    h
}

fn section(title: &str, open: bool, inner: &str) -> String {
    if inner.is_empty() {
        return String::new();
    }
    format!(
        "<details{}>\n<summary>{}</summary>\n<span id=\"s-{}\" class=\"anchor\"></span>\n{}</details>\n",
        if open { " open" } else { "" },
        esc(title),
        section_id(title),
        inner,
    )
}

fn section_id(title: &str) -> String {
    let mut s = String::new();
    let mut dash = false;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
            dash = false;
        } else if !dash {
            s.push('-');
            dash = true;
        }
    }
    s.trim_matches('-').to_string()
}

fn with_toc(h: String) -> String {
    let mut links = String::new();
    let mut pos = 0;
    while let Some(rel) = h[pos..].find("<summary>") {
        let ts = pos + rel + "<summary>".len();
        let te = match h[ts..].find("</summary>") {
            Some(e) => ts + e,
            None => break,
        };
        let title = &h[ts..te];
        let id = h[te..].find("id=\"s-").and_then(|ip| {
            let is = te + ip + 4;
            h[is..].find('"').map(|q| &h[is..is + q])
        });
        if let (Some(id), false) = (id, title.is_empty()) {
            links.push_str(&format!("<a href=\"#{}\">{}</a>", id, title));
        }
        pos = te;
    }
    if links.is_empty() {
        return h;
    }
    links.push_str(
        "<a href=\"\" class=\"reset-view\" title=\"Reset view: collapse sections, back to top\" aria-label=\"Reset view\">\
         <svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\">\
         <polyline points=\"1 4 1 10 7 10\"/><path d=\"M3.51 15a9 9 0 1 0 2.13-9.36L1 10\"/></svg></a>",
    );
    let nav = format!("<nav class=\"toc\">{}</nav>\n", links);
    match h.find("<details") {
        Some(p) => format!("{}{}{}", &h[..p], nav, &h[p..]),
        None => h,
    }
}

fn foot() -> String {
    "</div>\n</body>\n</html>\n".to_string()
}

struct Cmp {
    name: String,
    ca: Option<u64>,
    cb: Option<u64>,
    a: Option<u64>,
    b: Option<u64>,
    key: u64,
}

pub fn compare(a: &Report, b: &Report, name_a: &str, name_b: &str) -> String {
    let mut h = String::new();
    h.push_str(&head());
    h.push_str(&format!(
        "<p class=\"legend\">A = {} (baseline) · B = {} · Δ = B - A · \
         <span class=\"down\">green = lower cost</span> / <span class=\"up\">red = higher</span></p>\n",
        esc(name_a),
        esc(name_b),
    ));
    h.push_str(&compare_cards(a, b));
    h.push_str(&section(
        "COST DISTRIBUTION",
        true,
        &cost_distribution_cmp(&a.cost, &b.cost, name_a, name_b),
    ));
    h.push_str(&section(
        "COST BY BASE OPCODE",
        false,
        &cmp_table(&align(op_sort(&a.op_base), op_sort(&b.op_base), true), true, name_a, name_b),
    ));
    h.push_str(&section(
        "COST BY PRECOMPILED OPCODE",
        false,
        &cmp_table(
            &align(op_sort(&a.precompiles), op_sort(&b.precompiles), true),
            true,
            name_a,
            name_b,
        ),
    ));
    h.push_str(&section(
        "FROPS BY OPCODE",
        false,
        &cmp_table(&align(frop_sort(&a.frop), frop_sort(&b.frop), true), true, name_a, name_b),
    ));
    h.push_str(&section(
        "MEM COST BY TYPE",
        false,
        &cmp_table(
            &align(mem_sort(&a.mem_by_type), mem_sort(&b.mem_by_type), false),
            true,
            name_a,
            name_b,
        ),
    ));
    h.push_str(&section(
        "MEM TOTALS",
        false,
        &cmp_table(
            &align(mem_sort(&a.mem_totals), mem_sort(&b.mem_totals), false),
            true,
            name_a,
            name_b,
        ),
    ));
    h.push_str(&section(
        "DETAILED MEM",
        false,
        &cmp_table(
            &align(mem_sort(&a.detailed_mem), mem_sort(&b.detailed_mem), false),
            true,
            name_a,
            name_b,
        ),
    ));
    h.push_str(&section(
        "DETAILED MEM (FULL)",
        false,
        &cmp_table(
            &align(mem_sort(&a.detailed_mem_full), mem_sort(&b.detailed_mem_full), false),
            true,
            name_a,
            name_b,
        ),
    ));
    h.push_str(&section(
        "MEM OFFSETS (totals)",
        false,
        &cmp_table(
            &align(offset_sort(&a.mem_offsets), offset_sort(&b.mem_offsets), false),
            false,
            name_a,
            name_b,
        ),
    ));
    h.push_str(&section(
        "TOP MEMORY COST FUNCTIONS",
        false,
        &cmp_table(
            &align(mem_cost_sort(&a.mem_top_cost), mem_cost_sort(&b.mem_top_cost), false),
            true,
            name_a,
            name_b,
        ),
    ));
    h.push_str(&section(
        "TOP UNALIGNED MEMORY FUNCTIONS",
        false,
        &cmp_table(
            &align(
                mem_unaligned_sort(&a.mem_top_unaligned),
                mem_unaligned_sort(&b.mem_top_unaligned),
                false,
            ),
            true,
            name_a,
            name_b,
        ),
    ));
    h.push_str(&foot());
    with_toc(h)
}

struct SortRow {
    name: String,
    count: u64,
    pct2: f64,
    cost: u64,
    cost_pct: f64,
}

fn op_sort(rows: &[OpRow]) -> Vec<SortRow> {
    rows.iter()
        .map(|r| SortRow {
            name: r.name.clone(),
            count: r.count,
            pct2: r.count_pct,
            cost: r.cost,
            cost_pct: r.cost_pct,
        })
        .collect()
}
fn frop_sort(rows: &[FropRow]) -> Vec<SortRow> {
    rows.iter()
        .map(|r| SortRow {
            name: r.name.clone(),
            count: r.count,
            pct2: r.hit_pct,
            cost: r.cost,
            cost_pct: r.cost_pct,
        })
        .collect()
}
fn mem_sort(rows: &[MemRow]) -> Vec<SortRow> {
    rows.iter()
        .map(|r| SortRow {
            name: r.label.clone(),
            count: r.count,
            pct2: r.count_pct,
            cost: r.cost,
            cost_pct: r.cost_pct,
        })
        .collect()
}
fn mem_cost_sort(rows: &[MemFnCostRow]) -> Vec<SortRow> {
    rows.iter()
        .map(|r| SortRow {
            name: r.name.clone(),
            count: r.calls,
            pct2: 0.0,
            cost: r.cost,
            cost_pct: r.cost_pct,
        })
        .collect()
}
fn mem_unaligned_sort(rows: &[MemFnAlignRow]) -> Vec<SortRow> {
    rows.iter()
        .map(|r| SortRow {
            name: r.name.clone(),
            count: r.calls,
            pct2: 0.0,
            cost: r.unaligned,
            cost_pct: r.unaligned_pct,
        })
        .collect()
}
fn offset_sort(o: &Offsets) -> Vec<SortRow> {
    o.rows
        .iter()
        .map(|(name, vals)| SortRow {
            name: name.clone(),
            count: 0,
            pct2: 0.0,
            cost: *vals.last().unwrap_or(&0),
            cost_pct: 0.0,
        })
        .collect()
}

fn align(a: Vec<SortRow>, b: Vec<SortRow>, by_key: bool) -> Vec<Cmp> {
    let mut order: Vec<String> = Vec::new();
    let mut map: HashMap<String, Cmp> = HashMap::new();
    for row in a {
        order.push(row.name.clone());
        map.insert(
            row.name.clone(),
            Cmp {
                name: row.name,
                ca: Some(row.count),
                cb: None,
                a: Some(row.cost),
                b: None,
                key: row.count,
            },
        );
    }
    for row in b {
        match map.get_mut(&row.name) {
            Some(c) => {
                c.cb = Some(row.count);
                c.b = Some(row.cost);
                c.key = c.key.max(row.count);
            }
            None => {
                order.push(row.name.clone());
                map.insert(
                    row.name.clone(),
                    Cmp {
                        name: row.name,
                        ca: None,
                        cb: Some(row.count),
                        a: None,
                        b: Some(row.cost),
                        key: row.count,
                    },
                );
            }
        }
    }
    let mut rows: Vec<Cmp> = order.iter().filter_map(|n| map.remove(n)).collect();
    if by_key {
        rows.sort_by_key(|c| std::cmp::Reverse(c.key));
    }
    rows
}

fn ab_headers(name_a: &str, name_b: &str) -> String {
    format!(
        "<th class=\"num\">A <span class=\"fname\">{}</span></th>\
         <th class=\"num\">B <span class=\"fname\">{}</span></th>",
        esc(name_a),
        esc(name_b),
    )
}

fn cmp_table(rows: &[Cmp], show_count: bool, name_a: &str, name_b: &str) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut h = String::from("<div class=\"scroll\">\n<table class=\"ops\">\n<thead>\n");
    if show_count {
        h.push_str(
            "<tr><th></th><th class=\"grp\" colspan=\"3\">COUNT</th><th class=\"grp\" colspan=\"4\">COST</th></tr>\n<tr><th></th>",
        );
        h.push_str(&ab_headers(name_a, name_b));
        h.push_str("<th class=\"num\">Δ</th>");
        h.push_str(&ab_headers(name_a, name_b));
        h.push_str("<th class=\"num\">Δ</th><th class=\"num\">%</th></tr>\n");
    } else {
        h.push_str("<tr><th></th>");
        h.push_str(&ab_headers(name_a, name_b));
        h.push_str("<th class=\"num\">Δ</th><th class=\"num\">%</th></tr>\n");
    }
    h.push_str("</thead>\n<tbody>\n");
    for r in rows {
        let cost_a = r.a.map(fmt_num).unwrap_or_else(|| "—".to_string());
        let cost_b = r.b.map(fmt_num).unwrap_or_else(|| "—".to_string());
        let (dcost, pct, cls) = delta_cells(r.a, r.b);
        if show_count {
            let cnt_a = r.ca.map(fmt_num).unwrap_or_else(|| "—".to_string());
            let cnt_b = r.cb.map(fmt_num).unwrap_or_else(|| "—".to_string());
            let (dcount, _, clsc) = delta_cells(r.ca, r.cb);
            h.push_str(&format!(
                "<tr><td>{name}</td>\
                 <td class=\"num\">{cnt_a}</td><td class=\"num\">{cnt_b}</td><td class=\"num delta {clsc}\">{dcount}</td>\
                 <td class=\"num\">{cost_a}</td><td class=\"num\">{cost_b}</td><td class=\"num delta {cls}\">{dcost}</td><td class=\"num delta {cls}\">{pct}</td></tr>\n",
                name = esc(&r.name),
                cnt_a = cnt_a,
                cnt_b = cnt_b,
                clsc = clsc,
                dcount = dcount,
                cost_a = cost_a,
                cost_b = cost_b,
                cls = cls,
                dcost = dcost,
                pct = pct,
            ));
        } else {
            h.push_str(&format!(
                "<tr><td>{name}</td><td class=\"num\">{cost_a}</td><td class=\"num\">{cost_b}</td>\
                 <td class=\"num delta {cls}\">{dcost}</td><td class=\"num delta {cls}\">{pct}</td></tr>\n",
                name = esc(&r.name),
                cost_a = cost_a,
                cost_b = cost_b,
                cls = cls,
                dcost = dcost,
                pct = pct,
            ));
        }
    }
    h.push_str("</tbody>\n</table>\n</div>\n");
    h
}

fn cost_distribution_cmp(a: &[CostRow], b: &[CostRow], name_a: &str, name_b: &str) -> String {
    if a.is_empty() && b.is_empty() {
        return String::new();
    }
    let mut h = String::from("<table>\n");
    h.push_str("<thead><tr><th></th>");
    h.push_str(&ab_headers(name_a, name_b));
    h.push_str("<th class=\"num\">Δ</th><th class=\"num\">%</th></tr></thead>\n<tbody>\n");
    for ca in a {
        let bv = b.iter().find(|c| c.label == ca.label).map(|c| c.cost);
        let b_str = bv.map(fmt_num).unwrap_or_else(|| "—".to_string());
        let (delta, pct, cls) = delta_cells(Some(ca.cost), bv);
        h.push_str(&format!(
            "<tr class=\"{}\"><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td>\
             <td class=\"num delta {}\">{}</td><td class=\"num delta {}\">{}</td></tr>\n",
            cost_row_class(&ca.label),
            esc(&ca.label),
            fmt_num(ca.cost),
            b_str,
            cls,
            delta,
            cls,
            pct,
        ));
    }
    h.push_str("</tbody>\n</table>\n");
    h.push_str(COST_NOTE);
    h
}

fn compare_cards(a: &Report, b: &Report) -> String {
    let mut h = String::from("<div class=\"cards\">");
    h.push_str(&compare_card("STEPS", Some(a.steps), Some(b.steps)));
    h.push_str(&compare_card(
        "TOTAL COST",
        find_cost(&a.cost, "TOTAL").map(|c| c.cost),
        find_cost(&b.cost, "TOTAL").map(|c| c.cost),
    ));
    h.push_str(&compare_card(
        "FROPS",
        find_cost(&a.cost, "FROPS").map(|c| c.cost),
        find_cost(&b.cost, "FROPS").map(|c| c.cost),
    ));
    h.push_str("</div>\n");
    h
}

fn compare_card(label: &str, a: Option<u64>, b: Option<u64>) -> String {
    let a_str = a.map(fmt_num).unwrap_or_else(|| "—".to_string());
    let b_str = b.map(fmt_num).unwrap_or_else(|| "—".to_string());
    let (_, pct, cls) = delta_cells(a, b);
    format!(
        "<div class=\"card\"><div class=\"label\">{}</div>\
         <div class=\"value\">{}</div>\
         <div class=\"cmp\">→ {} <span class=\"delta {}\">{}</span></div></div>",
        esc(label),
        a_str,
        b_str,
        cls,
        pct,
    )
}

fn delta_cells(a: Option<u64>, b: Option<u64>) -> (String, String, &'static str) {
    let av = a.unwrap_or(0);
    let bv = b.unwrap_or(0);
    let d = bv as i128 - av as i128;
    let cls = if d < 0 {
        "down"
    } else if d > 0 {
        "up"
    } else {
        "zero"
    };
    let sign = if d > 0 {
        "+"
    } else if d < 0 {
        "-"
    } else {
        ""
    };
    let delta = format!("{}{}", sign, fmt_num(d.unsigned_abs() as u64));
    let pct = if av == 0 {
        "new".to_string()
    } else {
        let p = d as f64 / av as f64 * 100.0;
        let s = if p > 0.0 {
            "+"
        } else if p < 0.0 {
            "-"
        } else {
            ""
        };
        format!("{}{:.2}%", s, p.abs())
    };
    (delta, pct, cls)
}

fn header_cards(r: &Report) -> String {
    let mut h = String::from("<div class=\"cards\">");
    h.push_str(&card("STEPS", &fmt_num(r.steps)));
    if let Some(total) = find_cost(&r.cost, "TOTAL") {
        h.push_str(&card("TOTAL COST", &fmt_num(total.cost)));
    }
    if let Some(frops) = find_cost(&r.cost, "FROPS") {
        h.push_str(&gauge_card("FROPS", frops.pct, &fmt_num(frops.cost)));
    }
    if r.ram_usage.used > 0 {
        h.push_str(&gauge_card("RAM USAGE", r.ram_usage.pct, &fmt_num(r.ram_usage.used)));
    }
    h.push_str(&gauge_card("ROM USAGE", r.rom_usage.pct, &fmt_num(r.rom_usage.used)));
    h.push_str("</div>\n");
    h
}

const COST_NOTE: &str = "<p class=\"note\">VARIABLE = MAIN + OPCODES + PRECOMPILES + MEMORY \
     &nbsp;·&nbsp; TOTAL = VARIABLE + BASE &nbsp;·&nbsp; FROPS is shown separately (not part of TOTAL).</p>\n";

fn cost_row_class(label: &str) -> &'static str {
    match label {
        "MAIN" | "OPCODES" | "PRECOMPILES" | "MEMORY" => "lvl2",
        "VARIABLE" => "subtotal",
        "TOTAL" => "total",
        "FROPS" => "aside",
        _ => "",
    }
}

fn cost_donut(cost: &[CostRow]) -> String {
    const PARTS: [&str; 5] = ["MAIN", "OPCODES", "PRECOMPILES", "MEMORY", "BASE"];
    const C: f64 = 251.327;

    let get = |name: &str| cost.iter().find(|c| c.label == name).map(|c| c.cost).unwrap_or(0);
    let total: u64 = PARTS.iter().map(|p| get(p)).sum();
    if total == 0 {
        return String::new();
    }

    let mut arcs = String::new();
    let mut cum = 0.0_f64;
    for (i, p) in PARTS.iter().enumerate() {
        let cst = get(p);
        if cst == 0 {
            continue;
        }
        let seg = C * (cst as f64 / total as f64);
        let draw = (seg - 1.2).max(0.4);
        arcs.push_str(&format!(
            "<circle class=\"d{i}\" cx=\"50\" cy=\"50\" r=\"40\" fill=\"none\" stroke-width=\"15\" \
             stroke-dasharray=\"{draw:.2} {C:.2}\" stroke-dashoffset=\"{off:.2}\" transform=\"rotate(-90 50 50)\"/>",
            draw = draw,
            C = C,
            off = -cum,
        ));
        cum += seg;
    }

    let mut legend = String::from("<ul class=\"donut-legend\">");
    for (i, p) in PARTS.iter().enumerate() {
        let cst = get(p);
        let pctv = cst as f64 / total as f64 * 100.0;
        legend.push_str(&format!(
            "<li><span class=\"sw d{i}\"></span>\
             <span class=\"nm\">{name}</span><b>{pctv:.2}%</b><span class=\"v\">{val}</span></li>",
            name = esc(p),
            pctv = pctv,
            val = fmt_num(cst),
        ));
    }
    legend.push_str("</ul>");

    format!(
        "<div class=\"cost-viz\"><svg class=\"donut\" viewBox=\"0 0 100 100\">{arcs}\
         <text class=\"donut-c1\" x=\"50\" y=\"45\">TOTAL</text>\
         <text class=\"donut-c2\" x=\"50\" y=\"58\" textLength=\"54\" lengthAdjust=\"spacingAndGlyphs\">{total}</text>\
         </svg>{legend}</div>\n",
        arcs = arcs,
        total = fmt_num(total),
        legend = legend,
    )
}

fn cost_distribution(cost: &[CostRow]) -> String {
    if cost.is_empty() {
        return String::new();
    }
    let mut table = String::from("<table>\n");
    table.push_str("<thead><tr><th></th><th class=\"num\">COST</th><th class=\"num\">%</th><th></th></tr></thead>\n<tbody>\n");
    for c in cost {
        table.push_str(&format!(
            "<tr class=\"{}\"><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{:.2}%</td><td>{}</td></tr>\n",
            cost_row_class(&c.label),
            esc(&c.label),
            fmt_num(c.cost),
            c.pct,
            bar(c.pct),
        ));
    }
    table.push_str("</tbody>\n</table>\n");

    let mut h = String::from("<div class=\"cost-layout\">\n<div class=\"cost-table\">\n");
    h.push_str(&table);
    h.push_str("</div>\n");
    h.push_str(&cost_donut(cost));
    h.push_str("</div>\n");
    h.push_str(COST_NOTE);
    h
}

fn sortable(id: &str, by_count: &str, by_cost: &str) -> String {
    format!(
        "<div class=\"sortable\">\
         <input type=\"radio\" name=\"{id}\" id=\"{id}-count\" class=\"srt count\" autocomplete=\"off\" checked>\
         <input type=\"radio\" name=\"{id}\" id=\"{id}-cost\" class=\"srt cost\" autocomplete=\"off\">\
         <div class=\"by-count\">{by_count}</div>\
         <div class=\"by-cost\">{by_cost}</div>\
         </div>\n",
        id = id,
        by_count = by_count,
        by_cost = by_cost,
    )
}

fn sort_rows(id: &str, hdr2: &str, rows: &[SortRow], by_cost: bool) -> String {
    let mut sorted: Vec<&SortRow> = rows.iter().collect();
    if by_cost {
        sorted.sort_by_key(|r| std::cmp::Reverse(r.cost));
    } else {
        sorted.sort_by_key(|r| std::cmp::Reverse(r.count));
    }

    let mut h = String::from("<div class=\"scroll\">\n<table class=\"ops\">\n");
    h.push_str(&format!(
        "<thead><tr><th></th>\
         <th class=\"num\"><label for=\"{id}-count\" class=\"sortlab lab-count\">COUNT</label></th>\
         <th class=\"num\">{hdr2}</th>\
         <th class=\"num\"><label for=\"{id}-cost\" class=\"sortlab lab-cost\">COST</label></th>\
         <th class=\"num\">%</th><th></th></tr></thead>\n<tbody>\n",
        id = id,
        hdr2 = hdr2,
    ));
    for r in sorted {
        h.push_str(&format!(
            "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{:.2}%</td><td class=\"num\">{}</td><td class=\"num\">{:.2}%</td><td>{}</td></tr>\n",
            esc(&r.name),
            fmt_num(r.count),
            r.pct2,
            fmt_num(r.cost),
            r.cost_pct,
            bar(r.cost_pct),
        ));
    }
    h.push_str("</tbody>\n</table>\n</div>\n");
    h
}

fn sort_table(id: &str, hdr2: &str, rows: Vec<SortRow>) -> String {
    if rows.is_empty() {
        return String::new();
    }
    sortable(id, &sort_rows(id, hdr2, &rows, false), &sort_rows(id, hdr2, &rows, true))
}

fn offsets_table(o: &Offsets) -> String {
    if o.rows.is_empty() {
        return String::new();
    }
    let mut h = String::from("<div class=\"scroll\">\n<table class=\"ops\">\n<thead><tr><th></th>");
    for c in &o.cols {
        h.push_str(&format!("<th class=\"num\">{}</th>", esc(c)));
    }
    h.push_str("</tr></thead>\n<tbody>\n");
    for (label, vals) in &o.rows {
        h.push_str(&format!("<tr><td>{}</td>", esc(label)));
        for v in vals {
            h.push_str(&format!("<td class=\"num\">{}</td>", fmt_num(*v)));
        }
        h.push_str("</tr>\n");
    }
    h.push_str("</tbody>\n</table>\n</div>\n");
    h
}

fn find_cost<'a>(cost: &'a [CostRow], label: &str) -> Option<&'a CostRow> {
    cost.iter().find(|c| c.label == label)
}

pub(crate) fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn card(label: &str, value: &str) -> String {
    format!(
        "<div class=\"card stat-card\"><div class=\"label\">{}</div><div class=\"value\">{}</div></div>",
        esc(label),
        esc(value)
    )
}

fn gauge_card(label: &str, pct: f64, sub: &str) -> String {
    let val = 125.66 * (pct.clamp(0.0, 100.0) / 100.0);
    format!(
        "<div class=\"card gauge-card\"><div class=\"label\">{label}</div>\
         <svg class=\"gauge\" viewBox=\"0 0 100 56\">\
         <circle class=\"track\" cx=\"50\" cy=\"50\" r=\"40\" transform=\"rotate(180 50 50)\" fill=\"none\" stroke-width=\"8\" stroke-linecap=\"round\" stroke-dasharray=\"125.66 125.66\"/>\
         <circle class=\"val\" cx=\"50\" cy=\"50\" r=\"40\" transform=\"rotate(180 50 50)\" fill=\"none\" stroke-width=\"8\" stroke-linecap=\"round\" stroke-dasharray=\"{val:.2} 251.33\"/>\
         <text class=\"gval\" x=\"50\" y=\"43\">{pct:.2}%</text>\
         </svg><div class=\"sub\">{sub}</div></div>",
        label = esc(label),
        val = val,
        pct = pct,
        sub = esc(sub),
    )
}

fn bar(p: f64) -> String {
    let w = p.clamp(0.0, 100.0);
    format!(
        "<svg class=\"bar\" viewBox=\"0 0 100 10\" preserveAspectRatio=\"none\">\
         <rect class=\"bar-bg\" x=\"0\" y=\"0\" width=\"100\" height=\"10\" rx=\"2\"/>\
         <rect class=\"bar-fg\" x=\"0\" y=\"0\" width=\"{w:.2}\" height=\"10\" rx=\"2\"/></svg>"
    )
}
