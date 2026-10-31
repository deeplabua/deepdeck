//! Hook payloads from real Claude Code 2.1.316 sessions (anonymised) → events.

use std::path::{Path, PathBuf};

use deepdeck_core::event::PlanItem;
use deepdeck_core::{AgentState, EventBody, ProjectMeta, TaskStatus, fold_project, parse_hook};
use jiff::Timestamp;

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn now() -> Timestamp {
    "2026-10-31T07:00:00Z".parse().unwrap()
}

fn body(name: &str) -> EventBody {
    let hook = parse_hook(&fixture(name), now())
        .unwrap()
        .expect("event expected");
    assert_eq!(hook.cwd, PathBuf::from("/Users/dev/Projects/demo"));
    assert_eq!(hook.session, "0b1e5c6a-7d2f-4c1e-9a55-3f0d8e2b7c41");
    assert_eq!(hook.event.session.as_deref(), Some(hook.session.as_str()));
    hook.event.body
}

#[test]
fn session_start() {
    assert_eq!(
        body("session_start.json"),
        EventBody::AgentStarted {
            source: "startup".into(),
            herdr_pane: None
        }
    );
}

#[test]
fn user_prompt_is_truncated_to_200_chars() {
    let EventBody::UserPrompt { text } = body("user_prompt_submit.json") else {
        panic!()
    };
    assert_eq!(text.chars().count(), 200);
    assert!(text.starts_with("а вложения тоже переносим?"));
    assert!(text.ends_with('…'));
}

#[test]
fn permission_and_idle_notifications_mean_waiting() {
    assert_eq!(
        body("notification_permission.json"),
        EventBody::AgentWaiting {
            text: "Claude needs your permission".into(),
            reason: Some("permission_prompt".into())
        }
    );
    assert!(matches!(
        body("notification_idle.json"),
        EventBody::AgentWaiting { reason: Some(r), .. } if r == "idle_prompt"
    ));
}

#[test]
fn stop_recap_comes_from_last_assistant_message() {
    assert_eq!(
        body("stop.json"),
        EventBody::AgentStopped {
            recap: Some(
                "Импорт вложений готов: адаптер для старого формата добавлен, старый путь не тронут. \
                 Прогресс теперь считается по байтам."
                    .into()
            )
        }
    );
}

#[test]
fn session_end() {
    assert_eq!(
        body("session_end.json"),
        EventBody::AgentEnded {
            reason: Some("other".into())
        }
    );
}

#[test]
fn task_create_and_update() {
    assert_eq!(
        body("post_tool_use_task_create.json"),
        EventBody::TaskUpsert {
            task_id: "4".into(),
            subject: Some("UI прогресса импорта".into()),
            status: Some(TaskStatus::Pending)
        }
    );
    assert_eq!(
        body("post_tool_use_task_update.json"),
        EventBody::TaskUpsert {
            task_id: "4".into(),
            subject: None,
            status: Some(TaskStatus::InProgress)
        }
    );
}

#[test]
fn todo_write_is_a_snapshot() {
    let EventBody::PlanSnapshot { tasks } = body("post_tool_use_todo_write.json") else {
        panic!()
    };
    assert_eq!(
        tasks,
        vec![
            PlanItem {
                subject: "Write hello.txt".into(),
                status: TaskStatus::Completed
            },
            PlanItem {
                subject: "Write bye.txt".into(),
                status: TaskStatus::InProgress
            },
            PlanItem {
                subject: "Summarize".into(),
                status: TaskStatus::Pending
            },
        ]
    );
}

#[test]
fn other_tools_are_ignored_and_never_logged() {
    assert!(
        parse_hook(&fixture("post_tool_use_bash.json"), now())
            .unwrap()
            .is_none()
    );
}

#[test]
fn whole_session_folds_into_a_board() {
    let order = [
        "session_start.json",
        "user_prompt_submit.json",
        "post_tool_use_task_create.json",
        "notification_permission.json",
        "post_tool_use_task_update.json",
        "stop.json",
    ];
    let events: Vec<_> = order
        .iter()
        .map(|f| parse_hook(&fixture(f), now()).unwrap().unwrap().event)
        .collect();
    let p = fold_project(
        ProjectMeta::for_root(Path::new("/Users/dev/Projects/demo")),
        &events,
    );
    let a = &p.agents[0];
    assert!(matches!(a.state, AgentState::Stopped { .. }));
    assert_eq!(a.progress(), (0, 1));
    assert_eq!(a.current_task().unwrap().subject, "UI прогресса импорта");
}
