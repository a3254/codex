use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use serde_json::Value;
use uuid::Uuid;

pub(crate) const DEFAULT_DYNAMIC_DELAY: Duration = Duration::from_secs(10 * 60);
const MIN_INTERVAL: Duration = Duration::from_secs(60);
const MAX_PROMPT_BYTES: usize = 25_000;
pub(crate) const LOOP_COMPLETE_MARKER: &str = "[codex-loop-complete]";

const BUILT_IN_MAINTENANCE_PROMPT: &str = "\
Continue any unfinished work from this conversation. If there is a current branch pull request, \
check for review comments, failed CI runs, or merge conflicts and address them. If nothing is \
pending, do a small cleanup pass such as looking for obvious bugs or simplifications. Do not start \
unrelated new initiatives, and do not take irreversible actions unless the conversation already \
authorized them.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LoopScheduleKind {
    Fixed { interval: Duration },
    Dynamic,
}

#[derive(Clone, Debug)]
pub(crate) struct LoopJob {
    pub(crate) id: String,
    pub(crate) prompt: String,
    pub(crate) schedule_kind: LoopScheduleKind,
    pub(crate) expires_at: Instant,
    pub(crate) next_fire_at: Instant,
    pub(crate) generation: u64,
}

#[derive(Default)]
pub(crate) struct LoopScheduler {
    jobs: HashMap<String, LoopJob>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LoopCommand {
    Create {
        schedule_kind: LoopScheduleKind,
        prompt: Option<String>,
    },
    List,
    Cancel {
        id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NormalizedLoopRequest {
    Schedule {
        schedule_kind: LoopScheduleKind,
        prompt: String,
    },
    AlreadyDone {
        reason: Option<String>,
    },
}

impl LoopScheduler {
    pub(crate) fn create(
        &mut self,
        schedule_kind: LoopScheduleKind,
        prompt: String,
        now: Instant,
    ) -> LoopJob {
        let id = Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>();
        let delay = delay_for_schedule(&schedule_kind, &id);
        let job = LoopJob {
            id: id.clone(),
            prompt,
            schedule_kind,
            expires_at: now + Duration::from_secs(7 * 24 * 60 * 60),
            next_fire_at: now + delay,
            generation: 0,
        };
        self.jobs.insert(id, job.clone());
        job
    }

    pub(crate) fn list(&self) -> Vec<LoopJob> {
        let mut jobs = self.jobs.values().cloned().collect::<Vec<_>>();
        jobs.sort_by_key(|job| job.next_fire_at);
        jobs
    }

    pub(crate) fn cancel(&mut self, id: &str) -> Option<LoopJob> {
        self.jobs.remove(id)
    }

    pub(crate) fn get_due_prompt(
        &mut self,
        id: &str,
        generation: u64,
        now: Instant,
    ) -> Option<(String, Option<LoopJob>)> {
        let job = self.jobs.get_mut(id)?;
        if job.generation != generation {
            return None;
        }
        if now >= job.expires_at {
            self.jobs.remove(id);
            return None;
        }

        let prompt = loop_turn_prompt(&job.prompt);
        let delay = delay_for_schedule(&job.schedule_kind, &job.id);
        job.next_fire_at = now + delay;
        job.generation = job.generation.saturating_add(1);
        let rescheduled = job.clone();
        Some((prompt, Some(rescheduled)))
    }
}

pub(crate) fn parse_loop_command(args: &str) -> Result<LoopCommand, String> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Ok(LoopCommand::Create {
            schedule_kind: LoopScheduleKind::Dynamic,
            prompt: None,
        });
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or_default();
    let rest = parts.next().unwrap_or_default().trim();
    match first.to_ascii_lowercase().as_str() {
        "list" | "ls" => return Ok(LoopCommand::List),
        "cancel" | "delete" | "stop" if !rest.is_empty() => {
            return Ok(LoopCommand::Cancel {
                id: rest.to_string(),
            });
        }
        "cancel" | "delete" | "stop" => {
            return Err("Usage: /loop cancel <id>".to_string());
        }
        _ => {}
    }

    if let Some(interval) = parse_interval(first)? {
        return Ok(LoopCommand::Create {
            schedule_kind: LoopScheduleKind::Fixed { interval },
            prompt: non_empty_prompt(rest),
        });
    }

    Ok(LoopCommand::Create {
        schedule_kind: LoopScheduleKind::Dynamic,
        prompt: Some(trimmed.to_string()),
    })
}

pub(crate) fn loop_normalization_prompt(args: &str, default_prompt: &str) -> String {
    format!(
        r#"Prepare a Codex /loop scheduled task from this user request:

<loop_request>
{args}
</loop_request>

If the request omits the task prompt, use this default prompt:

<default_loop_prompt>
{default_prompt}
</default_loop_prompt>

First verify whether this exact requested task is already complete. Use available repository or shell tools if that is necessary to answer. Only return schedule:false when the request is unambiguously satisfied in the current conversation or current workspace state. Do not treat old plans, historical notes, completed TODOs, or similar prior work as completion unless the user explicitly asked to continue or monitor that exact artifact. When completion is ambiguous, schedule the loop.

Normalize the interval from the exact text inside <loop_request> into a fixed interval string using only s, m, h, or d. For example, flexible time wording may include phrases like "every N minutes", "hourly", "daily", "twice a day", "in N hours", "every N.Nh", or "every other hour". Do not copy interval values from these examples; the interval value must come only from <loop_request>. If the user did not specify a clear interval, use null for interval so Codex will use dynamic scheduling.

Return only a JSON object with this exact shape:
{{"schedule":true,"interval":null,"prompt":"cleaned recurring prompt","reason":null}}

If already complete, return:
{{"schedule":false,"interval":null,"prompt":null,"reason":"brief reason"}}

The cleaned prompt should preserve the user's requested outcome, tell the future loop turn to re-check only whether this exact scheduled task is already done before taking action, and tell it to report completion only when there is no remaining work or no next phase for this scheduled task."#
    )
}

fn loop_turn_prompt(prompt: &str) -> String {
    format!(
        "\
This is a scheduled /loop turn.

Re-check whether this exact scheduled task is already done, or whether there are no more phases to continue, before taking action. Do not treat old plans, historical notes, completed TODOs, or similar prior work as completion unless they are clearly the artifact or outcome requested by this scheduled task.

If this scheduled task's requested work is already done or there is no next phase for this scheduled task, do not take further action. Say that the loop is complete, and include this exact marker on its own line so Codex can cancel the scheduled loop:
{LOOP_COMPLETE_MARKER}

Otherwise, continue with the task.

Task:
{prompt}"
    )
}

pub(crate) fn loop_response_completed(response: &str) -> bool {
    response
        .lines()
        .any(|line| line.trim() == LOOP_COMPLETE_MARKER)
}

pub(crate) fn strip_loop_complete_marker(response: &str) -> String {
    response
        .lines()
        .filter(|line| line.trim() != LOOP_COMPLETE_MARKER)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub(crate) fn parse_normalized_loop_response(text: &str) -> Result<NormalizedLoopRequest, String> {
    let json = extract_json_object(text).ok_or_else(|| {
        "Loop setup response did not contain a JSON object. Please try /loop again.".to_string()
    })?;
    let value: Value = serde_json::from_str(json)
        .map_err(|err| format!("Failed to parse loop setup response: {err}"))?;
    let schedule = value
        .get("schedule")
        .and_then(Value::as_bool)
        .ok_or_else(|| "Loop setup response was missing boolean field `schedule`.".to_string())?;
    if !schedule {
        return Ok(NormalizedLoopRequest::AlreadyDone {
            reason: value
                .get("reason")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|reason| !reason.is_empty())
                .map(str::to_string),
        });
    }

    let prompt = value
        .get("prompt")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|prompt| !prompt.is_empty())
        .ok_or_else(|| "Loop setup response was missing non-empty field `prompt`.".to_string())?
        .to_string();
    let schedule_kind = match value.get("interval").and_then(Value::as_str).map(str::trim) {
        Some(interval) if !interval.is_empty() => {
            let Some(interval) = parse_interval(interval)? else {
                return Err(format!(
                    "Loop setup response used invalid interval `{interval}`."
                ));
            };
            LoopScheduleKind::Fixed { interval }
        }
        _ => LoopScheduleKind::Dynamic,
    };

    Ok(NormalizedLoopRequest::Schedule {
        schedule_kind,
        prompt,
    })
}

pub(crate) fn resolve_default_prompt(cwd: &Path, codex_home: &Path) -> String {
    let project_prompt = cwd.join(".codex").join("loop.md");
    read_prompt_file(project_prompt.as_path())
        .or_else(|| read_prompt_file(codex_home.join("loop.md").as_path()))
        .unwrap_or_else(|| BUILT_IN_MAINTENANCE_PROMPT.to_string())
}

pub(crate) fn cadence_label(schedule_kind: &LoopScheduleKind) -> String {
    match schedule_kind {
        LoopScheduleKind::Fixed { interval } => format!("Every {}", format_duration(*interval)),
        LoopScheduleKind::Dynamic => "Dynamic, about every 10 minutes".to_string(),
    }
}

pub(crate) fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs.is_multiple_of(86_400) {
        format!("{}d", secs / 86_400)
    } else if secs.is_multiple_of(3_600) {
        format!("{}h", secs / 3_600)
    } else if secs.is_multiple_of(60) {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

fn parse_interval(token: &str) -> Result<Option<Duration>, String> {
    let token = token.trim();
    if token.len() < 2 {
        return Ok(None);
    }
    let (number, unit) = token.split_at(token.len() - 1);
    let Some(unit) = unit.chars().next() else {
        return Ok(None);
    };
    if !matches!(unit, 's' | 'm' | 'h' | 'd') || !number.chars().all(|c| c.is_ascii_digit()) {
        return Ok(None);
    }
    let value = number
        .parse::<u64>()
        .map_err(|_| format!("Invalid /loop interval: {token}"))?;
    if value == 0 {
        return Err("Loop interval must be greater than zero.".to_string());
    }
    let seconds = match unit {
        's' => value.div_ceil(60) * 60,
        'm' => value * 60,
        'h' => value * 60 * 60,
        'd' => value * 24 * 60 * 60,
        _ => unreachable!("validated interval unit"),
    };
    let duration = Duration::from_secs(seconds);
    if duration < MIN_INTERVAL {
        return Err("Loop interval must be at least 1m.".to_string());
    }
    Ok(Some(duration))
}

fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (start <= end).then_some(&text[start..=end])
}

fn non_empty_prompt(prompt: &str) -> Option<String> {
    let trimmed = prompt.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn read_prompt_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let end = bytes.len().min(MAX_PROMPT_BYTES);
    Some(String::from_utf8_lossy(&bytes[..end]).trim().to_string())
        .filter(|prompt| !prompt.is_empty())
}

fn delay_for_schedule(schedule_kind: &LoopScheduleKind, id: &str) -> Duration {
    match schedule_kind {
        LoopScheduleKind::Fixed { interval } => *interval + jitter(*interval, id),
        LoopScheduleKind::Dynamic => DEFAULT_DYNAMIC_DELAY,
    }
}

fn jitter(interval: Duration, id: &str) -> Duration {
    let max = (interval.as_secs() / 10).min(15 * 60);
    if max == 0 {
        return Duration::ZERO;
    }
    let seed = id.bytes().fold(0_u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(u64::from(byte))
    });
    Duration::from_secs(seed % (max + 1))
}

#[cfg(all(test, any()))]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_fixed_interval_with_prompt() {
        assert_eq!(
            parse_loop_command("5m check CI").unwrap(),
            LoopCommand::Create {
                schedule_kind: LoopScheduleKind::Fixed {
                    interval: Duration::from_secs(300)
                },
                prompt: Some("check CI".to_string()),
            }
        );
    }

    #[test]
    fn parses_seconds_as_one_minute_granularity() {
        assert_eq!(
            parse_loop_command("30s check CI").unwrap(),
            LoopCommand::Create {
                schedule_kind: LoopScheduleKind::Fixed {
                    interval: Duration::from_secs(60)
                },
                prompt: Some("check CI".to_string()),
            }
        );
    }

    #[test]
    fn parses_prompt_only_as_dynamic() {
        assert_eq!(
            parse_loop_command("check deploy").unwrap(),
            LoopCommand::Create {
                schedule_kind: LoopScheduleKind::Dynamic,
                prompt: Some("check deploy".to_string()),
            }
        );
    }

    #[test]
    fn parses_management_commands() {
        assert_eq!(parse_loop_command("list").unwrap(), LoopCommand::List);
        assert_eq!(
            parse_loop_command("cancel abc12345").unwrap(),
            LoopCommand::Cancel {
                id: "abc12345".to_string()
            }
        );
    }

    #[test]
    fn normalization_prompt_keeps_examples_from_competing_with_user_interval() {
        let prompt = loop_normalization_prompt("1 hour say hello", "default work");

        assert!(prompt.contains("<loop_request>\n1 hour say hello\n</loop_request>"));
        assert!(!prompt.contains("2 hours"));
        assert!(!prompt.contains("2h"));
        assert!(prompt.contains("Do not copy interval values from these examples"));
    }

    #[test]
    fn normalization_prompt_requires_exact_task_completion() {
        let prompt = loop_normalization_prompt("find 5 app improvements", "default work");

        assert!(prompt.contains("Only return schedule:false"));
        assert!(prompt.contains("unambiguously satisfied"));
        assert!(prompt.contains("old plans, historical notes, completed TODOs"));
        assert!(prompt.contains("When completion is ambiguous, schedule the loop"));
    }

    #[test]
    fn parses_normalized_schedule_response() {
        assert_eq!(
            parse_normalized_loop_response(
                r#"```json
{"schedule":true,"interval":"2h","prompt":"Re-check CI before acting.","reason":null}
```"#,
            )
            .unwrap(),
            NormalizedLoopRequest::Schedule {
                schedule_kind: LoopScheduleKind::Fixed {
                    interval: Duration::from_secs(2 * 60 * 60)
                },
                prompt: "Re-check CI before acting.".to_string(),
            }
        );
    }

    #[test]
    fn parses_normalized_already_done_response() {
        assert_eq!(
            parse_normalized_loop_response(
                r#"{"schedule":false,"interval":null,"prompt":null,"reason":"CI already passed"}"#,
            )
            .unwrap(),
            NormalizedLoopRequest::AlreadyDone {
                reason: Some("CI already passed".to_string())
            }
        );
    }

    #[test]
    fn due_prompt_includes_completion_marker_instruction() {
        let mut scheduler = LoopScheduler::default();
        let now = Instant::now();
        let job = scheduler.create(
            LoopScheduleKind::Fixed {
                interval: Duration::from_secs(300),
            },
            "continue with the next phase".to_string(),
            now,
        );

        let (prompt, _) = scheduler
            .get_due_prompt(&job.id, job.generation, now)
            .expect("job should produce a due prompt");

        assert!(prompt.contains("continue with the next phase"));
        assert!(prompt.contains(LOOP_COMPLETE_MARKER));
        assert!(prompt.contains("no more phases"));
        assert!(prompt.contains("old plans, historical notes, completed TODOs"));
    }

    #[test]
    fn detects_and_strips_completion_marker() {
        let response = format!("Done.\n{LOOP_COMPLETE_MARKER}\n");

        assert!(loop_response_completed(&response));
        assert_eq!(strip_loop_complete_marker(&response), "Done.");
    }
}
