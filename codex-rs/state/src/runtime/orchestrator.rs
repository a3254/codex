use super::*;
use uuid::Uuid;

const DEFAULT_SCHEDULE_ID: &str = "default";
const LOOP_PAUSE_THRESHOLD: i64 = 3;

pub struct WorkspaceGoalUpdate {
    pub status: Option<crate::WorkspaceGoalStatus>,
    pub priority: Option<i64>,
}

pub struct OrchestratorOpportunityUpsert {
    pub workspace_key: String,
    pub title: String,
    pub rationale: String,
    pub status: crate::OrchestratorOpportunityStatus,
    pub priority: i64,
    pub impact_score: f64,
    pub confidence: f64,
    pub estimated_cost: i64,
    pub risk: crate::OrchestratorRisk,
    pub dedupe_key: String,
    pub created_by_run_id: Option<String>,
}

impl StateRuntime {
    pub async fn create_workspace_goal(
        &self,
        workspace_key: &str,
        objective: &str,
        source: crate::WorkspaceGoalSource,
        priority: i64,
    ) -> anyhow::Result<crate::WorkspaceGoal> {
        let id = Uuid::new_v4().to_string();
        let now_ms = datetime_to_epoch_millis(Utc::now());
        let row = sqlx::query(
            r#"
INSERT INTO workspace_goals (
    id,
    workspace_key,
    objective,
    source,
    status,
    priority,
    created_at_ms,
    updated_at_ms,
    completed_at_ms
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)
RETURNING
    id,
    workspace_key,
    objective,
    source,
    status,
    priority,
    created_at_ms,
    updated_at_ms,
    completed_at_ms
            "#,
        )
        .bind(id)
        .bind(workspace_key)
        .bind(objective)
        .bind(source.as_str())
        .bind(crate::WorkspaceGoalStatus::Active.as_str())
        .bind(priority)
        .bind(now_ms)
        .bind(now_ms)
        .fetch_one(self.pool.as_ref())
        .await?;

        WorkspaceGoalRow::try_from_row(&row).and_then(crate::WorkspaceGoal::try_from)
    }

    pub async fn list_workspace_goals(
        &self,
        workspace_key: &str,
        statuses: &[crate::WorkspaceGoalStatus],
    ) -> anyhow::Result<Vec<crate::WorkspaceGoal>> {
        let mut query = QueryBuilder::new(
            r#"
SELECT
    id,
    workspace_key,
    objective,
    source,
    status,
    priority,
    created_at_ms,
    updated_at_ms,
    completed_at_ms
FROM workspace_goals
WHERE workspace_key =
            "#,
        );
        query.push_bind(workspace_key);
        if !statuses.is_empty() {
            query.push(" AND status IN (");
            let mut separated = query.separated(", ");
            for status in statuses {
                separated.push_bind(status.as_str());
            }
            separated.push_unseparated(")");
        }
        query.push(" ORDER BY priority DESC, updated_at_ms DESC");
        let rows = query.build().fetch_all(self.pool.as_ref()).await?;
        rows.into_iter()
            .map(|row| {
                WorkspaceGoalRow::try_from_row(&row).and_then(crate::WorkspaceGoal::try_from)
            })
            .collect()
    }

    pub async fn update_workspace_goal(
        &self,
        id: &str,
        update: WorkspaceGoalUpdate,
    ) -> anyhow::Result<Option<crate::WorkspaceGoal>> {
        let now_ms = datetime_to_epoch_millis(Utc::now());
        let status = update.status.map(|status| status.as_str());
        let completed_at_ms = match update.status {
            Some(crate::WorkspaceGoalStatus::Complete) => Some(now_ms),
            _ => None,
        };
        let row = sqlx::query(
            r#"
UPDATE workspace_goals
SET
    status = COALESCE(?, status),
    priority = COALESCE(?, priority),
    completed_at_ms = CASE
        WHEN ? = 'complete' THEN ?
        WHEN ? IN ('active', 'paused') THEN NULL
        ELSE completed_at_ms
    END,
    updated_at_ms = ?
WHERE id = ?
RETURNING
    id,
    workspace_key,
    objective,
    source,
    status,
    priority,
    created_at_ms,
    updated_at_ms,
    completed_at_ms
            "#,
        )
        .bind(status)
        .bind(update.priority)
        .bind(status)
        .bind(completed_at_ms)
        .bind(status)
        .bind(now_ms)
        .bind(id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        row.map(|row| WorkspaceGoalRow::try_from_row(&row).and_then(crate::WorkspaceGoal::try_from))
            .transpose()
    }

    pub async fn archive_workspace_goal(
        &self,
        id: &str,
    ) -> anyhow::Result<Option<crate::WorkspaceGoal>> {
        self.update_workspace_goal(
            id,
            WorkspaceGoalUpdate {
                status: Some(crate::WorkspaceGoalStatus::Archived),
                priority: None,
            },
        )
        .await
    }

    pub async fn upsert_orchestrator_schedule(
        &self,
        params: &crate::OrchestratorScheduleUpsert,
    ) -> anyhow::Result<crate::OrchestratorSchedule> {
        validate_schedule_params(params)?;
        let now_ms = datetime_to_epoch_millis(Utc::now());
        let id = format!("{}:{DEFAULT_SCHEDULE_ID}", params.workspace_key);
        let enabled = i64::from(params.enabled);
        let row = sqlx::query(
            r#"
INSERT INTO orchestrator_schedules (
    id,
    workspace_key,
    enabled,
    cadence_seconds,
    next_run_at_ms,
    last_run_at_ms,
    lease_owner,
    lease_expires_at_ms,
    role_override,
    model_override,
    max_spawned_agents,
    max_run_seconds,
    base_token_budget,
    current_token_budget,
    min_token_budget,
    max_token_budget,
    consecutive_low_value_runs,
    consecutive_loop_detections,
    created_at_ms,
    updated_at_ms
) VALUES (?, ?, ?, ?, ?, NULL, NULL, NULL, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, ?, ?)
ON CONFLICT(id) DO UPDATE SET
    enabled = excluded.enabled,
    cadence_seconds = excluded.cadence_seconds,
    next_run_at_ms = CASE
        WHEN orchestrator_schedules.enabled = 0 AND excluded.enabled = 1 THEN excluded.next_run_at_ms
        ELSE orchestrator_schedules.next_run_at_ms
    END,
    role_override = excluded.role_override,
    model_override = excluded.model_override,
    max_spawned_agents = excluded.max_spawned_agents,
    max_run_seconds = excluded.max_run_seconds,
    base_token_budget = excluded.base_token_budget,
    current_token_budget = MIN(excluded.max_token_budget, MAX(excluded.min_token_budget, orchestrator_schedules.current_token_budget)),
    min_token_budget = excluded.min_token_budget,
    max_token_budget = excluded.max_token_budget,
    updated_at_ms = excluded.updated_at_ms
RETURNING
    id,
    workspace_key,
    enabled,
    cadence_seconds,
    next_run_at_ms,
    last_run_at_ms,
    lease_owner,
    lease_expires_at_ms,
    role_override,
    model_override,
    max_spawned_agents,
    max_run_seconds,
    base_token_budget,
    current_token_budget,
    min_token_budget,
    max_token_budget,
    consecutive_low_value_runs,
    consecutive_loop_detections,
    created_at_ms,
    updated_at_ms
            "#,
        )
        .bind(id)
        .bind(params.workspace_key.as_str())
        .bind(enabled)
        .bind(params.cadence_seconds)
        .bind(now_ms)
        .bind(params.role_override.as_deref())
        .bind(params.model_override.as_deref())
        .bind(params.max_spawned_agents)
        .bind(params.max_run_seconds)
        .bind(params.base_token_budget)
        .bind(params.base_token_budget)
        .bind(params.min_token_budget)
        .bind(params.max_token_budget)
        .bind(now_ms)
        .bind(now_ms)
        .fetch_one(self.pool.as_ref())
        .await?;

        OrchestratorScheduleRow::try_from_row(&row).and_then(crate::OrchestratorSchedule::try_from)
    }

    pub async fn list_orchestrator_schedules(
        &self,
        workspace_key: &str,
    ) -> anyhow::Result<Vec<crate::OrchestratorSchedule>> {
        let rows = sqlx::query(
            r#"
SELECT
    id,
    workspace_key,
    enabled,
    cadence_seconds,
    next_run_at_ms,
    last_run_at_ms,
    lease_owner,
    lease_expires_at_ms,
    role_override,
    model_override,
    max_spawned_agents,
    max_run_seconds,
    base_token_budget,
    current_token_budget,
    min_token_budget,
    max_token_budget,
    consecutive_low_value_runs,
    consecutive_loop_detections,
    created_at_ms,
    updated_at_ms
FROM orchestrator_schedules
WHERE workspace_key = ?
ORDER BY id
            "#,
        )
        .bind(workspace_key)
        .fetch_all(self.pool.as_ref())
        .await?;

        rows.into_iter()
            .map(|row| {
                OrchestratorScheduleRow::try_from_row(&row)
                    .and_then(crate::OrchestratorSchedule::try_from)
            })
            .collect()
    }

    pub async fn delete_orchestrator_schedule(&self, id: &str) -> anyhow::Result<bool> {
        let result = sqlx::query("DELETE FROM orchestrator_schedules WHERE id = ?")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn acquire_due_orchestrator_schedule(
        &self,
        workspace_key: &str,
        lease_owner: &str,
        lease_duration_ms: i64,
    ) -> anyhow::Result<Option<crate::OrchestratorSchedule>> {
        let now_ms = datetime_to_epoch_millis(Utc::now());
        let lease_expires_at_ms = now_ms + lease_duration_ms.max(1);
        let mut tx = self.pool.begin().await?;
        let id: Option<String> = sqlx::query_scalar(
            r#"
SELECT id
FROM orchestrator_schedules
WHERE workspace_key = ?
  AND enabled = 1
  AND next_run_at_ms <= ?
  AND (lease_expires_at_ms IS NULL OR lease_expires_at_ms <= ?)
ORDER BY next_run_at_ms ASC
LIMIT 1
            "#,
        )
        .bind(workspace_key)
        .bind(now_ms)
        .bind(now_ms)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(id) = id else {
            tx.commit().await?;
            return Ok(None);
        };

        let row = sqlx::query(
            r#"
UPDATE orchestrator_schedules
SET
    lease_owner = ?,
    lease_expires_at_ms = ?,
    updated_at_ms = ?
WHERE id = ?
  AND (lease_expires_at_ms IS NULL OR lease_expires_at_ms <= ?)
RETURNING
    id,
    workspace_key,
    enabled,
    cadence_seconds,
    next_run_at_ms,
    last_run_at_ms,
    lease_owner,
    lease_expires_at_ms,
    role_override,
    model_override,
    max_spawned_agents,
    max_run_seconds,
    base_token_budget,
    current_token_budget,
    min_token_budget,
    max_token_budget,
    consecutive_low_value_runs,
    consecutive_loop_detections,
    created_at_ms,
    updated_at_ms
            "#,
        )
        .bind(lease_owner)
        .bind(lease_expires_at_ms)
        .bind(now_ms)
        .bind(id)
        .bind(now_ms)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;

        row.map(|row| {
            OrchestratorScheduleRow::try_from_row(&row)
                .and_then(crate::OrchestratorSchedule::try_from)
        })
        .transpose()
    }

    pub async fn complete_orchestrator_schedule_run(
        &self,
        schedule_id: &str,
        lease_owner: &str,
        loop_detected: bool,
        low_value: bool,
        high_impact: bool,
    ) -> anyhow::Result<Option<crate::OrchestratorSchedule>> {
        let now_ms = datetime_to_epoch_millis(Utc::now());
        let row = sqlx::query(
            r#"
UPDATE orchestrator_schedules
SET
    enabled = CASE
        WHEN ? = 1 AND consecutive_loop_detections + 1 >= ? THEN 0
        ELSE enabled
    END,
    next_run_at_ms = ? + cadence_seconds * 1000,
    last_run_at_ms = ?,
    lease_owner = NULL,
    lease_expires_at_ms = NULL,
    current_token_budget = CASE
        WHEN ? = 1 THEN MAX(min_token_budget, CAST(current_token_budget * 0.7 AS INTEGER))
        WHEN ? = 1 THEN MAX(min_token_budget, CAST(current_token_budget * 0.85 AS INTEGER))
        WHEN ? = 1 THEN MIN(max_token_budget, CAST(current_token_budget * 1.15 AS INTEGER))
        ELSE current_token_budget
    END,
    consecutive_low_value_runs = CASE WHEN ? = 1 THEN consecutive_low_value_runs + 1 ELSE 0 END,
    consecutive_loop_detections = CASE WHEN ? = 1 THEN consecutive_loop_detections + 1 ELSE 0 END,
    updated_at_ms = ?
WHERE id = ?
  AND lease_owner = ?
RETURNING
    id,
    workspace_key,
    enabled,
    cadence_seconds,
    next_run_at_ms,
    last_run_at_ms,
    lease_owner,
    lease_expires_at_ms,
    role_override,
    model_override,
    max_spawned_agents,
    max_run_seconds,
    base_token_budget,
    current_token_budget,
    min_token_budget,
    max_token_budget,
    consecutive_low_value_runs,
    consecutive_loop_detections,
    created_at_ms,
    updated_at_ms
            "#,
        )
        .bind(i64::from(loop_detected))
        .bind(LOOP_PAUSE_THRESHOLD)
        .bind(now_ms)
        .bind(now_ms)
        .bind(i64::from(loop_detected))
        .bind(i64::from(low_value))
        .bind(i64::from(high_impact))
        .bind(i64::from(low_value))
        .bind(i64::from(loop_detected))
        .bind(now_ms)
        .bind(schedule_id)
        .bind(lease_owner)
        .fetch_optional(self.pool.as_ref())
        .await?;

        row.map(|row| {
            OrchestratorScheduleRow::try_from_row(&row)
                .and_then(crate::OrchestratorSchedule::try_from)
        })
        .transpose()
    }

    pub async fn create_orchestrator_run(
        &self,
        workspace_key: &str,
        schedule_id: Option<&str>,
    ) -> anyhow::Result<crate::OrchestratorRun> {
        let id = Uuid::new_v4().to_string();
        let now_ms = datetime_to_epoch_millis(Utc::now());
        let row = sqlx::query(
            r#"
INSERT INTO orchestrator_runs (
    id,
    schedule_id,
    workspace_key,
    phase,
    status,
    started_at_ms
) VALUES (?, ?, ?, ?, ?, ?)
RETURNING
    id,
    schedule_id,
    workspace_key,
    phase,
    status,
    scout_tokens,
    execution_tokens,
    subagent_tokens,
    budget_used,
    spawned_agent_count,
    impact_score,
    summary,
    decisions_json,
    loop_flags_json,
    verifier_outcome,
    goal_updates_json,
    started_at_ms,
    completed_at_ms
            "#,
        )
        .bind(id)
        .bind(schedule_id)
        .bind(workspace_key)
        .bind(crate::OrchestratorRunPhase::Scout.as_str())
        .bind(crate::OrchestratorRunStatus::Running.as_str())
        .bind(now_ms)
        .fetch_one(self.pool.as_ref())
        .await?;

        OrchestratorRunRow::try_from_row(&row).and_then(crate::OrchestratorRun::try_from)
    }

    pub async fn finish_orchestrator_run(
        &self,
        run_id: &str,
        completion: crate::OrchestratorRunCompletion,
    ) -> anyhow::Result<Option<crate::OrchestratorRun>> {
        let now_ms = datetime_to_epoch_millis(Utc::now());
        let decisions_json = serde_json::to_string(&completion.decisions)?;
        let loop_flags_json = serde_json::to_string(&completion.loop_flags)?;
        let goal_updates_json = serde_json::to_string(&completion.goal_updates)?;
        let row = sqlx::query(
            r#"
UPDATE orchestrator_runs
SET
    phase = ?,
    status = ?,
    scout_tokens = ?,
    execution_tokens = ?,
    subagent_tokens = ?,
    budget_used = ?,
    spawned_agent_count = ?,
    impact_score = ?,
    summary = ?,
    decisions_json = ?,
    loop_flags_json = ?,
    verifier_outcome = ?,
    goal_updates_json = ?,
    completed_at_ms = ?
WHERE id = ?
RETURNING
    id,
    schedule_id,
    workspace_key,
    phase,
    status,
    scout_tokens,
    execution_tokens,
    subagent_tokens,
    budget_used,
    spawned_agent_count,
    impact_score,
    summary,
    decisions_json,
    loop_flags_json,
    verifier_outcome,
    goal_updates_json,
    started_at_ms,
    completed_at_ms
            "#,
        )
        .bind(completion.phase.as_str())
        .bind(completion.status.as_str())
        .bind(completion.scout_tokens)
        .bind(completion.execution_tokens)
        .bind(completion.subagent_tokens)
        .bind(completion.budget_used)
        .bind(completion.spawned_agent_count)
        .bind(completion.impact_score)
        .bind(completion.summary)
        .bind(decisions_json)
        .bind(loop_flags_json)
        .bind(completion.verifier_outcome)
        .bind(goal_updates_json)
        .bind(now_ms)
        .bind(run_id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        row.map(|row| {
            OrchestratorRunRow::try_from_row(&row).and_then(crate::OrchestratorRun::try_from)
        })
        .transpose()
    }

    pub async fn list_orchestrator_runs(
        &self,
        workspace_key: &str,
        limit: i64,
    ) -> anyhow::Result<Vec<crate::OrchestratorRun>> {
        let rows = sqlx::query(
            r#"
SELECT
    id,
    schedule_id,
    workspace_key,
    phase,
    status,
    scout_tokens,
    execution_tokens,
    subagent_tokens,
    budget_used,
    spawned_agent_count,
    impact_score,
    summary,
    decisions_json,
    loop_flags_json,
    verifier_outcome,
    goal_updates_json,
    started_at_ms,
    completed_at_ms
FROM orchestrator_runs
WHERE workspace_key = ?
ORDER BY started_at_ms DESC
LIMIT ?
            "#,
        )
        .bind(workspace_key)
        .bind(limit.max(1))
        .fetch_all(self.pool.as_ref())
        .await?;

        rows.into_iter()
            .map(|row| {
                OrchestratorRunRow::try_from_row(&row).and_then(crate::OrchestratorRun::try_from)
            })
            .collect()
    }

    pub async fn upsert_orchestrator_opportunity(
        &self,
        params: &OrchestratorOpportunityUpsert,
    ) -> anyhow::Result<crate::OrchestratorOpportunity> {
        let now_ms = datetime_to_epoch_millis(Utc::now());
        let id = Uuid::new_v4().to_string();
        let row = sqlx::query(
            r#"
INSERT INTO orchestrator_opportunities (
    id,
    workspace_key,
    title,
    rationale,
    status,
    priority,
    impact_score,
    confidence,
    estimated_cost,
    risk,
    dedupe_key,
    created_by_run_id,
    completed_by_run_id,
    created_at_ms,
    updated_at_ms
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)
ON CONFLICT(workspace_key, dedupe_key) DO UPDATE SET
    title = excluded.title,
    rationale = excluded.rationale,
    status = CASE
        WHEN orchestrator_opportunities.status IN ('complete', 'rejected') THEN orchestrator_opportunities.status
        ELSE excluded.status
    END,
    priority = excluded.priority,
    impact_score = excluded.impact_score,
    confidence = excluded.confidence,
    estimated_cost = excluded.estimated_cost,
    risk = excluded.risk,
    updated_at_ms = excluded.updated_at_ms
RETURNING
    id,
    workspace_key,
    title,
    rationale,
    status,
    priority,
    impact_score,
    confidence,
    estimated_cost,
    risk,
    dedupe_key,
    created_by_run_id,
    completed_by_run_id,
    created_at_ms,
    updated_at_ms
            "#,
        )
        .bind(id)
        .bind(params.workspace_key.as_str())
        .bind(params.title.as_str())
        .bind(params.rationale.as_str())
        .bind(params.status.as_str())
        .bind(params.priority)
        .bind(params.impact_score)
        .bind(params.confidence)
        .bind(params.estimated_cost)
        .bind(params.risk.as_str())
        .bind(params.dedupe_key.as_str())
        .bind(params.created_by_run_id.as_deref())
        .bind(now_ms)
        .bind(now_ms)
        .fetch_one(self.pool.as_ref())
        .await?;

        OrchestratorOpportunityRow::try_from_row(&row)
            .and_then(crate::OrchestratorOpportunity::try_from)
    }

    pub async fn update_orchestrator_opportunity_status(
        &self,
        id: &str,
        status: crate::OrchestratorOpportunityStatus,
        completed_by_run_id: Option<&str>,
    ) -> anyhow::Result<Option<crate::OrchestratorOpportunity>> {
        let now_ms = datetime_to_epoch_millis(Utc::now());
        let row = sqlx::query(
            r#"
UPDATE orchestrator_opportunities
SET
    status = ?,
    completed_by_run_id = CASE WHEN ? = 'complete' THEN ? ELSE completed_by_run_id END,
    updated_at_ms = ?
WHERE id = ?
RETURNING
    id,
    workspace_key,
    title,
    rationale,
    status,
    priority,
    impact_score,
    confidence,
    estimated_cost,
    risk,
    dedupe_key,
    created_by_run_id,
    completed_by_run_id,
    created_at_ms,
    updated_at_ms
            "#,
        )
        .bind(status.as_str())
        .bind(status.as_str())
        .bind(completed_by_run_id)
        .bind(now_ms)
        .bind(id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        row.map(|row| {
            OrchestratorOpportunityRow::try_from_row(&row)
                .and_then(crate::OrchestratorOpportunity::try_from)
        })
        .transpose()
    }

    pub async fn list_orchestrator_opportunities(
        &self,
        workspace_key: &str,
    ) -> anyhow::Result<Vec<crate::OrchestratorOpportunity>> {
        let rows = sqlx::query(
            r#"
SELECT
    id,
    workspace_key,
    title,
    rationale,
    status,
    priority,
    impact_score,
    confidence,
    estimated_cost,
    risk,
    dedupe_key,
    created_by_run_id,
    completed_by_run_id,
    created_at_ms,
    updated_at_ms
FROM orchestrator_opportunities
WHERE workspace_key = ?
ORDER BY priority DESC, impact_score DESC, updated_at_ms DESC
            "#,
        )
        .bind(workspace_key)
        .fetch_all(self.pool.as_ref())
        .await?;

        rows.into_iter()
            .map(|row| {
                OrchestratorOpportunityRow::try_from_row(&row)
                    .and_then(crate::OrchestratorOpportunity::try_from)
            })
            .collect()
    }

    pub async fn record_orchestrator_work_ledger_entry(
        &self,
        workspace_key: &str,
        task_title: &str,
        outcome: crate::OrchestratorLedgerOutcome,
        touched_areas: &[String],
        blockers: &[String],
        dedupe_key: &str,
        run_id: Option<&str>,
    ) -> anyhow::Result<crate::OrchestratorWorkLedgerEntry> {
        let id = Uuid::new_v4().to_string();
        let now_ms = datetime_to_epoch_millis(Utc::now());
        let touched_areas_json = serde_json::to_string(touched_areas)?;
        let blockers_json = serde_json::to_string(blockers)?;
        let row = sqlx::query(
            r#"
INSERT INTO orchestrator_work_ledger (
    id,
    workspace_key,
    task_title,
    outcome,
    touched_areas_json,
    blockers_json,
    dedupe_key,
    run_id,
    created_at_ms,
    updated_at_ms
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING
    id,
    workspace_key,
    task_title,
    outcome,
    touched_areas_json,
    blockers_json,
    dedupe_key,
    run_id,
    created_at_ms,
    updated_at_ms
            "#,
        )
        .bind(id)
        .bind(workspace_key)
        .bind(task_title)
        .bind(outcome.as_str())
        .bind(touched_areas_json)
        .bind(blockers_json)
        .bind(dedupe_key)
        .bind(run_id)
        .bind(now_ms)
        .bind(now_ms)
        .fetch_one(self.pool.as_ref())
        .await?;

        OrchestratorWorkLedgerRow::try_from_row(&row)
            .and_then(crate::OrchestratorWorkLedgerEntry::try_from)
    }

    pub async fn list_orchestrator_work_ledger(
        &self,
        workspace_key: &str,
        limit: i64,
    ) -> anyhow::Result<Vec<crate::OrchestratorWorkLedgerEntry>> {
        let rows = sqlx::query(
            r#"
SELECT
    id,
    workspace_key,
    task_title,
    outcome,
    touched_areas_json,
    blockers_json,
    dedupe_key,
    run_id,
    created_at_ms,
    updated_at_ms
FROM orchestrator_work_ledger
WHERE workspace_key = ?
ORDER BY created_at_ms DESC
LIMIT ?
            "#,
        )
        .bind(workspace_key)
        .bind(limit.max(1))
        .fetch_all(self.pool.as_ref())
        .await?;

        rows.into_iter()
            .map(|row| {
                OrchestratorWorkLedgerRow::try_from_row(&row)
                    .and_then(crate::OrchestratorWorkLedgerEntry::try_from)
            })
            .collect()
    }
}

fn validate_schedule_params(params: &crate::OrchestratorScheduleUpsert) -> anyhow::Result<()> {
    if params.cadence_seconds <= 0 {
        anyhow::bail!("cadence_seconds must be positive");
    }
    if params.max_spawned_agents < 0 {
        anyhow::bail!("max_spawned_agents cannot be negative");
    }
    if params.max_run_seconds <= 0 {
        anyhow::bail!("max_run_seconds must be positive");
    }
    if params.min_token_budget <= 0
        || params.base_token_budget <= 0
        || params.max_token_budget < params.min_token_budget
    {
        anyhow::bail!("invalid token budget bounds");
    }
    Ok(())
}
