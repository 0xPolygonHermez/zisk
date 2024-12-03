use std::{collections::HashMap, sync::Mutex};

use p3_field::PrimeField;
use proofman_hints::{format_vec, HintFieldOutput};

pub type DebugData<F> = Mutex<HashMap<F, HashMap<Vec<HintFieldOutput<F>>, BusValue<F>>>>; // opid -> val -> BusValue

pub struct BusValue<F> {
    shared_data: SharedData<F>, // Data shared across all airgroups, airs, and instances
    grouped_data: AirGroupMap,  // Data grouped by: airgroup_id -> air_id -> instance_id -> MetaData
}

struct SharedData<F> {
    num_proves: F,
    num_assumes: F,
}

type AirGroupMap = HashMap<usize, AirIdMap>;
type AirIdMap = HashMap<usize, InstanceMap>;
type InstanceMap = HashMap<usize, MetaData>;

struct MetaData {
    row_proves: Vec<usize>,
    row_assumes: Vec<usize>,
}

#[allow(clippy::too_many_arguments)]
pub fn update_debug_data<F: PrimeField>(
    debug_data: &DebugData<F>,
    opid: F,
    val: Vec<HintFieldOutput<F>>,
    airgroup_id: usize,
    air_id: usize,
    instance_id: usize,
    row: usize,
    proves: bool,
    times: F,
) {
    let mut bus = debug_data.lock().expect("Bus values missing");

    let bus_opid = bus.entry(opid).or_default();

    let bus_val = bus_opid.entry(val).or_insert_with(|| BusValue {
        shared_data: SharedData { num_proves: F::zero(), num_assumes: F::zero() },
        grouped_data: AirGroupMap::new(),
    });

    let grouped_data = bus_val
        .grouped_data
        .entry(airgroup_id)
        .or_default()
        .entry(air_id)
        .or_default()
        .entry(instance_id)
        .or_insert_with(|| MetaData { row_proves: Vec::new(), row_assumes: Vec::new() });

    if proves {
        bus_val.shared_data.num_proves += times;
        grouped_data.row_proves.push(row);
    } else {
        assert!(times.is_one(), "The selector value is invalid: expected 1, but received {:?}.", times);
        bus_val.shared_data.num_assumes += times;
        grouped_data.row_assumes.push(row);
    }
}

pub fn print_debug_info<F: PrimeField>(name: &str, max_values_to_print: usize, debug_data: &DebugData<F>) {
    let mut there_are_errors = false;
    let mut bus_vals = debug_data.lock().expect("Bus values missing");
    for (opid, bus) in bus_vals.iter_mut() {
        if bus.iter().any(|(_, v)| v.shared_data.num_proves != v.shared_data.num_assumes) {
            if !there_are_errors {
                there_are_errors = true;
                log::error!("{}: Some bus values do not match.", name);
            }
            println!("\t► Mismatched bus values for opid {}:", opid);
        } else {
            continue;
        }

        // TODO: Sort unmatching values by the row
        let mut overassumed_values: Vec<(&Vec<HintFieldOutput<F>>, &mut BusValue<F>)> =
            bus.iter_mut().filter(|(_, v)| v.shared_data.num_proves < v.shared_data.num_assumes).collect();
        let len_overassumed = overassumed_values.len();

        if len_overassumed > 0 {
            println!("\t  ⁃ There are {} unmatching values thrown as 'assume':", len_overassumed);
        }

        for (i, (val, data)) in overassumed_values.iter_mut().enumerate() {
            if i == max_values_to_print {
                println!("\t      ...");
                break;
            }
            let shared_data = &data.shared_data;
            let grouped_data = &mut data.grouped_data;
            print_diffs(val, max_values_to_print, shared_data, grouped_data, false);
        }

        if len_overassumed > 0 {
            println!();
        }

        // TODO: Sort unmatching values by the row
        let mut overproven_values: Vec<(&Vec<HintFieldOutput<F>>, &mut BusValue<F>)> =
            bus.iter_mut().filter(|(_, v)| v.shared_data.num_proves > v.shared_data.num_assumes).collect();
        let len_overproven = overproven_values.len();

        if len_overproven > 0 {
            println!("\t  ⁃ There are {} unmatching values thrown as 'prove':", len_overproven);
        }

        for (i, (val, data)) in overproven_values.iter_mut().enumerate() {
            if i == max_values_to_print {
                println!("\t      ...");
                break;
            }

            let shared_data = &data.shared_data;
            let grouped_data = &mut data.grouped_data;
            print_diffs(val, max_values_to_print, shared_data, grouped_data, true);
        }

        if len_overproven > 0 {
            println!();
        }
    }

    fn print_diffs<F: PrimeField>(
        val: &[HintFieldOutput<F>],
        max_values_to_print: usize,
        shared_data: &SharedData<F>,
        grouped_data: &mut AirGroupMap,
        proves: bool,
    ) {
        let num_assumes = shared_data.num_assumes;
        let num_proves = shared_data.num_proves;

        let num = if proves { num_proves } else { num_assumes };
        let num_str = if num.is_one() { "time" } else { "times" };

        println!("\t    ==================================================");
        println!(
            "\t    • Value:\n\t        {}\n\t      Appears {} {} across the following:",
            format_vec(val),
            num,
            num_str,
        );

        // Collect and organize rows
        let mut organized_rows = Vec::new();
        for (airgroup_id, air_id_map) in grouped_data.iter_mut() {
            for (air_id, instance_map) in air_id_map.iter_mut() {
                for (instance_id, meta_data) in instance_map.iter_mut() {
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
            rows.sort();
            let rows_display =
                rows.iter().map(|x| x.to_string()).take(max_values_to_print).collect::<Vec<_>>().join(",");

            let truncated = rows.len() > max_values_to_print;
            println!(
                "\t        Airgroup: {:<3} | Air: {:<3} | Instance: {:<3} | Num: {:<9} | Rows: [{}{}]",
                airgroup_id,
                air_id,
                instance_id,
                rows.len(),
                rows_display,
                if truncated { ",..." } else { "" },
            );
        }

        println!("\t    --------------------------------------------------");
        let diff = if proves { num_proves - num_assumes } else { num_assumes - num_proves };
        println!(
            "\t    Total Num Assumes: {}.\n\t    Total Num Proves: {}.\n\t    Total Unmatched: {}.",
            num_assumes, num_proves, diff
        );
        println!("\t    ==================================================\n");
    }
}
