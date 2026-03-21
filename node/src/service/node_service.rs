use std::sync::Arc;
use zisk_distributed_grpc_api as coord;

use crate::cluster::ClusterRegistry;
use crate::coordinator_client::CoordinatorClient;
use crate::errors::{NodeError, NodeResult};
use crate::service::types::*;

pub struct ZiskNodeService {
    coordinator: Option<CoordinatorClient>,
    #[allow(dead_code)]
    cluster_registry: Option<Arc<ClusterRegistry>>,
}

impl ZiskNodeService {
    pub fn new(
        cluster_registry: Option<Arc<ClusterRegistry>>,
        coordinator: Option<CoordinatorClient>,
    ) -> Self {
        Self { coordinator, cluster_registry }
    }

    fn coordinator(&self) -> NodeResult<CoordinatorClient> {
        self.coordinator.clone().ok_or(NodeError::NoCoordinator)
    }
}

// ── Private coordinator → domain type conversions ─────────────────────────────

fn map_program_status(raw: i32) -> ProgramStatus {
    match coord::ProgramStatus::try_from(raw) {
        Ok(coord::ProgramStatus::Ready) => ProgramStatus::Ready,
        Ok(coord::ProgramStatus::Failed) => ProgramStatus::Failed,
        _ => ProgramStatus::Provisioning,
    }
}

fn timestamp_to_ms(ts: Option<prost_types::Timestamp>) -> Option<u64> {
    ts.map(|t| (t.seconds as u64) * 1000 + (t.nanos as u64) / 1_000_000)
}

fn map_job_status(state: &str) -> JobStatusCode {
    match state {
        "Created" => JobStatusCode::Queued,
        s if s.starts_with("Running") => JobStatusCode::Running,
        "Completed" => JobStatusCode::Completed,
        "Failed" => JobStatusCode::Failed,
        "Cancelled" => JobStatusCode::Cancelled,
        _ => JobStatusCode::Unspecified,
    }
}

fn map_job_phase(phase: &str) -> JobPhase {
    match phase {
        "Prove" => JobPhase::Prove,
        "Aggregate" => JobPhase::Aggregate,
        _ => JobPhase::Contributions,
    }
}

fn is_terminal(code: &JobStatusCode) -> bool {
    matches!(code, JobStatusCode::Completed | JobStatusCode::Failed | JobStatusCode::Cancelled)
}

fn coord_job_to_summary(j: coord::JobStatus) -> JobSummary {
    JobSummary {
        job_id: j.job_id,
        program_id: j.data_id,
        kind: None, // coordinator JobStatus does not carry job kind
        status_code: map_job_status(&j.state),
        phase: map_job_phase(&j.phase),
        created_at_ms: j.start_time,
    }
}

fn coord_job_to_info(j: coord::JobStatus) -> JobInfo {
    let status_code = map_job_status(&j.state);
    let completed_at_ms =
        if is_terminal(&status_code) { Some(j.start_time + j.duration_ms) } else { None };

    JobInfo {
        job_id: j.job_id,
        program_id: j.data_id,
        kind: None, // coordinator JobStatus does not carry job kind
        status_code,
        phase: map_job_phase(&j.phase),
        created_at_ms: j.start_time,
        completed_at_ms,
        result: None, // coordinator JobStatus does not carry proof data
        error: None,  // coordinator JobStatus does not carry error details
    }
}

fn coord_program_to_summary(p: coord::ProgramInfo) -> ProgramSummary {
    ProgramSummary {
        program_id: p.program_id,
        hash_id: p.hash_id,
        name: p.name,
        description: p.description,
        author: p.author,
        metadata: p.metadata,
        created_at_ms: timestamp_to_ms(p.created_at),
        status: map_program_status(p.status),
    }
}

fn map_coord_error(e: coord::ErrorResponse) -> NodeError {
    NodeError::CoordinatorError { code: e.code, message: e.message }
}

fn handle_job_status_response(resp: coord::JobStatusResponse) -> NodeResult<JobInfo> {
    match resp.result {
        Some(coord::job_status_response::Result::Job(j)) => Ok(coord_job_to_info(j)),
        Some(coord::job_status_response::Result::Error(e)) => Err(map_coord_error(e)),
        None => Err(NodeError::EmptyCoordinatorResponse),
    }
}

// ── Public service API ────────────────────────────────────────────────────────

impl ZiskNodeService {
    pub async fn get_node_info(&self) -> NodeResult<NodeVersionInfo> {
        Ok(NodeVersionInfo {
            zisk_version: env!("CARGO_PKG_VERSION").to_string(),
            available_setups: vec![], // TODO: populate from cluster registry
        })
    }

    pub async fn list_programs(&self) -> NodeResult<Vec<ProgramSummary>> {
        let mut client = self.coordinator()?;
        let resp = client.list_programs(coord::ListProgramsRequest {}).await?.into_inner();
        Ok(resp.programs.into_iter().map(coord_program_to_summary).collect())
    }

    pub async fn get_program(&self, lookup: ProgramLookup) -> NodeResult<ProgramSummary> {
        use coord::get_program_request::Lookup as CoordLookup;

        let coord_lookup = match lookup {
            ProgramLookup::ProgramId(v) => CoordLookup::ProgramId(v),
            ProgramLookup::HashId(v) => CoordLookup::HashId(v),
            ProgramLookup::Name(v) => CoordLookup::Name(v),
        };

        let mut client = self.coordinator()?;
        let resp = client
            .get_program(coord::GetProgramRequest { lookup: Some(coord_lookup) })
            .await?
            .into_inner();

        resp.program
            .map(coord_program_to_summary)
            .ok_or_else(|| NodeError::NotFound("program not found".to_string()))
    }

    pub async fn wait_program(&self, program_id: String) -> NodeResult<ProgramSummary> {
        let mut client = self.coordinator()?;
        let resp =
            client.wait_program(coord::WaitProgramRequest { program_id }).await?.into_inner();

        resp.program
            .map(coord_program_to_summary)
            .ok_or_else(|| NodeError::NotFound("program not found".to_string()))
    }

    pub async fn register_program(
        &self,
        params: RegisterProgramParams,
    ) -> NodeResult<RegisterProgramResult> {
        let mut client = self.coordinator()?;
        let resp = client
            .register_program(coord::RegisterProgramRequest {
                name: params.name,
                description: params.description,
                author: params.author,
                zisk_elf: params.zisk_elf,
                metadata: params.metadata,
            })
            .await?
            .into_inner();

        Ok(RegisterProgramResult {
            program_id: resp.program_id,
            hash_id: resp.hash_id,
            status: map_program_status(resp.status),
        })
    }

    pub async fn update_program(
        &self,
        params: UpdateProgramParams,
    ) -> NodeResult<UpdateProgramResult> {
        let mut client = self.coordinator()?;
        let resp = client
            .update_program(coord::UpdateProgramRequest {
                program_id: params.program_id,
                name: params.name,
                description: params.description,
                author: params.author,
                metadata: params.metadata,
                zisk_elf: params.zisk_elf,
            })
            .await?
            .into_inner();

        Ok(UpdateProgramResult {
            program_id: resp.program_id,
            hash_id: resp.hash_id,
            status: map_program_status(resp.status),
        })
    }

    pub async fn delete_program(&self, lookup: ProgramOrHashLookup) -> NodeResult<()> {
        use coord::delete_program_request::Lookup as CoordLookup;

        let coord_lookup = match lookup {
            ProgramOrHashLookup::ProgramId(v) => CoordLookup::ProgramId(v),
            ProgramOrHashLookup::HashId(v) => CoordLookup::HashId(v),
        };

        let mut client = self.coordinator()?;
        client.delete_program(coord::DeleteProgramRequest { lookup: Some(coord_lookup) }).await?;
        Ok(())
    }

    pub async fn list_jobs(&self) -> NodeResult<Vec<JobSummary>> {
        let mut client = self.coordinator()?;
        let resp =
            client.jobs_list(coord::JobsListRequest { active_only: false }).await?.into_inner();

        match resp.result {
            Some(coord::jobs_list_response::Result::JobsList(list)) => {
                Ok(list.jobs.into_iter().map(coord_job_to_summary).collect())
            }
            Some(coord::jobs_list_response::Result::Error(e)) => Err(map_coord_error(e)),
            None => Ok(vec![]),
        }
    }

    pub async fn get_job(&self, job_id: String) -> NodeResult<JobInfo> {
        let mut client = self.coordinator()?;
        let resp = client.job_status(coord::JobStatusRequest { job_id }).await?.into_inner();
        handle_job_status_response(resp)
    }

    pub async fn wait_job(&self, job_id: String) -> NodeResult<JobInfo> {
        let mut client = self.coordinator()?;
        let resp = client.wait_job(coord::WaitJobRequest { job_id }).await?.into_inner();
        handle_job_status_response(resp)
    }

    pub async fn cancel_job(&self, job_id: String) -> NodeResult<CancelJobResult> {
        let mut client = self.coordinator()?;
        let resp =
            client.cancel_job(coord::CancelJobRequest { job_id, reason: None }).await?.into_inner();

        match resp.result {
            Some(coord::cancel_job_response::Result::JobId(id)) => Ok(CancelJobResult {
                job_id: id,
                status_code: JobStatusCode::Cancelled,
                phase: JobPhase::Contributions,
            }),
            Some(coord::cancel_job_response::Result::Error(e)) => Err(map_coord_error(e)),
            None => Err(NodeError::EmptyCoordinatorResponse),
        }
    }
}
