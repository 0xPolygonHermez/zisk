use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Database schema for the coordinator
/// This is designed to be easily adaptable to SQLite when needed

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub block_id: u64,
    pub phase: JobPhase,
    pub status: JobStatus,
    pub prover_count: u32,
    pub input_data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub phase1_data: Option<Vec<u64>>, // Combined phase1 results
    pub phase2_data: Option<Vec<u64>>, // Phase2 challenge data
    pub final_proof: Option<Vec<u8>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobPhase {
    Phase0,            // Registration and waiting for provers
    Phase1,            // Sub-job dispatch
    Phase1Aggregation, // Collecting phase1 results
    Phase2,            // Final computation
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverSession {
    pub prover_id: String,
    pub session_id: String,
    pub state: ProverState,
    pub capabilities: ProverCapabilities,
    pub current_job_id: Option<String>,
    pub current_rank_id: Option<u32>,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub last_seen_phase: Option<String>,
    pub jobs_completed: u32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProverState {
    Idle,
    Assigned,
    Computing,
    Error,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverCapabilities {
    pub cpu_cores_num: u32,
    pub gpu_num: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase1Result {
    pub job_id: String,
    pub prover_id: String,
    pub rank_id: u32,
    pub result_data: Vec<u64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAssignment {
    pub job_id: String,
    pub prover_id: String,
    pub rank_id: u32,
    pub assigned_at: DateTime<Utc>,
    pub status: AssignmentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssignmentStatus {
    Assigned,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// In-memory database implementation
/// This will be easily replaceable with SQLite later
#[derive(Debug, Default)]
pub struct InMemoryDatabase {
    pub jobs: HashMap<String, Job>,
    pub prover_sessions: HashMap<String, ProverSession>,
    pub phase1_results: HashMap<String, Vec<Phase1Result>>, // job_id -> results
    pub job_assignments: HashMap<String, Vec<JobAssignment>>, // job_id -> assignments
}

impl InMemoryDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    // Job operations
    pub fn create_job(&mut self, job: Job) -> Result<()> {
        self.jobs.insert(job.job_id.clone(), job);
        Ok(())
    }

    pub fn get_job(&self, job_id: &str) -> Option<&Job> {
        self.jobs.get(job_id)
    }

    pub fn update_job(&mut self, job: Job) -> Result<()> {
        self.jobs.insert(job.job_id.clone(), job);
        Ok(())
    }

    pub fn list_active_jobs(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|job| {
                !matches!(
                    job.status,
                    JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
                )
            })
            .collect()
    }

    // Prover session operations
    pub fn create_prover_session(&mut self, session: ProverSession) -> Result<()> {
        self.prover_sessions.insert(session.prover_id.clone(), session);
        Ok(())
    }

    pub fn get_prover_session(&self, prover_id: &str) -> Option<&ProverSession> {
        self.prover_sessions.get(prover_id)
    }

    pub fn update_prover_session(&mut self, session: ProverSession) -> Result<()> {
        self.prover_sessions.insert(session.prover_id.clone(), session);
        Ok(())
    }

    pub fn list_active_provers(&self) -> Vec<&ProverSession> {
        self.prover_sessions.values().filter(|session| session.is_active).collect()
    }

    pub fn list_idle_provers(&self) -> Vec<&ProverSession> {
        self.prover_sessions
            .values()
            .filter(|session| session.is_active && session.state == ProverState::Idle)
            .collect()
    }

    // Phase1 results operations
    pub fn add_phase1_result(&mut self, result: Phase1Result) -> Result<()> {
        self.phase1_results.entry(result.job_id.clone()).or_default().push(result);
        Ok(())
    }

    pub fn get_phase1_results(&self, job_id: &str) -> Vec<&Phase1Result> {
        self.phase1_results.get(job_id).map(|results| results.iter().collect()).unwrap_or_default()
    }

    // Job assignments operations
    pub fn create_job_assignment(&mut self, assignment: JobAssignment) -> Result<()> {
        self.job_assignments.entry(assignment.job_id.clone()).or_default().push(assignment);
        Ok(())
    }

    pub fn get_job_assignments(&self, job_id: &str) -> Vec<&JobAssignment> {
        self.job_assignments
            .get(job_id)
            .map(|assignments| assignments.iter().collect())
            .unwrap_or_default()
    }

    pub fn update_assignment_status(
        &mut self,
        job_id: &str,
        prover_id: &str,
        status: AssignmentStatus,
    ) -> Result<()> {
        if let Some(assignments) = self.job_assignments.get_mut(job_id) {
            for assignment in assignments.iter_mut() {
                if assignment.prover_id == prover_id {
                    assignment.status = status;
                    break;
                }
            }
        }
        Ok(())
    }

    // Utility methods
    pub fn generate_job_id() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn generate_session_id() -> String {
        Uuid::new_v4().to_string()
    }

    // Recovery operations
    pub fn get_jobs_in_progress(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|job| matches!(job.status, JobStatus::Running | JobStatus::Pending))
            .collect()
    }

    pub fn mark_prover_disconnected(&mut self, prover_id: &str) -> Result<()> {
        if let Some(session) = self.prover_sessions.get_mut(prover_id) {
            session.state = ProverState::Disconnected;
            session.is_active = false;
        }
        Ok(())
    }

    pub fn cleanup_old_sessions(&mut self, timeout_minutes: i64) -> Result<()> {
        let cutoff = Utc::now() - chrono::Duration::minutes(timeout_minutes);

        self.prover_sessions
            .retain(|_, session| session.last_heartbeat > cutoff || session.is_active);

        Ok(())
    }
}

/// Database trait for future SQLite implementation
pub trait Database: Send + Sync {
    fn create_job(&mut self, job: Job) -> Result<()>;
    fn get_job(&self, job_id: &str) -> Option<Job>;
    fn update_job(&mut self, job: Job) -> Result<()>;
    fn list_active_jobs(&self) -> Vec<Job>;

    fn create_prover_session(&mut self, session: ProverSession) -> Result<()>;
    fn get_prover_session(&self, prover_id: &str) -> Option<ProverSession>;
    fn update_prover_session(&mut self, session: ProverSession) -> Result<()>;
    fn list_active_provers(&self) -> Vec<ProverSession>;
    fn list_idle_provers(&self) -> Vec<ProverSession>;

    fn add_phase1_result(&mut self, result: Phase1Result) -> Result<()>;
    fn get_phase1_results(&self, job_id: &str) -> Vec<Phase1Result>;

    fn create_job_assignment(&mut self, assignment: JobAssignment) -> Result<()>;
    fn get_job_assignments(&self, job_id: &str) -> Vec<JobAssignment>;
    fn update_assignment_status(
        &mut self,
        job_id: &str,
        prover_id: &str,
        status: AssignmentStatus,
    ) -> Result<()>;
}
