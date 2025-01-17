use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use p3_field::PrimeField;
use proofman_common::ProofCtx;
use proofman_hints::{format_vec, HintFieldOutput};

pub type DebugData<F> = Mutex<HashMap<F, HashMap<Vec<HintFieldOutput<F>>, BusValue<F>>>>; // opid -> val -> BusValue

#[derive(Debug)]
pub struct BusValue<F> {
    shared_data: SharedData<F>, // Data shared across all airgroups, airs, and instances
    grouped_data: AirGroupMap,  // Data grouped by: airgroup_id -> air_id -> instance_id -> InstanceData
}

#[derive(Debug)]
struct SharedData<F> {
    direct_was_called: bool,
    num_proves: F,
    num_assumes: F,
}

type AirGroupMap = HashMap<usize, AirMap>;
type AirMap = HashMap<usize, AirData>;

#[derive(Debug)]
struct AirData {
    name_piop: String,
    name_expr: Vec<String>,
    instances: InstanceMap,
}

type InstanceMap = HashMap<usize, InstanceData>;

#[derive(Debug)]
struct InstanceData {
    row_proves: Vec<usize>,
    row_assumes: Vec<usize>,
}

#[allow(clippy::too_many_arguments)]
pub fn update_debug_data<F: PrimeField>(
    debug_data: &DebugData<F>,
    name_piop: &str,
    name_expr: &[String],
    opid: F,
    val: Vec<HintFieldOutput<F>>,
    airgroup_id: usize,
    air_id: usize,
    instance_id: usize,
    row: usize,
    proves: bool,
    times: F,
    is_global: bool,
) {
    let mut bus = debug_data.lock().expect("Bus values missing");

    let bus_opid = bus.entry(opid).or_default();

    let bus_val = bus_opid.entry(val).or_insert_with(|| BusValue {
        shared_data: SharedData { direct_was_called: false, num_proves: F::zero(), num_assumes: F::zero() },
        grouped_data: AirGroupMap::new(),
    });

    let grouped_data = bus_val
        .grouped_data
        .entry(airgroup_id)
        .or_default()
        .entry(air_id)
        .or_insert_with(|| AirData {
            name_piop: name_piop.to_owned(),
            name_expr: name_expr.to_owned(),
            instances: InstanceMap::new(),
        })
        .instances
        .entry(instance_id)
        .or_insert_with(|| InstanceData { row_proves: Vec::new(), row_assumes: Vec::new() });

    // If the value is global but it was already processed, skip it
    if is_global {
        if bus_val.shared_data.direct_was_called {
            return;
        }
        bus_val.shared_data.direct_was_called = true;
    }

    if proves {
        bus_val.shared_data.num_proves += times;
        grouped_data.row_proves.push(row);
    } else {
        assert!(times.is_one(), "The selector value is invalid: expected 1, but received {:?}.", times);
        bus_val.shared_data.num_assumes += times;
        grouped_data.row_assumes.push(row);
    }
}

pub fn print_debug_info<F: PrimeField>(
    pctx: &ProofCtx<F>,
    name: &str,
    max_values_to_print: usize,
    print_to_file: bool,
    debug_data: &DebugData<F>,
) {
    let mut file_path = PathBuf::new();
    let mut output: Box<dyn Write> = Box::new(io::stdout());
    let mut there_are_errors = false;
    let mut bus_vals = debug_data.lock().expect("Bus values missing");
    for (opid, bus) in bus_vals.iter_mut() {
        if bus.iter().any(|(_, v)| v.shared_data.num_proves != v.shared_data.num_assumes) {
            if !there_are_errors {
                // Print to a file if requested
                if print_to_file {
                    let tmp_dir = Path::new("tmp");
                    if !tmp_dir.exists() {
                        match fs::create_dir_all(tmp_dir) {
                            Ok(_) => log::info!("Debug   : Created directory: {:?}", tmp_dir),
                            Err(e) => {
                                eprintln!("Failed to create directory {:?}: {}", tmp_dir, e);
                                std::process::exit(1);
                            }
                        }
                    }

                    file_path = tmp_dir.join(format!("{}_debug.log", name));

                    match File::create(&file_path) {
                        Ok(file) => {
                            output = Box::new(file);
                        }
                        Err(e) => {
                            eprintln!("Failed to create log file at {:?}: {}", file_path, e);
                            std::process::exit(1);
                        }
                    }
                }

                let file_msg = if print_to_file {
                    format!(" Check the {:?} file for more details.", file_path)
                } else {
                    "".to_string()
                };
                log::error!("{}: Some bus values do not match.{}", name, file_msg);

                // Set the flag to avoid printing the error message multiple times
                there_are_errors = true;
            }
            writeln!(output, "\t► Mismatched bus values for opid {}:", opid).expect("Write error");
        } else {
            continue;
        }

        // TODO: Sort unmatching values by the row
        let mut overassumed_values: Vec<(&Vec<HintFieldOutput<F>>, &mut BusValue<F>)> =
            bus.iter_mut().filter(|(_, v)| v.shared_data.num_proves < v.shared_data.num_assumes).collect();
        let len_overassumed = overassumed_values.len();

        if len_overassumed > 0 {
            writeln!(output, "\t  ⁃ There are {} unmatching values thrown as 'assume':", len_overassumed)
                .expect("Write error");
        }

        for (i, (val, data)) in overassumed_values.iter_mut().enumerate() {
            if i == max_values_to_print {
                writeln!(output, "\t      ...").expect("Write error");
                break;
            }
            let shared_data = &data.shared_data;
            let grouped_data = &mut data.grouped_data;
            print_diffs(pctx, val, max_values_to_print, shared_data, grouped_data, false, &mut output);
        }

        if len_overassumed > 0 {
            writeln!(output).expect("Write error");
        }

        // TODO: Sort unmatching values by the row
        let mut overproven_values: Vec<(&Vec<HintFieldOutput<F>>, &mut BusValue<F>)> =
            bus.iter_mut().filter(|(_, v)| v.shared_data.num_proves > v.shared_data.num_assumes).collect();
        let len_overproven = overproven_values.len();

        if len_overproven > 0 {
            writeln!(output, "\t  ⁃ There are {} unmatching values thrown as 'prove':", len_overproven)
                .expect("Write error");
        }

        for (i, (val, data)) in overproven_values.iter_mut().enumerate() {
            if i == max_values_to_print {
                writeln!(output, "\t      ...").expect("Write error");
                break;
            }

            let shared_data = &data.shared_data;
            let grouped_data = &mut data.grouped_data;
            print_diffs(pctx, val, max_values_to_print, shared_data, grouped_data, true, &mut output);
        }

        if len_overproven > 0 {
            writeln!(output).expect("Write error");
        }
    }

    fn print_diffs<F: PrimeField>(
        pctx: &ProofCtx<F>,
        val: &[HintFieldOutput<F>],
        max_values_to_print: usize,
        shared_data: &SharedData<F>,
        grouped_data: &mut AirGroupMap,
        proves: bool,
        output: &mut dyn Write,
    ) {
        let num_assumes = shared_data.num_assumes;
        let num_proves = shared_data.num_proves;

        let num = if proves { num_proves } else { num_assumes };
        let num_str = if num.is_one() { "time" } else { "times" };

        writeln!(output, "\t    ==================================================").expect("Write error");
        writeln!(
            output,
            "\t    • Value:\n\t        {}\n\t      Appears {} {} across the following:",
            format_vec(val),
            num,
            num_str,
        )
        .expect("Write error");

        // Collect and organize rows
        let mut organized_rows = Vec::new();
        for (airgroup_id, air_id_map) in grouped_data.iter_mut() {
            for (air_id, air_data) in air_id_map.iter_mut() {
                for (instance_id, meta_data) in air_data.instances.iter_mut() {
                    let rows = {
                        let rows = if proves { &meta_data.row_proves } else { &meta_data.row_assumes };
                        if rows.is_empty() {
                            continue;
                        }
                        rows.clone()
                    };
                    organized_rows.push((*airgroup_id, *air_id, *instance_id, rows));
                }
            }
        }

        // Sort rows by airgroup_id, air_id, and instance_id
        organized_rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

        // Print grouped rows
        for (airgroup_id, air_id, instance_id, mut rows) in organized_rows {
            let airgroup_name = pctx.global_info.get_air_group_name(airgroup_id);
            let air_name = pctx.global_info.get_air_name(airgroup_id, air_id);
            let piop_name = &grouped_data.get(&airgroup_id).unwrap().get(&air_id).unwrap().name_piop;
            let expr_name = &grouped_data.get(&airgroup_id).unwrap().get(&air_id).unwrap().name_expr;

            rows.sort();
            let rows_display =
                rows.iter().map(|x| x.to_string()).take(max_values_to_print).collect::<Vec<_>>().join(",");

            let truncated = rows.len() > max_values_to_print;
            writeln!(output, "\t        - Airgroup: {} (id: {})", airgroup_name, airgroup_id).expect("Write error");
            writeln!(output, "\t          Air: {} (id: {})", air_name, air_id).expect("Write error");

            writeln!(output, "\t          PIOP: {}", piop_name).expect("Write error");
            writeln!(output, "\t          Expression: {:?}", expr_name).expect("Write error");

            writeln!(
                output,
                "\t          Instance ID: {} | Num: {} | Rows: [{}{}]",
                instance_id,
                rows.len(),
                rows_display,
                if truncated { ",..." } else { "" }
            )
            .expect("Write error");
        }

        writeln!(output, "\t    --------------------------------------------------").expect("Write error");
        let diff = if proves { num_proves - num_assumes } else { num_assumes - num_proves };
        writeln!(
            output,
            "\t    Total Num Assumes: {}.\n\t    Total Num Proves: {}.\n\t    Total Unmatched: {}.",
            num_assumes, num_proves, diff
        )
        .expect("Write error");
        writeln!(output, "\t    ==================================================\n").expect("Write error");
    }
}
