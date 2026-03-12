use memmap2::Mmap;
use object::elf::STT_GNU_IFUNC;
use object::{elf::STT_FUNC, Object, ObjectSymbol, Symbol, SymbolFlags, SymbolKind};
use regex::Regex;

use std::fs::File;
use std::io::Result;

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub is_selected_roi: bool,
}

pub struct ElfSymbolReader {
    functions: Vec<SymbolInfo>,
    profile_tags: Vec<(u16, String)>,
    roi_filter: Option<Regex>,
}

impl Default for ElfSymbolReader {
    fn default() -> Self {
        Self::new()
    }
}
impl ElfSymbolReader {
    pub fn new() -> Self {
        Self { functions: Vec::new(), profile_tags: Vec::new(), roi_filter: None }
    }

    /// Sets a regex filter to mark matching symbols as ROI
    pub fn set_roi_filter(&mut self, pattern: &str) -> std::result::Result<(), regex::Error> {
        self.roi_filter = Some(Regex::new(pattern)?);
        Ok(())
    }

    pub fn load_from_file(&mut self, path: &str) -> Result<()> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        match object::File::parse(&*mmap) {
            Ok(obj) => {
                self.parse_symbols(&obj);
                Ok(())
            }
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        }
    }

    fn demangle_name(&self, name: &str) -> String {
        symbolic_demangle::demangle(name).into_owned()
    }

    fn parse_function_symbol(&mut self, symbol: &Symbol<'_, '_>) {
        if symbol.kind() == SymbolKind::Text {
            if let Ok(name) = symbol.name() {
                if !name.is_empty() {
                    if let SymbolFlags::Elf { st_info, .. } = symbol.flags() {
                        let st_type = st_info & 0x0f;
                        if st_type == STT_FUNC || st_type == STT_GNU_IFUNC {
                            let name = self.demangle_name(name);
                            let address = symbol.address();
                            let size = symbol.size();
                            let is_selected_roi = self
                                .roi_filter
                                .as_ref()
                                .map(|re| re.is_match(&name))
                                .unwrap_or(false);
                            let symbol_info = SymbolInfo { name, address, size, is_selected_roi };
                            self.functions.push(symbol_info);
                        }
                    }
                }
            }
        }
    }

    /// Returns a reference to the profile tags
    pub fn profile_tags(&self) -> &Vec<(u16, String)> {
        &self.profile_tags
    }

    /// Returns the name of a profile tag by its ID
    pub fn get_profile_tag_name(&self, id: u16) -> Option<&str> {
        self.profile_tags.iter().find(|(tag_id, _)| *tag_id == id).map(|(_, name)| name.as_str())
    }

    fn parse_symbols(&mut self, obj: &object::File) {
        // Parse regular symbol table
        for symbol in obj.symbols() {
            self.parse_function_symbol(&symbol);
            self.parse_cost_tag_symbol(&symbol);
        }

        // Parse dynamic symbol table if available
        for symbol in obj.dynamic_symbols() {
            self.parse_function_symbol(&symbol);
            self.parse_cost_tag_symbol(&symbol);
        }
    }

    fn parse_cost_tag_symbol(&mut self, symbol: &Symbol<'_, '_>) {
        if let Ok(name) = symbol.name() {
            if let Some(rest) = name.strip_prefix("__ZISKOS_PROFILE_ID_") {
                // Parse format: __ZISKOS_COST_ID_<id>_<name>
                if let Some(underscore_pos) = rest.find('_') {
                    let id_str = &rest[..underscore_pos];
                    let tag_name = &rest[underscore_pos + 1..];

                    if let Ok(id) = id_str.parse::<u16>() {
                        self.profile_tags.push((id, tag_name.to_string()));
                    }
                }
            }
        }
    }

    /// Returns an iterator over all functions
    pub fn functions(&self) -> impl Iterator<Item = &SymbolInfo> {
        self.functions.iter()
    }

    /// Returns an iterator over all ROI functions (those matching the filter)
    pub fn roi_functions(&self) -> impl Iterator<Item = &SymbolInfo> {
        self.functions.iter().filter(|s| s.is_selected_roi)
    }

    /// Returns the symbol at the given address, if it exists
    pub fn get_symbol_at_address(&self, address: u64) -> Option<&SymbolInfo> {
        self.functions.iter().find(|s| s.address == address)
    }
}
