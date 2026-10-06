use crate::stats::detect_sep;

#[derive(Debug, Default)]
pub struct Report {
    pub steps: u64,
    pub cost: Vec<CostRow>,
    pub ram_usage: Usage,
    pub rom_usage: Usage,
    pub mem_by_type: Vec<MemRow>,
    pub mem_totals: Vec<MemRow>,
    pub detailed_mem: Vec<MemRow>,
    pub detailed_mem_full: Vec<MemRow>,
    pub mem_offsets: Offsets,
    pub mem_top_cost: Vec<MemFnCostRow>,
    pub mem_top_unaligned: Vec<MemFnAlignRow>,
    pub mem_top_ratio: Vec<MemFnRatioRow>,
    pub op_base: Vec<OpRow>,
    pub precompiles: Vec<OpRow>,
    pub frop: Vec<FropRow>,
}

/// `MEM_TOP_COST` row: a function's total memory cost, its share of the program's memory cost,
/// the calls it received and the cost each call paid.
#[derive(Debug)]
pub struct MemFnCostRow {
    pub name: String,
    pub cost: u64,
    pub cost_pct: f64,
    pub calls: u64,
    pub cost_per_call: u64,
}

/// `MEM_TOP_UNALIGNED` row: a function's unaligned and aligned memory cost, and how much of its
/// memory cost is unaligned.
#[derive(Debug)]
pub struct MemFnAlignRow {
    pub name: String,
    pub unaligned: u64,
    pub aligned: u64,
    pub unaligned_pct: f64,
    pub calls: u64,
}

/// `MEM_TOP_RATIO` row: how far a function's unaligned cost per step exceeds the program average,
/// with the unaligned cost behind the ratio and the unaligned accesses each call performs.
#[derive(Debug)]
pub struct MemFnRatioRow {
    pub name: String,
    pub ratio: f64,
    pub unaligned: u64,
    pub unaligned_pct: f64,
    pub accesses_per_call: u64,
    pub calls: u64,
}

#[derive(Debug, Default)]
pub struct Usage {
    pub used: u64,
    pub pct: f64,
}

#[derive(Debug)]
pub struct CostRow {
    pub label: String,
    pub cost: u64,
    pub pct: f64,
}

#[derive(Debug)]
pub struct MemRow {
    pub label: String,
    pub count: u64,
    pub count_pct: f64,
    pub cost: u64,
    pub cost_pct: f64,
}

#[derive(Debug)]
pub struct OpRow {
    pub name: String,
    pub count: u64,
    pub count_pct: f64,
    pub cost: u64,
    pub cost_pct: f64,
}

#[derive(Debug)]
pub struct FropRow {
    pub name: String,
    pub count: u64,
    pub hit_pct: f64,
    pub cost: u64,
    pub cost_pct: f64,
}

#[derive(Debug, Default)]
pub struct Offsets {
    pub cols: Vec<String>,
    pub rows: Vec<(String, Vec<u64>)>,
}

pub fn parse(csv: &str) -> Report {
    let mut r = Report::default();
    let sep = detect_sep(csv);

    for line in csv.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(sep).collect();

        match f[0] {
            "STEPS" => r.steps = num(f.get(1)),

            "COST" => {
                if f.get(1) == Some(&"COST DISTRIBUTION") {
                    continue;
                }
                r.cost.push(CostRow {
                    label: s(f.get(1)),
                    cost: num(f.get(2)),
                    pct: pct(f.get(3)),
                });
            }

            "RAM USAGE" => r.ram_usage = Usage { used: num(f.get(1)), pct: pct(f.get(2)) },
            "ROM USAGE" => r.rom_usage = Usage { used: num(f.get(1)), pct: pct(f.get(2)) },

            "MEM" => {
                if f.get(1) == Some(&"COST BY TYPE") {
                    continue;
                }
                let row = MemRow {
                    label: s(f.get(1)),
                    count: num(f.get(2)),
                    count_pct: pct(f.get(3)),
                    cost: num(f.get(4)),
                    cost_pct: pct(f.get(5)),
                };
                if row.label.starts_with("TOTAL") {
                    r.mem_totals.push(row);
                } else {
                    r.mem_by_type.push(row);
                }
            }

            "DETAILED_MEM" => {
                if f.get(1) == Some(&"TYPE") {
                    continue;
                }
                r.detailed_mem.push(MemRow {
                    label: s(f.get(1)),
                    count: num(f.get(2)),
                    count_pct: pct(f.get(3)),
                    cost: num(f.get(4)),
                    cost_pct: pct(f.get(5)),
                });
            }

            "DETAILED_MEM FULL" => r.detailed_mem_full.push(MemRow {
                label: s(f.get(1)),
                count: num(f.get(2)),
                count_pct: pct(f.get(3)),
                cost: num(f.get(4)),
                cost_pct: pct(f.get(5)),
            }),

            "MEM_OFFSETS" => {
                if f.get(1).map(|x| x.trim()) == Some("offset") {
                    r.mem_offsets.cols = f.iter().skip(2).map(|x| x.trim().to_string()).collect();
                } else if f.len() >= 2 {
                    let vals: Vec<u64> =
                        f.iter().skip(2).map(|x| x.trim().parse().unwrap_or(0)).collect();
                    r.mem_offsets.rows.push((s(f.get(1)), vals));
                }
            }

            // The per-function memory rankings put the (possibly quoted) function name last, so
            // the fields are split off the front and the remainder is the name — see `tail`.
            "MEM_TOP_COST" => {
                if let Some(v) = tail(line, sep, 5) {
                    r.mem_top_cost.push(MemFnCostRow {
                        cost: n(&v[0]),
                        cost_pct: p(&v[1]),
                        calls: n(&v[2]),
                        cost_per_call: n(&v[3]),
                        name: unquote(&v[4]),
                    });
                }
            }

            "MEM_TOP_UNALIGNED" => {
                if let Some(v) = tail(line, sep, 5) {
                    r.mem_top_unaligned.push(MemFnAlignRow {
                        unaligned: n(&v[0]),
                        aligned: n(&v[1]),
                        unaligned_pct: p(&v[2]),
                        calls: n(&v[3]),
                        name: unquote(&v[4]),
                    });
                }
            }

            "MEM_TOP_RATIO" => {
                if let Some(v) = tail(line, sep, 6) {
                    r.mem_top_ratio.push(MemFnRatioRow {
                        ratio: p(&v[0]),
                        unaligned: n(&v[1]),
                        unaligned_pct: p(&v[2]),
                        accesses_per_call: n(&v[3]),
                        calls: n(&v[4]),
                        name: unquote(&v[5]),
                    });
                }
            }

            "OP_BASE" => {
                if f.get(1) == Some(&"OPCODE") {
                    continue;
                }
                r.op_base.push(OpRow {
                    name: s(f.get(1)),
                    count: num(f.get(2)),
                    count_pct: pct(f.get(3)),
                    cost: num(f.get(4)),
                    cost_pct: pct(f.get(5)),
                });
            }

            "PRECOMPILES" => r.precompiles.push(OpRow {
                name: s(f.get(1)),
                count: num(f.get(2)),
                count_pct: pct(f.get(3)),
                cost: num(f.get(4)),
                cost_pct: pct(f.get(5)),
            }),

            "FROP" => r.frop.push(FropRow {
                name: s(f.get(1)),
                count: num(f.get(2)),
                hit_pct: pct(f.get(3)),
                cost: num(f.get(4)),
                cost_pct: pct(f.get(5)),
            }),

            _ => {}
        }
    }

    r
}

/// Splits the `count` fields that follow the section tag in `line`, keeping the last one whole
/// (the function name, which may itself contain the separator). Returns `None` for the header row,
/// recognised by a first field that is not a number.
fn tail(line: &str, sep: char, count: usize) -> Option<Vec<String>> {
    let rest = line.split_once(sep)?.1;
    let v: Vec<String> = rest.splitn(count, sep).map(|x| x.trim().to_string()).collect();
    if v.len() < count {
        return None;
    }
    // The header row's first field is a label (`MEM COST`, `UNALIGNED`, `RATIO`), not a number.
    v[0].parse::<f64>().ok().map(|_| v)
}

/// Undoes the RFC 4180 quoting the snapshot applies to function names carrying the separator.
fn unquote(field: &str) -> String {
    match field.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        Some(inner) => inner.replace("\"\"", "\""),
        None => field.to_string(),
    }
}

fn n(field: &str) -> u64 {
    field.parse().unwrap_or(0)
}

fn p(field: &str) -> f64 {
    field.trim_end_matches('%').parse().unwrap_or(0.0)
}

fn s(field: Option<&&str>) -> String {
    field.map(|x| x.trim().to_string()).unwrap_or_default()
}

fn num(field: Option<&&str>) -> u64 {
    field.and_then(|x| x.trim().parse().ok()).unwrap_or(0)
}

fn pct(field: Option<&&str>) -> f64 {
    field.map(|x| x.trim().trim_end_matches('%')).and_then(|x| x.parse().ok()).unwrap_or(0.0)
}
