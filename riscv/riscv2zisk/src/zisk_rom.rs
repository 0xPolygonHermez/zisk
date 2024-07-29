use std::collections::HashMap;

use crate::{ZiskInst, INVALID_VALUE}; /* TODO: Ask Jordi.  b_offset_imm0 is signed, so it could easily
                                       * become 0xFFFFFFFFFFFFFFFF */
use crate::{ZiskInstBuilder, INVALID_VALUE_S64, SRC_IND, SRC_SP, SRC_STEP}; // TODO: Ask Jordi.

/// RO data structure
#[derive(Debug, Default)]
pub struct RoData {
    pub from: u64,
    pub length: usize,
    pub data: Vec<u8>,
}

/// RO data implementation
impl RoData {
    /// Creates a new RoData struct with the provided data
    pub fn new(from: u64, length: usize, data: Vec<u8>) -> RoData {
        RoData { from, length, data }
    }
}

/// ZisK ROM data, including a map address to ZisK instruction
#[derive(Default)]
pub struct ZiskRom {
    pub next_init_inst_addr: u64,
    pub insts: HashMap<u64, ZiskInstBuilder>,
    pub ro_data: Vec<RoData>,
    pub from: u64,
    pub length: u64,
    pub data: Vec<u8>,
    pub rom_entry_instructions: Vec<ZiskInst>,
    pub rom_instructions: Vec<ZiskInst>,
}

/// ZisK ROM implementation
impl ZiskRom {
    pub fn new() -> ZiskRom {
        ZiskRom {
            next_init_inst_addr: 0,
            insts: HashMap::new(),
            ro_data: Vec::new(),
            from: 0,
            length: 0,
            data: Vec::new(),
            rom_entry_instructions: Vec::new(),
            rom_instructions: Vec::new(),
        }
    }

    /// Saves ZisK rom into a JSON object
    pub fn save_to_json(&self, j: &mut json::JsonValue) {
        // Clear output data, just in case
        j.clear();

        // Save next init inst addr
        j["nextInitInstAddr"] = self.next_init_inst_addr.into();

        // Create the insts JSON object
        j["insts"] = json::JsonValue::new_object();

        // Save instructions program addresses into a vector
        let mut keys: Vec<u64> = Vec::new();
        for key in self.insts.keys() {
            keys.push(*key);
        }

        // Sort the vector
        keys.sort();

        // For all program addresses in the vector, create a new JSON object describing the ZisK
        // instruction
        for key in keys {
            let i = &self.insts[&key].i;
            let mut inst_json = json::JsonValue::new_object();
            inst_json["paddr"] = i.paddr.into();
            if i.store_ra != INVALID_VALUE {
                inst_json["store_ra"] = i.store_ra.into();
            }
            if i.store_use_sp != INVALID_VALUE {
                inst_json["store_use_sp"] = i.store_use_sp.into();
            }
            inst_json["store"] = i.store.into();
            if i.store_offset != INVALID_VALUE_S64 {
                inst_json["store_offset"] = i.store_offset.into();
            }
            if i.set_pc != INVALID_VALUE {
                inst_json["set_pc"] = i.set_pc.into();
            }
            if i.set_sp != INVALID_VALUE {
                inst_json["set_sp"] = i.set_sp.into();
            }
            if i.ind_width != INVALID_VALUE {
                inst_json["ind_width"] = i.ind_width.into();
            }
            if i.inc_sp != INVALID_VALUE {
                inst_json["inc_sp"] = i.inc_sp.into();
            }
            if i.end != INVALID_VALUE {
                inst_json["end"] = i.end.into();
            }
            if i.a_src != INVALID_VALUE {
                inst_json["a_src"] = i.a_src.into();
            }
            if i.a_src == SRC_STEP {
                inst_json["a_src_step"] = json::JsonValue::from(1);
            }
            if i.a_src == SRC_SP {
                inst_json["a_src_sp"] = json::JsonValue::from(1);
            }
            if i.a_use_sp_imm1 != INVALID_VALUE {
                inst_json["a_use_sp_imm1"] = i.a_use_sp_imm1.into();
            }
            if i.a_offset_imm0 != INVALID_VALUE {
                inst_json["a_offset_imm0"] = i.a_offset_imm0.into();
            }
            if i.b_src != INVALID_VALUE {
                inst_json["b_src"] = i.b_src.into();
            }
            if i.b_src == SRC_IND {
                inst_json["b_src_ind"] = json::JsonValue::from(1);
            }
            if i.b_use_sp_imm1 != INVALID_VALUE {
                inst_json["b_use_sp_imm1"] = i.b_use_sp_imm1.into();
            }
            if i.b_offset_imm0 != INVALID_VALUE {
                inst_json["b_offset_imm0"] = i.b_offset_imm0.into();
            }
            if i.is_external_op != INVALID_VALUE {
                inst_json["is_external_op"] = i.is_external_op.into();
            }
            inst_json["op"] = i.op.into();
            inst_json["opStr"] = i.op_str.into();
            if i.jmp_offset1 != INVALID_VALUE_S64 {
                inst_json["jmp_offset1"] = i.jmp_offset1.into();
            }
            if i.jmp_offset2 != INVALID_VALUE_S64 {
                inst_json["jmp_offset2"] = i.jmp_offset2.into();
            }
            inst_json["verbose"] = i.verbose.clone().into();
            j["insts"][i.paddr.to_string()] = inst_json;
        }

        // Save RO data
        j["roData"] = json::JsonValue::new_array();
        for ro in &self.ro_data {
            let mut ro_json = json::JsonValue::new_object();
            ro_json["start"] = ro.from.into();
            let mut data_json = json::JsonValue::new_object();
            data_json["type"] = "Buffer".into(); // TODO: Ask Jordi
            data_json["data"] = json::JsonValue::new_array();
            for d in 0..ro.data.len() {
                let _ = data_json["data"].push(ro.data[d]);
            }
            ro_json["data"] = data_json;
            let _ = j["roData"].push(ro_json);
        }
    }

    /// Saves ZisK rom into a file: first save to a JSON object, then convert it to string, then
    /// save the string to the file
    pub fn save_to_file(&self, file_name: &str) {
        let mut j = json::JsonValue::new_object();
        self.save_to_json(&mut j);
        let s = json::stringify_pretty(j, 1);

        let path = std::path::PathBuf::from(file_name);
        let result = std::fs::write(path, s);
        if result.is_err() {
            panic!("ZiskRom::save_to_file() failed writing to file={}", file_name);
        }
    }
}
