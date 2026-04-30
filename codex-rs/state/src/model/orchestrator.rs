use anyhow::Result;
use anyhow::anyhow;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use super::epoch_millis_to_datetime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceGoalSource {
    Human,
    Orchestrator,
}

impl WorkspaceGoalSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Orchestrator => "orchestrator",
        }
    }
}

impl TryFrom<&str> for WorkspaceGoalSource {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "human" => Ok(Self::Human),
            "orchestrator" => Ok(Self::Orchestrator),
            other => Err(anyhow!("unknown workspace goal source `{other}`")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceGoalStatus {
    Active,
    Paused,
    Complete,
    Archived,
}

impl WorkspaceGoalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Complete => "complete",
            Self::Archived => "archived",
        }
    }
}

impl TryFrom<&str> for WorkspaceGoalStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "complete" => Ok(Self::Complete),
            "archived" => Ok(Self::Archived),
            other => Err(anyhow!("unknown workspace goal status `{other}`")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceGoal {
    pub id: String,
    pub workspace_key: String,
    pub objective: String,
    pub source: WorkspaceGoalSource,
    pub status: WorkspaceGoalStatus,
    pub priority: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub struct WorkspaceGoalRow {
    pub id: String,
    pub workspace_key: String,
    pub objective: String,
    pub source: String,
    pub status: String,
    pub priority: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub completed_at_ms: Option<i64>,
}

impl WorkspaceGoalRow {
    pub(crate) fn try_from_row(row: &SqliteRow) -> Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            workspace_key: row.try_get("workspace_key")?,
            objective: row.try_get("objective")?,
            source: row.try_get("source")?,
            status: row.try_get("status")?,
            priority: row.try_get("priority")?,
            created_at_ms: row.try_get("created_at_ms")?,
            updated_at_ms: row.try_get("updated_at_ms")?,
            completed_at_ms: row.try_get("completed_at_ms")?,
        })
    }
}

impl TryFrom<WorkspaceGoalRow> for WorkspaceGoal {
    type Error = anyhow::Error;

    fn try_from(row: WorkspaceGoalRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            workspace_key: row.workspace_key,
            objective: row.objective,
            source: WorkspaceGoalSource::try_from(row.source.as_str())?,
            status: WorkspaceGoalStatus::try_from(row.status.as_str())?,
            priority: row.priority,
            created_at: epoch_millis_to_datetime(row.created_at_ms)?,
            updated_at: epoch_millis_to_datetime(row.updated_at_ms)?,
            completed_at: row
                .completed_at_ms
                .map(epoch_millis_to_datetime)
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorRunPhase {
    Scout,
    Execution,
    Verification,
    Complete,
}

impl OrchestratorRunPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scout => "scout",
            Self::Execution => "execution",
            Self::Verification => "verification",
            Self::Complete => "complete",
        }
    }
}

impl TryFrom<&str> for OrchestratorRunPhase {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "scout" => Ok(Self::Scout),
            "execution" => Ok(Self::Execution),
            "verification" => Ok(Self::Verification),
            "complete" => Ok(Self::Complete),
            other => Err(anyhow!("unknown orchestrator run phase `{other}`")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorRunStatus {
    Running,
    Complete,
    Failed,
    Cancelled,
    LoopDetected,
}

impl OrchestratorRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Complete => "complete",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::LoopDetected => "loop_detected",
        }
    }
}

impl TryFrom<&str> for OrchestratorRunStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "running" => Ok(Self::Running),
            "complete" => Ok(Self::Complete),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "loop_detected" => Ok(Self::LoopDetected),
            other => Err(anyhow!("unknown orchestrator run status `{other}`")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorOpportunityStatus {
    Proposed,
    Accepted,
    InProgress,
    Complete,
    Rejected,
    Stale,
}

impl OrchestratorOpportunityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::InProgress => "in_progress",
            Self::Complete => "complete",
            Self::Rejected => "rejected",
            Self::Stale => "stale",
        }
    }
}

impl TryFrom<&str> for OrchestratorOpportunityStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "proposed" => Ok(Self::Proposed),
            "accepted" => Ok(Self::Accepted),
            "in_progress" => Ok(Self::InProgress),
            "complete" => Ok(Self::Complete),
            "rejected" => Ok(Self::Rejected),
            "stale" => Ok(Self::Stale),
            other => Err(anyhow!("unknown orchestrator opportunity status `{other}`")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorRisk {
    Low,
    Medium,
    High,
}

impl OrchestratorRisk {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl TryFrom<&str> for OrchestratorRisk {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(anyhow!("unknown orchestrator risk `{other}`")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorLedgerOutcome {
    Attempted,
    Completed,
    Rejected,
    Blocked,
}

impl OrchestratorLedgerOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Attempted => "attempted",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Blocked => "blocked",
        }
    }
}

impl TryFrom<&str> for OrchestratorLedgerOutcome {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "attempted" => Ok(Self::Attempted),
            "completed" => Ok(Self::Completed),
            "rejected" => Ok(Self::Rejected),
            "blocked" => Ok(Self::Blocked),
            other => Err(anyhow!("unknown orchestrator ledger outcome `{other}`")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorSchedule {
    pub id: String,
    pub workspace_key: String,
    pub enabled: bool,
    pub cadence_seconds: i64,
    pub next_run_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub role_override: Option<String>,
    pub model_override: Option<String>,
    pub max_spawned_agents: i64,
    pub max_run_seconds: i64,
    pub base_token_budget: i64,
    pub current_token_budget: i64,
    pub min_token_budget: i64,
    pub max_token_budget: i64,
    pub consecutive_low_value_runs: i64,
    pub consecutive_loop_detections: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct OrchestratorScheduleRow {
    pub id: String,
    pub workspace_key: String,
    pub enabled: i64,
    pub cadence_seconds: i64,
    pub next_run_at_ms: i64,
    pub last_run_at_ms: Option<i64>,
    pub lease_owner: Option<String>,
    pub lease_expires_at_ms: Option<i64>,
    pub role_override: Option<String>,
    pub model_override: Option<String>,
    pub max_spawned_agents: i64,
    pub max_run_seconds: i64,
    pub base_token_budget: i64,
    pub current_token_budget: i64,
    pub min_token_budget: i64,
    pub max_token_budget: i64,
    pub consecutive_low_value_runs: i64,
    pub consecutive_loop_detections: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl OrchestratorScheduleRow {
    pub(crate) fn try_from_row(row: &SqliteRow) -> Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            workspace_key: row.try_get("workspace_key")?,
            enabled: row.try_get("enabled")?,
            cadence_seconds: row.try_get("cadence_seconds")?,
            next_run_at_ms: row.try_get("next_run_at_ms")?,
            last_run_at_ms: row.try_get("last_run_at_ms")?,
            lease_owner: row.try_get("lease_owner")?,
            lease_expires_at_ms: row.try_get("lease_expires_at_ms")?,
            role_override: row.try_get("role_override")?,
            model_override: row.try_get("model_override")?,
            max_spawned_agents: row.try_get("max_spawned_agents")?,
            max_run_seconds: row.try_get("max_run_seconds")?,
            base_token_budget: row.try_get("base_token_budget")?,
            current_token_budget: row.try_get("current_token_budget")?,
            min_token_budget: row.try_get("min_token_budget")?,
            max_token_budget: row.try_get("max_token_budget")?,
            consecutive_low_value_runs: row.try_get("consecutive_low_value_runs")?,
            consecutive_loop_detections: row.try_get("consecutive_loop_detections")?,
            created_at_ms: row.try_get("created_at_ms")?,
            updated_at_ms: row.try_get("updated_at_ms")?,
        })
    }
}

impl TryFrom<OrchestratorScheduleRow> for OrchestratorSchedule {
    type Error = anyhow::Error;

    fn try_from(row: OrchestratorScheduleRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            workspace_key: row.workspace_key,
            enabled: row.enabled != 0,
            cadence_seconds: row.cadence_seconds,
            next_run_at: epoch_millis_to_datetime(row.next_run_at_ms)?,
            last_run_at: row
                .last_run_at_ms
                .map(epoch_millis_to_datetime)
                .transpose()?,
            lease_owner: row.lease_owner,
            lease_expires_at: row
                .lease_expires_at_ms
                .map(epoch_millis_to_datetime)
                .transpose()?,
            role_override: row.role_override,
            model_override: row.model_override,
            max_spawned_agents: row.max_spawned_agents,
            max_run_seconds: row.max_run_seconds,
            base_token_budget: row.base_token_budget,
            current_token_budget: row.current_token_budget,
            min_token_budget: row.min_token_budget,
            max_token_budget: row.max_token_budget,
            consecutive_low_value_runs: row.consecutive_low_value_runs,
            consecutive_loop_detections: row.consecutive_loop_detections,
            created_at: epoch_millis_to_datetime(row.created_at_ms)?,
            updated_at: epoch_millis_to_datetime(row.updated_at_ms)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrchestratorRun {
    pub id: String,
    pub schedule_id: Option<String>,
    pub workspace_key: String,
    pub phase: OrchestratorRunPhase,
    pub status: OrchestratorRunStatus,
    pub scout_tokens: i64,
    pub execution_tokens: i64,
    pub subagent_tokens: i64,
    pub budget_used: i64,
    pub spawned_agent_count: i64,
    pub impact_score: Option<f64>,
    pub summary: Option<String>,
    pub decisions: serde_json::Value,
    pub loop_flags: serde_json::Value,
    pub verifier_outcome: Option<String>,
    pub goal_updates: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub struct OrchestratorRunRow {
    pub id: String,
    pub schedule_id: Option<String>,
    pub workspace_key: String,
    pub phase: String,
    pub status: String,
    pub scout_tokens: i64,
    pub execution_tokens: i64,
    pub subagent_tokens: i64,
    pub budget_used: i64,
    pub spawned_agent_count: i64,
    pub impact_score: Option<f64>,
    pub summary: Option<String>,
    pub decisions_json: String,
    pub loop_flags_json: String,
    pub verifier_outcome: Option<String>,
    pub goal_updates_json: String,
    pub started_at_ms: i64,
    pub completed_at_ms: Option<i64>,
}

impl OrchestratorRunRow {
    pub(crate) fn try_from_row(row: &SqliteRow) -> Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            schedule_id: row.try_get("schedule_id")?,
            workspace_key: row.try_get("workspace_key")?,
            phase: row.try_get("phase")?,
            status: row.try_get("status")?,
            scout_tokens: row.try_get("scout_tokens")?,
            execution_tokens: row.try_get("execution_tokens")?,
            subagent_tokens: row.try_get("subagent_tokens")?,
            budget_used: row.try_get("budget_used")?,
            spawned_agent_count: row.try_get("spawned_agent_count")?,
            impact_score: row.try_get("impact_score")?,
            summary: row.try_get("summary")?,
            decisions_json: row.try_get("decisions_json")?,
            loop_flags_json: row.try_get("loop_flags_json")?,
            verifier_outcome: row.try_get("verifier_outcome")?,
            goal_updates_json: row.try_get("goal_updates_json")?,
            started_at_ms: row.try_get("started_at_ms")?,
            completed_at_ms: row.try_get("completed_at_ms")?,
        })
    }
}

impl TryFrom<OrchestratorRunRow> for OrchestratorRun {
    type Error = anyhow::Error;

    fn try_from(row: OrchestratorRunRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            schedule_id: row.schedule_id,
            workspace_key: row.workspace_key,
            phase: OrchestratorRunPhase::try_from(row.phase.as_str())?,
            status: OrchestratorRunStatus::try_from(row.status.as_str())?,
            scout_tokens: row.scout_tokens,
            execution_tokens: row.execution_tokens,
            subagent_tokens: row.subagent_tokens,
            budget_used: row.budget_used,
            spawned_agent_count: row.spawned_agent_count,
            impact_score: row.impact_score,
            summary: row.summary,
            decisions: serde_json::from_str(&row.decisions_json)?,
            loop_flags: serde_json::from_str(&row.loop_flags_json)?,
            verifier_outcome: row.verifier_outcome,
            goal_updates: serde_json::from_str(&row.goal_updates_json)?,
            started_at: epoch_millis_to_datetime(row.started_at_ms)?,
            completed_at: row
                .completed_at_ms
                .map(epoch_millis_to_datetime)
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrchestratorOpportunity {
    pub id: String,
    pub workspace_key: String,
    pub title: String,
    pub rationale: String,
    pub status: OrchestratorOpportunityStatus,
    pub priority: i64,
    pub impact_score: f64,
    pub confidence: f64,
    pub estimated_cost: i64,
    pub risk: OrchestratorRisk,
    pub dedupe_key: String,
    pub created_by_run_id: Option<String>,
    pub completed_by_run_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct OrchestratorOpportunityRow {
    pub id: String,
    pub workspace_key: String,
    pub title: String,
    pub rationale: String,
    pub status: String,
    pub priority: i64,
    pub impact_score: f64,
    pub confidence: f64,
    pub estimated_cost: i64,
    pub risk: String,
    pub dedupe_key: String,
    pub created_by_run_id: Option<String>,
    pub completed_by_run_id: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl OrchestratorOpportunityRow {
    pub(crate) fn try_from_row(row: &SqliteRow) -> Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            workspace_key: row.try_get("workspace_key")?,
            title: row.try_get("title")?,
            rationale: row.try_get("rationale")?,
            status: row.try_get("status")?,
            priority: row.try_get("priority")?,
            impact_score: row.try_get("impact_score")?,
            confidence: row.try_get("confidence")?,
            estimated_cost: row.try_get("estimated_cost")?,
            risk: row.try_get("risk")?,
            dedupe_key: row.try_get("dedupe_key")?,
            created_by_run_id: row.try_get("created_by_run_id")?,
            completed_by_run_id: row.try_get("completed_by_run_id")?,
            created_at_ms: row.try_get("created_at_ms")?,
            updated_at_ms: row.try_get("updated_at_ms")?,
        })
    }
}

impl TryFrom<OrchestratorOpportunityRow> for OrchestratorOpportunity {
    type Error = anyhow::Error;

    fn try_from(row: OrchestratorOpportunityRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            workspace_key: row.workspace_key,
            title: row.title,
            rationale: row.rationale,
            status: OrchestratorOpportunityStatus::try_from(row.status.as_str())?,
            priority: row.priority,
            impact_score: row.impact_score,
            confidence: row.confidence,
            estimated_cost: row.estimated_cost,
            risk: OrchestratorRisk::try_from(row.risk.as_str())?,
            dedupe_key: row.dedupe_key,
            created_by_run_id: row.created_by_run_id,
            completed_by_run_id: row.completed_by_run_id,
            created_at: epoch_millis_to_datetime(row.created_at_ms)?,
            updated_at: epoch_millis_to_datetime(row.updated_at_ms)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorWorkLedgerEntry {
    pub id: String,
    pub workspace_key: String,
    pub task_title: String,
    pub outcome: OrchestratorLedgerOutcome,
    pub touched_areas: Vec<String>,
    pub blockers: Vec<String>,
    pub dedupe_key: String,
    pub run_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct OrchestratorWorkLedgerRow {
    pub id: String,
    pub workspace_key: String,
    pub task_title: String,
    pub outcome: String,
    pub touched_areas_json: String,
    pub blockers_json: String,
    pub dedupe_key: String,
    pub run_id: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl OrchestratorWorkLedgerRow {
    pub(crate) fn try_from_row(row: &SqliteRow) -> Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            workspace_key: row.try_get("workspace_key")?,
            task_title: row.try_get("task_title")?,
            outcome: row.try_get("outcome")?,
            touched_areas_json: row.try_get("touched_areas_json")?,
            blockers_json: row.try_get("blockers_json")?,
            dedupe_key: row.try_get("dedupe_key")?,
            run_id: row.try_get("run_id")?,
            created_at_ms: row.try_get("created_at_ms")?,
            updated_at_ms: row.try_get("updated_at_ms")?,
        })
    }
}

impl TryFrom<OrchestratorWorkLedgerRow> for OrchestratorWorkLedgerEntry {
    type Error = anyhow::Error;

    fn try_from(row: OrchestratorWorkLedgerRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            workspace_key: row.workspace_key,
            task_title: row.task_title,
            outcome: OrchestratorLedgerOutcome::try_from(row.outcome.as_str())?,
            touched_areas: serde_json::from_str(&row.touched_areas_json)?,
            blockers: serde_json::from_str(&row.blockers_json)?,
            dedupe_key: row.dedupe_key,
            run_id: row.run_id,
            created_at: epoch_millis_to_datetime(row.created_at_ms)?,
            updated_at: epoch_millis_to_datetime(row.updated_at_ms)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestratorScheduleUpsert {
    pub workspace_key: String,
    pub cadence_seconds: i64,
    pub enabled: bool,
    pub role_override: Option<String>,
    pub model_override: Option<String>,
    pub max_spawned_agents: i64,
    pub max_run_seconds: i64,
    pub base_token_budget: i64,
    pub min_token_budget: i64,
    pub max_token_budget: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrchestratorRunCompletion {
    pub phase: OrchestratorRunPhase,
    pub status: OrchestratorRunStatus,
    pub scout_tokens: i64,
    pub execution_tokens: i64,
    pub subagent_tokens: i64,
    pub budget_used: i64,
    pub spawned_agent_count: i64,
    pub impact_score: Option<f64>,
    pub summary: Option<String>,
    pub decisions: serde_json::Value,
    pub loop_flags: serde_json::Value,
    pub verifier_outcome: Option<String>,
    pub goal_updates: serde_json::Value,
}
