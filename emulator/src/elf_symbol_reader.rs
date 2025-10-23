use memmap2::Mmap;
use object::{elf::STT_FUNC, Object, ObjectSymbol, Symbol, SymbolFlags, SymbolKind};
use std::fs::File;
use std::io::Result;

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub address: u64,
    pub size: u64,
}

pub struct ElfSymbolReader {
    functions: Vec<SymbolInfo>,
}

impl Default for ElfSymbolReader {
    fn default() -> Self {
        Self::new()
    }
}
impl ElfSymbolReader {
    pub fn new() -> Self {
        Self { functions: Vec::new() }
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
                        if (st_info & STT_FUNC) != 0 {
                            let name = self.demangle_name(name);
                            let address = symbol.address();
                            let size = symbol.size();
                            let symbol_info = SymbolInfo { name, address, size };
                            self.functions.push(symbol_info);
                        }
                    }
                }
            }
        }
    }

    fn parse_symbols(&mut self, obj: &object::File) {
        // Parse regular symbol table
        for symbol in obj.symbols() {
            self.parse_function_symbol(&symbol);
        }

        // Parse dynamic symbol table if available
        for symbol in obj.dynamic_symbols() {
            self.parse_function_symbol(&symbol);
        }
    }

    /// Returns an iterator over all functions
    pub fn functions(&self) -> impl Iterator<Item = &SymbolInfo> {
        self.functions.iter()
    }
}
