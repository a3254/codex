CREATE TABLE workspace_goals (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_key TEXT NOT NULL,
    objective TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('human', 'orchestrator')),
    status TEXT NOT NULL CHECK(status IN ('active', 'paused', 'complete', 'archived')),
    priority INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    completed_at_ms INTEGER
);

CREATE INDEX workspace_goals_workspace_status_priority_idx
    ON workspace_goals(workspace_key, status, priority DESC, updated_at_ms DESC);

CREATE TABLE orchestrator_schedules (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_key TEXT NOT NULL,
    enabled INTEGER NOT NULL CHECK(enabled IN (0, 1)),
    cadence_seconds INTEGER NOT NULL,
    next_run_at_ms INTEGER NOT NULL,
    last_run_at_ms INTEGER,
    lease_owner TEXT,
    lease_expires_at_ms INTEGER,
    role_override TEXT,
    model_override TEXT,
    max_spawned_agents INTEGER NOT NULL,
    max_run_seconds INTEGER NOT NULL,
    base_token_budget INTEGER NOT NULL,
    current_token_budget INTEGER NOT NULL,
    min_token_budget INTEGER NOT NULL,
    max_token_budget INTEGER NOT NULL,
    consecutive_low_value_runs INTEGER NOT NULL DEFAULT 0,
    consecutive_loop_detections INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX orchestrator_schedules_due_idx
    ON orchestrator_schedules(enabled, next_run_at_ms, lease_expires_at_ms);

CREATE TABLE orchestrator_runs (
    id TEXT PRIMARY KEY NOT NULL,
    schedule_id TEXT REFERENCES orchestrator_schedules(id) ON DELETE SET NULL,
    workspace_key TEXT NOT NULL,
    phase TEXT NOT NULL CHECK(phase IN ('scout', 'execution', 'verification', 'complete')),
    status TEXT NOT NULL CHECK(status IN ('running', 'complete', 'failed', 'cancelled', 'loop_detected')),
    scout_tokens INTEGER NOT NULL DEFAULT 0,
    execution_tokens INTEGER NOT NULL DEFAULT 0,
    subagent_tokens INTEGER NOT NULL DEFAULT 0,
    budget_used INTEGER NOT NULL DEFAULT 0,
    spawned_agent_count INTEGER NOT NULL DEFAULT 0,
    impact_score REAL,
    summary TEXT,
    decisions_json TEXT NOT NULL DEFAULT '[]',
    loop_flags_json TEXT NOT NULL DEFAULT '[]',
    verifier_outcome TEXT CHECK(verifier_outcome IN ('passed', 'failed', 'needs_human', 'skipped_trivial')),
    goal_updates_json TEXT NOT NULL DEFAULT '[]',
    started_at_ms INTEGER NOT NULL,
    completed_at_ms INTEGER
);

CREATE INDEX orchestrator_runs_workspace_started_idx
    ON orchestrator_runs(workspace_key, started_at_ms DESC);

CREATE TABLE orchestrator_run_agents (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES orchestrator_runs(id) ON DELETE CASCADE,
    agent_thread_id TEXT NOT NULL,
    role TEXT NOT NULL,
    instruction TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('spawned', 'running', 'complete', 'failed', 'cancelled')),
    final_summary TEXT,
    verification_status TEXT CHECK(verification_status IN ('passed', 'failed', 'needs_human', 'skipped_trivial')),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX orchestrator_run_agents_run_idx
    ON orchestrator_run_agents(run_id);

CREATE TABLE orchestrator_opportunities (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_key TEXT NOT NULL,
    title TEXT NOT NULL,
    rationale TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('proposed', 'accepted', 'in_progress', 'complete', 'rejected', 'stale')),
    priority INTEGER NOT NULL DEFAULT 0,
    impact_score REAL NOT NULL,
    confidence REAL NOT NULL,
    estimated_cost INTEGER NOT NULL,
    risk TEXT NOT NULL CHECK(risk IN ('low', 'medium', 'high')),
    dedupe_key TEXT NOT NULL,
    created_by_run_id TEXT REFERENCES orchestrator_runs(id) ON DELETE SET NULL,
    completed_by_run_id TEXT REFERENCES orchestrator_runs(id) ON DELETE SET NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE(workspace_key, dedupe_key)
);

CREATE INDEX orchestrator_opportunities_workspace_status_idx
    ON orchestrator_opportunities(workspace_key, status, priority DESC, impact_score DESC);

CREATE TABLE orchestrator_work_ledger (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_key TEXT NOT NULL,
    task_title TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK(outcome IN ('attempted', 'completed', 'rejected', 'blocked')),
    touched_areas_json TEXT NOT NULL DEFAULT '[]',
    blockers_json TEXT NOT NULL DEFAULT '[]',
    dedupe_key TEXT NOT NULL,
    run_id TEXT REFERENCES orchestrator_runs(id) ON DELETE SET NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX orchestrator_work_ledger_workspace_dedupe_idx
    ON orchestrator_work_ledger(workspace_key, dedupe_key, created_at_ms DESC);
