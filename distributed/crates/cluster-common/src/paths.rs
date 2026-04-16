use std::path::PathBuf;

/// Returns the content-addressed ELF cache path for a given `hash_id`.
///
/// Path: `~/.zisk/cache/{hash_id}.elf`
pub fn elf_cache_path(hash_id: &str) -> PathBuf {
    let mut base =
        std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"));
    base.push(".zisk");
    base.push("cache");
    base.push(format!("{}.elf", hash_id));
    base
}
