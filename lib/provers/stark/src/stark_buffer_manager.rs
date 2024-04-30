use std::collections::HashMap;

use log::trace;
use proofman::executor::BufferManager;
use zkevm_lib_c::ffi::{
    chelpers_new_c, get_map_offsets_c, get_map_totaln_c, init_hints_c, set_mapOffsets_c, stark_info_new_c,
};

struct StarkBufferManagerItem {
    stark_info_filename: String,
    zkevm_chelpers: String,
    // p_stark_info: Option<*mut c_void>,
    // p_chelpers: Option<*mut c_void>,
}

impl StarkBufferManagerItem {
    pub fn new(stark_info_filename: &str, zkevm_chelpers: &str) -> Self {
        Self {
            stark_info_filename: stark_info_filename.to_string(),
            zkevm_chelpers: zkevm_chelpers.to_string(),
            // p_stark_info: None,
            // p_chelpers: None,
        }
    }
}

pub struct StarkBufferManager<T> {
    items: HashMap<String, StarkBufferManagerItem>,
    phantom: std::marker::PhantomData<T>,
}

impl<T> StarkBufferManager<T> {
    pub fn new() -> Self {
        Self { items: HashMap::new(), phantom: std::marker::PhantomData }
    }

    pub fn insert_item(&mut self, name: &str, stark_info_filename: &str, zkevm_chelpers: &str) {
        let item = StarkBufferManagerItem::new(stark_info_filename, zkevm_chelpers);
        self.items.insert(name.to_string(), item);
    }
}

impl<T> BufferManager<T> for StarkBufferManager<T> {
    fn get_buffer(&self, name: &str) -> Option<(Vec<u8>, usize)> {
        let item = self.items.get(name)?;

        // if item.p_stark_info.is_none() || item.p_chelpers.is_none() {
        init_hints_c();
        let p_stark_info = stark_info_new_c(item.stark_info_filename.as_str());
        let p_chelpers = chelpers_new_c(item.zkevm_chelpers.as_str());
        // item.p_stark_info = Some(stark_info_new_c(item.stark_info_filename.as_str()));
        // item.p_chelpers = Some(chelpers_new_c(item.zkevm_chelpers.as_str()));
        // }

        // let p_stark_info = item.p_stark_info.unwrap();
        // let p_chelpers = item.p_chelpers.unwrap();

        set_mapOffsets_c(p_stark_info, p_chelpers);

        let map_total_n = get_map_totaln_c(p_stark_info);
        let buffer_size = map_total_n * std::mem::size_of::<T>() as u64;

        trace!("strkbffrmg: Preallocating a buffer of {}bytes", buffer_size);
        let buffer = vec![0u8; buffer_size as usize];

        let offset = get_map_offsets_c(p_stark_info, "cm1", false) as usize * std::mem::size_of::<T>();

        Some((buffer, offset))
    }
}
