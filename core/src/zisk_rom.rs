use std::collections::HashMap;

use crate::{ZiskInst, ZiskInstBuilder, ROM_ADDR, ROM_ENTRY, SRC_IND, SRC_STEP};

// #[cfg(feature = "sp")]
// use crate::SRC_SP;

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
#[derive(Default, Debug)]
pub struct ZiskRom {
    pub next_init_inst_addr: u64,
    pub insts: HashMap<u64, ZiskInstBuilder>,
    pub ro_data: Vec<RoData>,
    pub from: u64,
    pub length: u64,
    pub data: Vec<u8>,
    pub rom_entry_instructions: Vec<ZiskInst>,
    pub rom_instructions: Vec<ZiskInst>,
    // Rom Non 4 bytes aligned instructions
    pub offset_rom_na_unstructions: u64,
    pub rom_na_instructions: Vec<ZiskInst>,
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
            offset_rom_na_unstructions: 0,
            rom_na_instructions: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn get_instruction(&self, pc: u64) -> &ZiskInst {
        if pc >= ROM_ADDR {
            if pc & 0b11 == 0 {
                // pc is aligned to a 4-byte boundary
                &self.rom_instructions[((pc - ROM_ADDR) >> 2) as usize]
            } else {
                // pc is not aligned to a 4-byte boundary
                &self.rom_na_instructions[(pc - self.offset_rom_na_unstructions) as usize]
            }
        } else if pc >= ROM_ENTRY {
            // pc is in the ROM_ENTRY range
            &self.rom_entry_instructions[((pc - ROM_ENTRY) >> 2) as usize]
        } else {
            panic!("ZiskRom::get_instruction() pc={} is out of range", pc);
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
            if i.store_ra {
                inst_json["store_ra"] = i.store_ra.into();
            }
            // #[cfg(feature = "sp")]
            // if i.store_use_sp {
            //     inst_json["store_use_sp"] = i.store_use_sp.into();
            // }
            inst_json["store"] = i.store.into();
            if i.store_offset != 0 {
                inst_json["store_offset"] = i.store_offset.into();
            }
            if i.set_pc {
                inst_json["set_pc"] = i.set_pc.into();
            }
            // #[cfg(feature = "sp")]
            // if i.set_sp {
            //     inst_json["set_sp"] = i.set_sp.into();
            // }
            if i.ind_width != 0 {
                inst_json["ind_width"] = i.ind_width.into();
            }
            // #[cfg(feature = "sp")]
            // if i.inc_sp != 0 {
            //     inst_json["inc_sp"] = i.inc_sp.into();
            // }
            if i.end {
                inst_json["end"] = i.end.into();
            }
            if i.a_src != 0 {
                inst_json["a_src"] = i.a_src.into();
            }
            if i.a_src == SRC_STEP {
                inst_json["a_src_step"] = json::JsonValue::from(1);
            }
            // #[cfg(feature = "sp")]
            // if i.a_src == SRC_SP {
            //     inst_json["a_src_sp"] = json::JsonValue::from(1);
            // }
            // #[cfg(feature = "sp")]
            // if i.a_use_sp_imm1 != 0 {
            //     inst_json["a_use_sp_imm1"] = i.a_use_sp_imm1.into();
            // }
            if i.a_offset_imm0 != 0 {
                inst_json["a_offset_imm0"] = i.a_offset_imm0.into();
            }
            if i.b_src != 0 {
                inst_json["b_src"] = i.b_src.into();
            }
            if i.b_src == SRC_IND {
                inst_json["b_src_ind"] = json::JsonValue::from(1);
            }
            // #[cfg(feature = "sp")]
            // if i.b_use_sp_imm1 != 0 {
            //     inst_json["b_use_sp_imm1"] = i.b_use_sp_imm1.into();
            // }
            if i.b_offset_imm0 != 0 {
                inst_json["b_offset_imm0"] = i.b_offset_imm0.into();
            }
            inst_json["is_external_op"] = i.is_external_op.into();
            inst_json["op"] = i.op.into();
            inst_json["opStr"] = i.op_str.into();
            if i.jmp_offset1 != 0 {
                inst_json["jmp_offset1"] = i.jmp_offset1.into();
            }
            if i.jmp_offset2 != 0 {
                inst_json["jmp_offset2"] = i.jmp_offset2.into();
            }
            if !i.verbose.is_empty() {
                inst_json["verbose"] = i.verbose.clone().into();
            }
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

    /// Saves ZisK rom into a PIL data string
    pub fn save_to_pil(&self, s: &mut String) {
        // Clear output data, just in case
        s.clear();

        // Save instructions program addresses into a vector
        let mut keys: Vec<u64> = Vec::new();
        for key in self.insts.keys() {
            keys.push(*key);
        }

        // Sort the vector
        keys.sort();

        // For all program addresses in the vector, create a new PIL line describing the ZisK
        // instruction
        for key in &keys {
            let i = &self.insts[key].i;
            let rom_flags = i.get_flags();

            // #[cfg(feature = "sp")]
            // {
            //     *s += &format!(
            //         "romLine({},{},{},{},{},{},{},{},{},{},{}); // {}: {}\n",
            //         key,
            //         rom_flags,
            //         i.op,
            //         i.a_offset_imm0,
            //         i.b_offset_imm0,
            //         i.ind_width,
            //         i.store_offset,
            //         i.jmp_offset1,
            //         i.jmp_offset2,
            //         i.inc_sp,
            //         i.b_use_sp_imm1,
            //         i.op_str,
            //         i.verbose,
            //     );
            // }

            // #[cfg(not(feature = "sp"))]
            {
                *s += &format!(
                    "romLine({},{},{},{},{},{},{},{},{}); // {}: {}\n",
                    key,
                    rom_flags,
                    i.op,
                    i.a_offset_imm0,
                    i.b_offset_imm0,
                    i.ind_width,
                    i.store_offset,
                    i.jmp_offset1,
                    i.jmp_offset2,
                    i.op_str,
                    i.verbose,
                );
            }
        }
        println!(
            "ZiskRom::save_to_pil() {} bytes, {} instructions, {:02} bytes/inst",
            s.len(),
            keys.len(),
            s.len() as f64 / keys.len() as f64,
        )
    }

    /// Saves ZisK rom into a binary data vector
    pub fn save_to_bin(&self, v: &mut Vec<u8>) {
        // Clear output data, just in case
        v.clear();

        // Save instructions program addresses into a vector
        let mut keys: Vec<u64> = Vec::new();
        for key in self.insts.keys() {
            keys.push(*key);
        }

        // Sort the vector
        keys.sort();

        // For all program addresses in the vector, create a new binary slice describing the ZisK
        // instruction
        for key in &keys {
            let mut aux: [u8; 8];
            let i = &self.insts[key].i;
            let rom_flags = i.get_flags();
            aux = key.to_le_bytes();
            v.extend(aux);
            aux = rom_flags.to_le_bytes();
            v.extend(aux);
            v.push(i.op);
            aux = i.a_offset_imm0.to_le_bytes();
            v.extend(aux);
            aux = i.b_offset_imm0.to_le_bytes();
            v.extend(aux);
            aux = i.ind_width.to_le_bytes();
            v.extend(aux);
            aux = i.store_offset.to_le_bytes();
            v.extend(aux);
            aux = i.jmp_offset1.to_le_bytes();
            v.extend(aux);
            aux = i.jmp_offset2.to_le_bytes();
            v.extend(aux);
            // #[cfg(feature = "sp")]
            // {
            //     aux = i.inc_sp.to_le_bytes();
            //     v.extend(aux);
            //     aux = i.b_use_sp_imm1.to_le_bytes();
            //     v.extend(aux);
            // }
        }
        println!(
            "ZiskRom::save_to_bin() {} bytes, {} instructions, {:02} bytes/inst",
            v.len(),
            keys.len(),
            v.len() as f64 / keys.len() as f64,
        )
    }

    /// Saves ZisK rom into a file: first save to a JSON object, then convert it to string, then
    /// save the string to the file
    pub fn save_to_json_file(&self, file_name: &str) {
        let mut j = json::JsonValue::new_object();
        self.save_to_json(&mut j);
        let s = json::stringify_pretty(j, 1);
        let s_len = s.len();
        let path = std::path::PathBuf::from(file_name);
        let result = std::fs::write(path, s);
        if result.is_err() {
            panic!("ZiskRom::save_to_json_file() failed writing to file={}", file_name);
        }
        println!("ZiskRom::save_to_json_file() {} bytes", s_len);
    }

    /// Saves ZisK rom into a PIL file: first save to a string, then
    /// save the string to the file
    pub fn save_to_pil_file(&self, file_name: &str) {
        // Get a string with the PIL data
        let mut s = String::new();
        self.save_to_pil(&mut s);

        // Save to file
        let path = std::path::PathBuf::from(file_name);
        let result = std::fs::write(path, s);
        if result.is_err() {
            panic!("ZiskRom::save_to_pil_file() failed writing to file={}", file_name);
        }
    }

    /// Saves ZisK rom into a binary file: first save to a vector, then
    /// save the vector to the file
    pub fn save_to_bin_file(&self, file_name: &str) {
        // Get a vector with the ROM data
        let mut v: Vec<u8> = Vec::new();
        self.save_to_bin(&mut v);

        // Save to file
        let path = std::path::PathBuf::from(file_name);
        let result = std::fs::write(path, v);
        if result.is_err() {
            panic!("ZiskRom::save_to_bin_file() failed writing to file={}", file_name);
        }
    }
}
