use std::{collections::HashMap, sync::Mutex};

use num_traits::ToPrimitive;
use p3_field::PrimeField;
use proofman_hints::{format_vec, HintFieldOutput};

pub type DebugData<F> = Mutex<HashMap<F, HashMap<Vec<HintFieldOutput<F>>, BusValue<F>>>>; // opid -> val -> BusValue

pub struct BusValue<F: PrimeField> {
    pub num_proves: F,
    pub num_assumes: F,
    // meta data
    pub row_proves: Vec<usize>,
    pub row_assumes: Vec<usize>,
}

pub fn print_debug_info<F: PrimeField>(name: &str, max_values_to_print: usize, debug_data: &DebugData<F>) {
    let mut there_are_errors = false;
    let mut bus_vals = debug_data.lock().expect("Bus values missing");
    for (opid, bus) in bus_vals.iter_mut() {
        if bus.iter().any(|(_, v)| v.num_proves != v.num_assumes) {
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
            bus.iter_mut().filter(|(_, v)| v.num_proves < v.num_assumes).collect();
        let len_overassumed = overassumed_values.len();

        if len_overassumed > 0 {
            println!("\t  ⁃ There are {} unmatching values thrown as 'assume':", len_overassumed);
        }

        for (i, (val, data)) in overassumed_values.iter_mut().enumerate() {
            print_diffs(val, max_values_to_print, data.num_assumes, data.num_proves, &mut data.row_assumes, false);

            if i == max_values_to_print {
                println!("\t      ...");
                break;
            }
        }

        if len_overassumed > 0 {
            println!();
        }

        // TODO: Sort unmatching values by the row
        let mut overproven_values: Vec<(&Vec<HintFieldOutput<F>>, &mut BusValue<F>)> =
            bus.iter_mut().filter(|(_, v)| v.num_proves > v.num_assumes).collect();
        let len_overproven = overproven_values.len();

        if len_overproven > 0 {
            println!("\t  ⁃ There are {} unmatching values thrown as 'prove':", len_overproven);
        }

        for (i, (val, data)) in overproven_values.iter_mut().enumerate() {
            print_diffs(val, max_values_to_print, data.num_proves, data.num_assumes, &mut data.row_proves, true);

            if i == max_values_to_print {
                println!("\t      ...");
                break;
            }
        }

        if len_overproven > 0 {
            println!();
        }
    }

    fn print_diffs<F: PrimeField>(
        val: &[HintFieldOutput<F>],
        max_values_to_print: usize,
        num_vals_left: F,
        num_vals_right: F,
        rows: &mut [usize],
        reverse_print: bool,
    ) {
        let diff = num_vals_left - num_vals_right;
        let diff = diff.as_canonical_biguint().to_usize().expect("Cannot convert to usize");

        rows.sort();
        let rows = rows
            .iter()
            .map(|x| x.to_string())
            .take(std::cmp::min(max_values_to_print, diff))
            .collect::<Vec<_>>()
            .join(",");

        let name_str = match rows.len() {
            1 => format!("at row {}.", rows),
            len if max_values_to_print < len => format!("at rows {},...", rows),
            _ => format!("at rows {}.", rows),
        };
        let diff_str = if diff == 1 { "time" } else { "times" };

        let (num_assumes, num_proves) =
            if reverse_print { (num_vals_right, num_vals_left) } else { (num_vals_left, num_vals_right) };
        println!(
            "\t    • Value:\n\t        {}\n\t      Appears {} {} {}\n\t      Num Assumes: {}.\n\t      Num Proves: {}.",
            format_vec(val),
            diff,
            diff_str,
            name_str,
            num_assumes,
            num_proves
        );
    }
}
