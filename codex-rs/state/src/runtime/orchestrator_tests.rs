use pretty_assertions::assert_eq;

use crate::OrchestratorLedgerOutcome;
use crate::OrchestratorOpportunityStatus;
use crate::OrchestratorOpportunityUpsert;
use crate::OrchestratorRisk;
use crate::OrchestratorScheduleUpsert;
use crate::StateRuntime;
use crate::WorkspaceGoalSource;
use crate::WorkspaceGoalStatus;
use crate::WorkspaceGoalUpdate;
use crate::runtime::test_support::unique_temp_dir;

async fn test_runtime() -> anyhow::Result<std::sync::Arc<StateRuntime>> {
    StateRuntime::init(unique_temp_dir(), "test-provider".to_string()).await
}

fn schedule(workspace_key: &str) -> OrchestratorScheduleUpsert {
    OrchestratorScheduleUpsert {
        workspace_key: workspace_key.to_string(),
        cadence_seconds: 60,
        enabled: true,
        role_override: None,
        model_override: None,
        max_spawned_agents: 2,
        max_run_seconds: 300,
        base_token_budget: 10_000,
        min_token_budget: 2_000,
        max_token_budget: 20_000,
    }
}

fn opportunity(workspace_key: &str, dedupe_key: &str) -> OrchestratorOpportunityUpsert {
    OrchestratorOpportunityUpsert {
        workspace_key: workspace_key.to_string(),
        title: "Add focused tests".to_string(),
        rationale: "Improves confidence in orchestrator state".to_string(),
        status: OrchestratorOpportunityStatus::Proposed,
        priority: 10,
        impact_score: 0.8,
        confidence: 0.9,
        estimated_cost: 1_500,
        risk: OrchestratorRisk::Low,
        dedupe_key: dedupe_key.to_string(),
        created_by_run_id: None,
    }
}

#[tokio::test]
async fn orchestrator_workspace_goals_are_workspace_scoped() -> anyhow::Result<()> {
    let runtime = test_runtime().await?;
    let first = runtime
        .create_workspace_goal(
            "workspace-a",
            "Improve tests",
            WorkspaceGoalSource::Human,
            5,
        )
        .await?;
    runtime
        .create_workspace_goal(
            "workspace-b",
            "Improve docs",
            WorkspaceGoalSource::Orchestrator,
            1,
        )
        .await?;

    let goals = runtime
        .list_workspace_goals("workspace-a", &[WorkspaceGoalStatus::Active])
        .await?;

    assert_eq!(vec![first], goals);
    assert_eq!(WorkspaceGoalSource::Human, goals[0].source);
    Ok(())
}

#[tokio::test]
async fn orchestrator_workspace_goal_status_preserves_objective() -> anyhow::Result<()> {
    let runtime = test_runtime().await?;
    let goal = runtime
        .create_workspace_goal(
            "workspace",
            "Ship orchestrator",
            WorkspaceGoalSource::Human,
            0,
        )
        .await?;

    let completed = runtime
        .update_workspace_goal(
            &goal.id,
            WorkspaceGoalUpdate {
                status: Some(WorkspaceGoalStatus::Complete),
                priority: Some(2),
            },
        )
        .await?
        .expect("goal should exist");

    assert_eq!("Ship orchestrator", completed.objective);
    assert_eq!(WorkspaceGoalStatus::Complete, completed.status);
    assert_eq!(Some(completed.updated_at), completed.completed_at);
    assert_eq!(2, completed.priority);
    Ok(())
}

#[tokio::test]
async fn orchestrator_schedule_lease_prevents_double_execution() -> anyhow::Result<()> {
    let runtime = test_runtime().await?;
    let schedule = runtime
        .upsert_orchestrator_schedule(&schedule("workspace"))
        .await?;

    let claimed = runtime
        .acquire_due_orchestrator_schedule("workspace", "worker-a", 30_000)
        .await?
        .expect("schedule should be due");
    let second = runtime
        .acquire_due_orchestrator_schedule("workspace", "worker-b", 30_000)
        .await?;

    assert_eq!(schedule.id, claimed.id);
    assert_eq!(Some("worker-a".to_string()), claimed.lease_owner);
    assert_eq!(None, second);
    Ok(())
}

#[tokio::test]
async fn orchestrator_missed_intervals_coalesce_after_completion() -> anyhow::Result<()> {
    let runtime = test_runtime().await?;
    let schedule = runtime
        .upsert_orchestrator_schedule(&schedule("workspace"))
        .await?;
    runtime
        .acquire_due_orchestrator_schedule("workspace", "worker", 30_000)
        .await?;

    let completed = runtime
        .complete_orchestrator_schedule_run(
            &schedule.id,
            "worker",
            /*loop_detected*/ false,
            /*low_value*/ false,
            /*high_impact*/ false,
        )
        .await?
        .expect("lease owner should complete schedule");

    assert_eq!(None, completed.lease_owner);
    assert!(completed.next_run_at > completed.last_run_at.expect("last run"));
    let not_due = runtime
        .acquire_due_orchestrator_schedule("workspace", "worker", 30_000)
        .await?;
    assert_eq!(None, not_due);
    Ok(())
}

#[tokio::test]
async fn orchestrator_opportunities_dedupe_by_key() -> anyhow::Result<()> {
    let runtime = test_runtime().await?;
    let first = runtime
        .upsert_orchestrator_opportunity(&opportunity("workspace", "tests"))
        .await?;
    let mut second_params = opportunity("workspace", "tests");
    second_params.title = "Add better tests".to_string();
    second_params.impact_score = 0.95;

    let second = runtime
        .upsert_orchestrator_opportunity(&second_params)
        .await?;
    let opportunities = runtime.list_orchestrator_opportunities("workspace").await?;

    assert_eq!(first.id, second.id);
    assert_eq!(1, opportunities.len());
    assert_eq!("Add better tests", opportunities[0].title);
    assert_eq!(0.95, opportunities[0].impact_score);
    Ok(())
}

#[tokio::test]
async fn orchestrator_work_ledger_records_outcomes() -> anyhow::Result<()> {
    let runtime = test_runtime().await?;
    let touched = vec!["state".to_string(), "app-server".to_string()];
    let blockers = vec!["needs review".to_string()];

    let entry = runtime
        .record_orchestrator_work_ledger_entry(
            "workspace",
            "Wire APIs",
            OrchestratorLedgerOutcome::Blocked,
            &touched,
            &blockers,
            "api-wire",
            None,
        )
        .await?;
    let entries = runtime
        .list_orchestrator_work_ledger("workspace", /*limit*/ 10)
        .await?;

    assert_eq!(vec![entry], entries);
    assert_eq!(OrchestratorLedgerOutcome::Blocked, entries[0].outcome);
    assert_eq!(touched, entries[0].touched_areas);
    assert_eq!(blockers, entries[0].blockers);
    Ok(())
}

#[tokio::test]
async fn orchestrator_adaptive_budget_stays_within_bounds() -> anyhow::Result<()> {
    let runtime = test_runtime().await?;
    let schedule = runtime
        .upsert_orchestrator_schedule(&schedule("workspace"))
        .await?;

    let mut latest = schedule;
    for index in 0..10 {
        runtime
            .acquire_due_orchestrator_schedule("workspace", &format!("worker-{index}"), 30_000)
            .await?;
        latest = runtime
            .complete_orchestrator_schedule_run(
                &latest.id,
                &format!("worker-{index}"),
                /*loop_detected*/ false,
                /*low_value*/ true,
                /*high_impact*/ false,
            )
            .await?
            .expect("leased schedule should complete");
        sqlx::query("UPDATE orchestrator_schedules SET next_run_at_ms = 0 WHERE id = ?")
            .bind(latest.id.as_str())
            .execute(latest_pool(&runtime))
            .await?;
    }

    assert_eq!(2_000, latest.current_token_budget);
    Ok(())
}

fn latest_pool(runtime: &StateRuntime) -> &sqlx::SqlitePool {
    runtime.pool.as_ref()
}
