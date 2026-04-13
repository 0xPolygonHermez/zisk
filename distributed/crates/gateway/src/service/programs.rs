//! RegisterGuestProgram handler and proto↔domain conversions for programs.

use std::sync::Arc;
use tonic::Status;
use tracing::instrument;

use crate::backend::BackendService;
use crate::proto::{RegisterGuestProgramRequest, RegisterGuestProgramResponse};

#[instrument(skip(backend, req), fields(elf_bytes = req.zisk_elf.len()))]
pub async fn handle_register_guest_program<B: BackendService>(
    backend: &Arc<B>,
    req: RegisterGuestProgramRequest,
) -> Result<RegisterGuestProgramResponse, Status> {
    if req.zisk_elf.is_empty() {
        return Err(Status::invalid_argument("zisk_elf must not be empty"));
    }

    let hash_id = backend.register_guest_program(req.zisk_elf).await.map_err(Status::from)?;

    Ok(RegisterGuestProgramResponse { hash_id })
}
