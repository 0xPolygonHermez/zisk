use crate::{
    AirGroupMap, AirIdMap, AirInstance, DebugInfo, GlobalInfo, InstanceMap, ModeName, ProofCtx, ProofOptions, StdMode,
    VerboseMode, DEFAULT_PRINT_VALS,
};
use proofman_starks_lib_c::set_log_level_c;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use p3_field::Field;
use serde::Deserialize;
use std::fs;

pub fn add_air_instance<F: Field>(air_instance: AirInstance<F>, pctx: Arc<ProofCtx<F>>) -> bool {
    let (is_mine, gid) = pctx.dctx.write().unwrap().add_instance(
        air_instance.airgroup_id,
        air_instance.air_id,
        pctx.get_weight(air_instance.airgroup_id, air_instance.air_id),
    );
    if is_mine {
        pctx.add_air_instance(air_instance, gid);
    }
    is_mine
}

pub fn initialize_logger(verbose_mode: VerboseMode) {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(verbose_mode.into())
        .init();
    set_log_level_c(verbose_mode.into());
}

pub fn format_bytes(mut num_bytes: f64) -> String {
    let units = ["Bytes", "KB", "MB", "GB"];
    let mut unit_index = 0;

    while num_bytes >= 0.01 && unit_index < units.len() - 1 {
        if num_bytes < 1024.0 {
            break;
        }
        num_bytes /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", num_bytes, units[unit_index])
}

pub fn skip_prover_instance(
    options: ProofOptions,
    airgroup_id: usize,
    air_id: usize,
    air_instance_id: usize,
) -> (bool, Vec<usize>) {
    if options.debug_info.debug_instances.is_empty() {
        return (false, Vec::new());
    } else if let Some(airgroup_id_map) = options.debug_info.debug_instances.get(&airgroup_id) {
        if airgroup_id_map.is_empty() {
            return (false, Vec::new());
        } else if let Some(air_id_map) = airgroup_id_map.get(&air_id) {
            if air_id_map.is_empty() {
                return (false, Vec::new());
            } else if let Some(instance_id_map) = air_id_map.get(&air_instance_id) {
                return (false, instance_id_map.clone());
            }
        }
    }

    (true, Vec::new())
}

#[derive(Debug, Default, Deserialize)]
struct StdDebugMode {
    #[serde(default)]
    opids: Option<Vec<u64>>,
    #[serde(default)]
    n_print: Option<usize>,
    #[serde(default)]
    print_to_file: bool,
}

#[derive(Debug, Deserialize)]
struct DebugJson {
    #[serde(default)]
    constraints: Option<Vec<AirGroupJson>>,
    #[serde(default)]
    global_constraints: Option<Vec<usize>>,
    #[serde(default)]
    std_mode: Option<StdDebugMode>,
}

#[derive(Debug, Deserialize)]
struct AirGroupJson {
    #[serde(default)]
    airgroup_id: Option<usize>,
    #[serde(default)]
    airgroup: Option<String>,
    #[serde(default)]
    air_ids: Option<Vec<AirIdJson>>,
}

#[derive(Debug, Deserialize)]
struct AirIdJson {
    #[serde(default)]
    air_id: Option<usize>,
    #[serde(default)]
    air: Option<String>,
    #[serde(default)]
    instance_ids: Option<Vec<InstanceJson>>,
}

#[derive(Debug, Deserialize)]
struct InstanceJson {
    #[serde(default)]
    instance_id: Option<usize>,
    #[serde(default)]
    constraints: Option<Vec<usize>>,
}

pub fn json_to_debug_instances_map(proving_key_path: PathBuf, json_path: String) -> DebugInfo {
    // Check proving_key_path exists
    if !proving_key_path.exists() {
        panic!("Proving key folder not found at path: {:?}", proving_key_path);
    }

    let global_info: GlobalInfo = GlobalInfo::new(&proving_key_path);

    // Read the file contents
    let debug_json = fs::read_to_string(&json_path).unwrap_or_else(|_| panic!("Failed to read file {}", json_path));

    // Deserialize the JSON into the `DebugJson` struct
    let json: DebugJson = serde_json::from_str(&debug_json)
        .unwrap_or_else(|err| panic!("Failed to parse JSON file: {}: {}", json_path, err));

    // Initialize the airgroup map
    let mut airgroup_map: AirGroupMap = HashMap::new();

    // Populate the airgroup map using the deserialized data
    if let Some(constraints) = json.constraints {
        for airgroup in constraints {
            let mut air_id_map: AirIdMap = HashMap::new();

            if airgroup.airgroup.is_none() && airgroup.airgroup_id.is_none() {
                panic!("Airgroup or airgroup_id must be defined in the JSON file");
            }
            if airgroup.airgroup.is_some() && airgroup.airgroup_id.is_some() {
                panic!("Only airgroup or airgroup_id can be defined in the JSON file, not both");
            }

            let airgroup_id = if airgroup.airgroup_id.is_some() {
                airgroup.airgroup_id.unwrap()
            } else {
                let airgroup_name = airgroup.airgroup.unwrap().to_string();
                let airgroup_id = global_info.air_groups.iter().position(|x| x == &airgroup_name);
                if airgroup_id.is_none() {
                    panic!("Airgroup name {} not found in global_info.airgroups", airgroup_name);
                }
                airgroup_id.unwrap()
            };

            if let Some(air_ids) = airgroup.air_ids {
                for air in air_ids {
                    if air.air.is_none() && air.air_id.is_none() {
                        panic!("Air or air_id must be defined in the JSON file");
                    }
                    if air.air.is_some() && air.air_id.is_some() {
                        panic!("Only air or air_id can be defined in the JSON file, not both");
                    }

                    let air_id = if air.air_id.is_some() {
                        air.air_id.unwrap()
                    } else {
                        let air_name = air.air.unwrap().to_string();
                        let air_id = global_info.airs[airgroup_id].iter().position(|x| x.name == air_name);
                        if air_id.is_none() {
                            panic!("Airgroup name {} not found in global_info.airgroups", air_name);
                        }
                        air_id.unwrap()
                    };

                    let mut instance_map: InstanceMap = HashMap::new();

                    if let Some(instances) = air.instance_ids {
                        for instance in instances {
                            let instance_constraints = instance.constraints.unwrap_or_default();
                            instance_map.insert(instance.instance_id.unwrap_or_default(), instance_constraints);
                        }
                    }

                    air_id_map.insert(air_id, instance_map);
                }
            }

            airgroup_map.insert(airgroup_id, air_id_map);
        }
    }

    // Default global_constraints to an empty Vec if None
    let global_constraints = json.global_constraints.unwrap_or_default();

    let std_mode = if !airgroup_map.is_empty() {
        StdMode::new(ModeName::Standard, Vec::new(), 0, false)
    } else {
        let mode = json.std_mode.unwrap_or_default();
        StdMode::new(
            ModeName::Debug,
            mode.opids.unwrap_or_default(),
            mode.n_print.unwrap_or(DEFAULT_PRINT_VALS),
            mode.print_to_file,
        )
    };

    DebugInfo {
        debug_instances: airgroup_map.clone(),
        debug_global_instances: global_constraints,
        std_mode,
        save_proofs_to_file: true,
    }
}
