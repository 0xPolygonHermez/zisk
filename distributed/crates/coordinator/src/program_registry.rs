//! Guest program registry for the coordinator.
//!
//! Programs are stored as ELF files on disk at:
//!   `~/.zisk/cache/programs/{name}-{program_id}-{hash_id}.elf`
//!
//! where `program_id` is a UUID (stable across ELF updates) and `hash_id` is
//! the blake3 hash of the ELF bytes. Status is tracked in-memory only — no
//! JSON registry. On coordinator restart the programs dir is scanned and all
//! found programs are loaded as READY. Workers connecting later will receive
//! these programs and re-ack quickly (cache files already exist).

use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::sync::{watch, RwLock};
use tracing::{info, warn};
use zisk_distributed_common::{
    ProgramInfoDto, ProgramLookupDto, ProgramSetupAckDto, ProgramStatusDto, WorkerId,
};

use crate::coordinator_errors::{CoordinatorError, CoordinatorResult};

pub struct RegisterProgramParams {
    pub program_id: String,
    pub hash_id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: Option<String>,
    pub expected_acks: usize,
}

pub struct UpdateProgramParams {
    pub program_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: Option<String>,
    pub new_hash_id: Option<String>,
    pub new_expected_acks: Option<usize>,
}

struct ProgramState {
    program_id: String,
    name: String,
    description: Option<String>,
    author: Option<String>,
    metadata: Option<String>,
    created_at: DateTime<Utc>,
    /// Number of connected workers at the time of registration.
    /// When 0, the program is immediately READY (no workers to wait for).
    expected_acks: usize,
    /// worker_id → success
    received_acks: HashMap<WorkerId, bool>,
    /// Notifies subscribers when status transitions (Provisioning → Ready/Failed).
    status_tx: watch::Sender<ProgramStatusDto>,
}

impl ProgramState {
    fn status(&self) -> ProgramStatusDto {
        if self.received_acks.values().any(|&ok| !ok) {
            return ProgramStatusDto::Failed;
        }
        if self.received_acks.len() >= self.expected_acks {
            ProgramStatusDto::Ready
        } else {
            ProgramStatusDto::Provisioning
        }
    }
}

pub struct ProgramRegistry {
    /// In-memory state keyed by hash_id.
    states: RwLock<HashMap<String, ProgramState>>,
    /// `~/.zisk/cache/programs/`
    programs_dir: PathBuf,
}

impl ProgramRegistry {
    /// Creates a new registry, ensuring the programs directory exists, and scans it
    /// for existing ELF files. Previously registered programs are loaded as READY
    /// (expected_acks = 0) so they are immediately available.
    pub fn new(programs_dir: PathBuf) -> Self {
        if let Err(e) = std::fs::create_dir_all(&programs_dir) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                warn!("Failed to create programs directory {:?}: {}", programs_dir, e);
            }
        }

        let mut states = HashMap::new();
        match std::fs::read_dir(&programs_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("elf") {
                        continue;
                    }
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Some((name, program_id, hash_id)) = parse_program_stem(stem) {
                            info!(
                                "Loaded program from disk: {} ({}) hash {}",
                                name,
                                &program_id[..8],
                                &hash_id[..8]
                            );
                            let (status_tx, _) = watch::channel(ProgramStatusDto::Ready);
                            states.insert(
                                hash_id.to_string(),
                                ProgramState {
                                    program_id: program_id.to_string(),
                                    name: name.to_string(),
                                    description: None,
                                    author: None,
                                    metadata: None,
                                    created_at: Utc::now(),
                                    expected_acks: 0,
                                    received_acks: HashMap::new(),
                                    status_tx,
                                },
                            );
                        }
                    }
                }
            }
            Err(e) => warn!("Failed to scan programs directory: {}", e),
        }

        Self { states: RwLock::new(states), programs_dir }
    }

    /// Returns the canonical ELF path: `programs_dir/{name}-{program_id}-{hash_id}.elf`
    pub fn elf_path(&self, name: &str, program_id: &str, hash_id: &str) -> PathBuf {
        self.programs_dir.join(format!("{name}-{program_id}-{hash_id}.elf"))
    }

    /// Registers a new program. The ELF file must already be written to `elf_path()` before
    /// calling this.
    pub async fn register(&self, params: RegisterProgramParams) -> CoordinatorResult<()> {
        let RegisterProgramParams {
            program_id,
            hash_id,
            name,
            description,
            author,
            metadata,
            expected_acks,
        } = params;
        let mut states = self.states.write().await;
        if states.contains_key(&hash_id) {
            return Err(CoordinatorError::InvalidRequest(format!(
                "Program {hash_id} already registered"
            )));
        }
        if expected_acks == 0 {
            info!("Program {} ({}) registered as READY (no workers)", name, &program_id[..8]);
        } else {
            info!(
                "Program {} ({}) registered, waiting for {} worker acks",
                name,
                &program_id[..8],
                expected_acks
            );
        }
        let initial_status = if expected_acks == 0 {
            ProgramStatusDto::Ready
        } else {
            ProgramStatusDto::Provisioning
        };
        let (status_tx, _) = watch::channel(initial_status);
        states.insert(
            hash_id,
            ProgramState {
                program_id,
                name,
                description,
                author,
                metadata,
                created_at: Utc::now(),
                expected_acks,
                received_acks: HashMap::new(),
                status_tx,
            },
        );
        Ok(())
    }

    /// Records a rom-setup ack from a worker.
    pub async fn ack_worker(&self, dto: &ProgramSetupAckDto, worker_id: &WorkerId) {
        let mut states = self.states.write().await;
        let Some(state) = states.get_mut(&dto.hash_id) else {
            warn!(
                "Received ack for unknown program hash {}",
                &dto.hash_id[..8.min(dto.hash_id.len())]
            );
            return;
        };
        state.received_acks.insert(worker_id.clone(), dto.success);

        let new_status = state.status();
        state.status_tx.send_if_modified(|cur| {
            if *cur == new_status {
                return false;
            }
            match &new_status {
                ProgramStatusDto::Ready => {
                    info!("Program {} ({}) is READY", state.name, &state.program_id[..8]);
                }
                ProgramStatusDto::Failed => {
                    warn!(
                        "Program {} ({}) FAILED on worker {}: {}",
                        state.name,
                        &state.program_id[..8],
                        worker_id,
                        dto.error.as_deref().unwrap_or("")
                    );
                }
                ProgramStatusDto::Provisioning => {}
            }
            *cur = new_status;
            true
        });
    }

    /// Returns the ProgramInfoDto for the given lookup, or None if not found.
    pub async fn get(&self, lookup: &ProgramLookupDto) -> Option<ProgramInfoDto> {
        let states = self.states.read().await;
        match lookup {
            ProgramLookupDto::HashId(hash_id) => {
                states.get(hash_id.as_str()).map(|s| to_dto(hash_id, s))
            }
            ProgramLookupDto::ProgramId(program_id) => states
                .iter()
                .find(|(_, s)| &s.program_id == program_id)
                .map(|(hash_id, s)| to_dto(hash_id, s)),
            ProgramLookupDto::Name(name) => {
                states.iter().find(|(_, s)| &s.name == name).map(|(hash_id, s)| to_dto(hash_id, s))
            }
        }
    }

    /// Returns all registered programs.
    pub async fn list(&self) -> Vec<ProgramInfoDto> {
        let states = self.states.read().await;
        states.iter().map(|(hash_id, s)| to_dto(hash_id, s)).collect()
    }

    /// Updates mutable fields of an existing program. If `new_hash_id` is supplied, the
    /// entry is re-keyed and the caller is responsible for renaming the ELF file and
    /// re-triggering rom-setup. Returns `(program_id, old_hash_id, new_hash_id)`.
    pub async fn update(
        &self,
        params: UpdateProgramParams,
    ) -> CoordinatorResult<(String, String, String)> {
        let UpdateProgramParams {
            program_id,
            name,
            description,
            author,
            metadata,
            new_hash_id,
            new_expected_acks,
        } = params;
        let mut states = self.states.write().await;

        let old_hash_id = states
            .iter()
            .find(|(_, s)| s.program_id == program_id)
            .map(|(k, _)| k.clone())
            .ok_or(CoordinatorError::NotFoundOrInaccessible)?;

        let mut state = states.remove(&old_hash_id).expect("entry confirmed under write lock");

        if let Some(v) = name {
            state.name = v;
        }
        if let Some(v) = description {
            state.description = Some(v);
        }
        if let Some(v) = author {
            state.author = Some(v);
        }
        if let Some(v) = metadata {
            state.metadata = Some(v);
        }

        let final_hash_id = if let Some(h) = new_hash_id {
            if states.contains_key(h.as_str()) {
                // Restore the state we removed before returning the error.
                states.insert(old_hash_id.clone(), state);
                return Err(CoordinatorError::InvalidRequest(format!(
                    "A program with hash_id {h} already exists"
                )));
            }
            if let Some(acks) = new_expected_acks {
                state.expected_acks = acks;
                state.received_acks.clear();
            }
            state.status_tx.send_replace(ProgramStatusDto::Provisioning);
            states.insert(h.clone(), state);
            h
        } else {
            states.insert(old_hash_id.clone(), state);
            old_hash_id.clone()
        };

        info!("Updated program {} → hash {}", &program_id[..8], &final_hash_id[..8]);
        Ok((program_id.to_string(), old_hash_id, final_hash_id))
    }

    /// Removes a program from the registry and deletes its ELF file from disk.
    pub async fn delete(&self, lookup: &ProgramLookupDto) -> CoordinatorResult<()> {
        let mut states = self.states.write().await;

        let hash_id = match lookup {
            ProgramLookupDto::HashId(h) => {
                if states.contains_key(h.as_str()) {
                    h.clone()
                } else {
                    return Err(CoordinatorError::NotFoundOrInaccessible);
                }
            }
            ProgramLookupDto::ProgramId(pid) => states
                .iter()
                .find(|(_, s)| &s.program_id == pid)
                .map(|(k, _)| k.clone())
                .ok_or(CoordinatorError::NotFoundOrInaccessible)?,
            ProgramLookupDto::Name(_) => {
                return Err(CoordinatorError::InvalidRequest(
                    "delete by name is not supported".to_string(),
                ));
            }
        };

        let state = states.remove(&hash_id).expect("entry confirmed under write lock");
        let elf = self.elf_path(&state.name, &state.program_id, &hash_id);
        if elf.exists() {
            if let Err(e) = std::fs::remove_file(&elf) {
                warn!("Failed to delete ELF file {:?}: {}", elf, e);
            }
        }
        info!("Deleted program {} ({})", state.name, &state.program_id[..8]);
        Ok(())
    }

    /// Returns all (hash_id, name, program_id) triples for push_programs_to_worker.
    pub async fn all_entries(&self) -> Vec<(String, String, String)> {
        self.states
            .read()
            .await
            .iter()
            .map(|(hash_id, s)| (hash_id.clone(), s.name.clone(), s.program_id.clone()))
            .collect()
    }

    /// Returns a watch receiver that fires whenever the program's status changes.
    /// Returns None if no program with the given program_id exists.
    pub async fn subscribe_status(
        &self,
        program_id: &str,
    ) -> Option<watch::Receiver<ProgramStatusDto>> {
        let states = self.states.read().await;
        states.values().find(|s| s.program_id == program_id).map(|s| s.status_tx.subscribe())
    }

    pub fn programs_dir(&self) -> &Path {
        &self.programs_dir
    }
}

fn to_dto(hash_id: &str, s: &ProgramState) -> ProgramInfoDto {
    ProgramInfoDto {
        program_id: s.program_id.clone(),
        hash_id: hash_id.to_string(),
        name: s.name.clone(),
        description: s.description.clone(),
        author: s.author.clone(),
        metadata: s.metadata.clone(),
        status: s.status(),
        created_at: s.created_at,
    }
}

/// Parses a program stem of the form `{name}-{program_id}-{hash_id}` where:
///   - hash_id    = 64-char blake3 hex
///   - program_id = 36-char UUID (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`)
///
/// Returns `Some((name, program_id, hash_id))` on success.
fn parse_program_stem(stem: &str) -> Option<(&str, &str, &str)> {
    // Minimum: 1 (name) + 1 (-) + 36 (uuid) + 1 (-) + 64 (hash) = 103 chars
    let len = stem.len();
    if len < 103 {
        return None;
    }
    let hash_id = &stem[len - 64..];
    if !hash_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    if stem.as_bytes()[len - 65] != b'-' {
        return None;
    }
    let program_id = &stem[len - 101..len - 65];
    if program_id.len() != 36 {
        return None;
    }
    if stem.as_bytes()[len - 102] != b'-' {
        return None;
    }
    let name = &stem[..len - 102];
    if name.is_empty() {
        return None;
    }
    Some((name, program_id, hash_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stem_valid() {
        let hash = "a".repeat(64);
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let stem = format!("my-program-{uuid}-{hash}");
        let (name, pid, h) = parse_program_stem(&stem).unwrap();
        assert_eq!(name, "my-program");
        assert_eq!(pid, uuid);
        assert_eq!(h, hash);
    }

    #[test]
    fn parse_stem_too_short() {
        assert!(parse_program_stem("short").is_none());
    }

    #[test]
    fn parse_stem_name_with_dashes() {
        let hash = "b".repeat(64);
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let stem = format!("my-complex-name-{uuid}-{hash}");
        let (name, pid, h) = parse_program_stem(&stem).unwrap();
        assert_eq!(name, "my-complex-name");
        assert_eq!(pid, uuid);
        assert_eq!(h, hash);
    }
}
