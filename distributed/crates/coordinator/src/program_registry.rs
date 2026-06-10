use std::collections::{HashMap, HashSet};

pub(crate) const DEFAULT_MAX_PROGRAMS: usize = 50;
pub(crate) const UNKNOWN_PROGRAM_ALIAS: &str = "unknown";

pub(crate) fn default_alias_for_hash(hash_id: &str) -> String {
    if hash_id.is_empty() {
        return UNKNOWN_PROGRAM_ALIAS.to_owned();
    }
    hash_id[..byte_index_after_chars(hash_id, 8)].to_owned()
}

#[derive(Debug)]
pub(crate) struct ProgramRegistry {
    max_programs: usize,
    aliases_by_hash: HashMap<String, String>,
    aliases: HashSet<String>,
}

impl Default for ProgramRegistry {
    fn default() -> Self {
        Self::with_max(DEFAULT_MAX_PROGRAMS)
    }
}

impl ProgramRegistry {
    pub(crate) fn with_max(max_programs: usize) -> Self {
        Self { max_programs, aliases_by_hash: HashMap::new(), aliases: HashSet::new() }
    }

    pub(crate) fn get(&self, hash_id: &str) -> Option<&str> {
        self.aliases_by_hash.get(hash_id).map(String::as_str)
    }

    pub(crate) fn register(&mut self, hash_id: &str) -> Option<String> {
        if hash_id.is_empty() {
            return Some(UNKNOWN_PROGRAM_ALIAS.to_owned());
        }
        if let Some(alias) = self.aliases_by_hash.get(hash_id) {
            return Some(alias.clone());
        }

        if self.aliases_by_hash.len() >= self.max_programs {
            return None;
        }

        let alias = self.next_alias(hash_id);
        self.aliases.insert(alias.clone());
        self.aliases_by_hash.insert(hash_id.to_owned(), alias.clone());
        Some(alias)
    }

    fn next_alias(&self, hash_id: &str) -> String {
        let mut end = byte_index_after_chars(hash_id, 8);
        loop {
            let candidate = hash_id[..end].to_owned();
            if !self.aliases.contains(&candidate) {
                return candidate;
            }
            if end == hash_id.len() {
                return hash_id.to_owned();
            }
            end = next_char_boundary(hash_id, end);
        }
    }
}

fn byte_index_after_chars(value: &str, count: usize) -> usize {
    value.char_indices().nth(count).map(|(idx, _)| idx).unwrap_or(value.len())
}

fn next_char_boundary(value: &str, current: usize) -> usize {
    value[current..]
        .char_indices()
        .nth(1)
        .map(|(offset, _)| current + offset)
        .unwrap_or(value.len())
}

#[cfg(test)]
#[path = "../tests/unit/program_registry.rs"]
mod tests;
