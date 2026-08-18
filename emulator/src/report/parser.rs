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
    pub op_base: Vec<OpRow>,
    pub precompiles: Vec<OpRow>,
    pub frop: Vec<FropRow>,
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

fn s(field: Option<&&str>) -> String {
    field.map(|x| x.trim().to_string()).unwrap_or_default()
}

fn num(field: Option<&&str>) -> u64 {
    field.and_then(|x| x.trim().parse().ok()).unwrap_or(0)
}

fn pct(field: Option<&&str>) -> f64 {
    field.map(|x| x.trim().trim_end_matches('%')).and_then(|x| x.parse().ok()).unwrap_or(0.0)
}
