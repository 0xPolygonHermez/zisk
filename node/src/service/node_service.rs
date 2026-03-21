use std::sync::Arc;
use zisk_distributed_grpc_api as coord;

use crate::cluster::ClusterRegistry;
use crate::coordinator_client::ZiskCoordinatorClient;
use crate::errors::{NodeError, NodeResult};
use crate::service::types::*;
use crate::util::timestamp_to_ms;

pub struct ZiskNodeService {
    coordinator: Option<ZiskCoordinatorClient>,
    #[allow(dead_code)]
    cluster_registry: Option<Arc<ClusterRegistry>>,
}

impl ZiskNodeService {
    pub fn new(
        cluster_registry: Option<Arc<ClusterRegistry>>,
        coordinator: Option<ZiskCoordinatorClient>,
    ) -> Self {
        Self { coordinator, cluster_registry }
    }

    fn coordinator(&self) -> NodeResult<ZiskCoordinatorClient> {
        self.coordinator.clone().ok_or(NodeError::NoCoordinator)
    }
}

// ── Private coordinator → domain type conversions ─────────────────────────────

fn coord_job_to_summary(j: coord::JobStatus) -> NodeResult<JobSummary> {
    let status = JobStatus::from_coordinator(&j.state, Some(&j.phase))?;
    Ok(JobSummary {
        job_id: j.job_id,
        program_id: j.data_id,
        kind: None, // coordinator JobStatus does not carry job kind
        status,
        created_at_ms: j.start_time,
    })
}

fn coord_job_to_info(j: coord::JobStatus) -> NodeResult<JobInfo> {
    let status = JobStatus::from_coordinator(&j.state, Some(&j.phase))?;
    let completed_at_ms =
        if status.is_terminal() { Some(j.start_time + j.duration_ms) } else { None };
    Ok(JobInfo {
        job_id: j.job_id,
        program_id: j.data_id,
        kind: None, // coordinator JobStatus does not carry job kind
        status,
        created_at_ms: j.start_time,
        completed_at_ms,
        result: None, // coordinator JobStatus does not carry proof data
        error: None,  // coordinator JobStatus does not carry error details
    })
}

fn coord_program_to_summary(p: coord::ProgramInfo) -> NodeResult<ProgramSummary> {
    Ok(ProgramSummary {
        program_id: p.program_id,
        hash_id: p.hash_id,
        name: p.name,
        description: p.description,
        author: p.author,
        metadata: p.metadata,
        created_at_ms: timestamp_to_ms(p.created_at),
        status: ProgramStatus::try_from(p.status)?,
    })
}

fn handle_job_status_response(resp: coord::JobStatusResponse) -> NodeResult<JobInfo> {
    match resp.result {
        Some(coord::job_status_response::Result::Job(j)) => coord_job_to_info(j),
        Some(coord::job_status_response::Result::Error(e)) => Err(NodeError::from(e)),
        None => Err(NodeError::InvalidCoordinatorResponse("empty response".into())),
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
        resp.programs.into_iter().map(coord_program_to_summary).collect::<NodeResult<Vec<_>>>()
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
            .transpose()?
            .ok_or_else(|| NodeError::NotFound("program not found".to_string()))
    }

    pub async fn wait_program(&self, program_id: String) -> NodeResult<ProgramSummary> {
        let mut client = self.coordinator()?;
        let resp =
            client.wait_program(coord::WaitProgramRequest { program_id }).await?.into_inner();

        resp.program
            .map(coord_program_to_summary)
            .transpose()?
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
            status: ProgramStatus::try_from(resp.status)?,
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
            status: ProgramStatus::try_from(resp.status)?,
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
                list.jobs.into_iter().map(coord_job_to_summary).collect::<NodeResult<Vec<_>>>()
            }
            Some(coord::jobs_list_response::Result::Error(e)) => Err(NodeError::from(e)),
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
            Some(coord::cancel_job_response::Result::Job(job)) => Ok(CancelJobResult {
                job_id: job.job_id,
                previous_status: JobStatus::from_coordinator(
                    &job.previous_state,
                    job.phase.as_deref(),
                )?,
            }),
            Some(coord::cancel_job_response::Result::Error(e)) => Err(NodeError::from(e)),
            None => Err(NodeError::InvalidCoordinatorResponse("empty response".into())),
        }
    }
}
