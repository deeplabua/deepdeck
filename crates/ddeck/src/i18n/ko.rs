//! 한국어.

use super::Strings;
use crate::update::Notice;

pub const KO: Strings = Strings {
    app: "DeepDeck",
    tab_projects: "프로젝트",
    tab_timeline: "타임라인",
    tab_inbox: "받은함",

    state_waiting: "응답 대기 중",
    state_working: "작업 중",
    state_done: "완료",
    state_idle: "유휴",
    state_ended: "종료됨",

    plan: "계획",
    you: "나:",
    recap: "요약:",
    summary: "정리:",
    notes_questions: "질문",
    notes_ideas: "아이디어",
    notes_tasks: "할 일",
    notes_handoffs: "내 차례",
    last_session: "지난 세션:",
    quiet: "유휴",
    more_done: |n| format!("+ 완료 {n}개"),
    more: |n| format!("+ {n}개 더"),
    more_sessions: |n| format!("+세션 {n}개"),
    ago: |secs| match secs.max(0) {
        0..60 => "방금".to_owned(),
        s @ 60..3600 => format!("{}분", s / 60),
        s @ 3600..86_400 => format!("{}시간", s / 3600),
        s => format!("{}일", s / 86_400),
    },

    kind_task: "할 일",
    kind_question: "질문",
    kind_idea: "아이디어",
    kind_handoff: "내 차례",
    input_hint: "tab 프로젝트 · ↵ 저장 · esc 취소",
    edit_label: "편집",
    copied: |how| format!("복사됨 · {how}"),
    sorted: |order| match order {
        deepdeck_core::Order::Name => "프로젝트 이름순 — s: 주목순".to_owned(),
        deepdeck_core::Order::Attention => "프로젝트 주목순 — s: 이름순".to_owned(),
    },
    notice: |n| match n {
        Notice::NewVersion(v, cmd) => format!("↑ DeepDeck {v} 출시 — {cmd}"),
        Notice::Setup { installed: Some(v) } => format!(
            "에이전트 지침은 v{v}, 이 ddeck은 v{} — ddeck setup을 실행하세요",
            crate::setup::BLOCK_VERSION
        ),
        Notice::Setup { installed: None } => {
            "에이전트에 DeepDeck 지침이 없습니다 — ddeck setup을 실행하세요".to_owned()
        }
    },
    notice_copy: "  (u: 복사)",
    herdr_outside: "o는 herdr의 에이전트로 이동합니다 — herdr 패널에서 ddeck을 실행하세요",

    keys: &[
        ("o", "에이전트"),
        ("y", "복사"),
        ("s", "정렬"),
        ("f", "필터"),
        ("a", "할 일"),
        ("?", "질문"),
        ("i", "아이디어"),
        ("↵", "열기"),
        ("␣", "완료"),
        ("d", "삭제"),
        ("e", "편집"),
        ("q", "종료"),
    ],
    keys_timeline: &[
        ("j/k", "스크롤"),
        ("f", "이 프로젝트 / 전체"),
        ("q", "종료"),
    ],
    keys_inbox: &[
        ("o", "에이전트"),
        ("y", "복사"),
        ("f", "필터"),
        ("↵", "열기"),
        ("␣", "해결"),
        ("m", "→ 할 일"),
        ("e", "편집"),
        ("?", "질문"),
        ("i", "아이디어"),
        ("q", "종료"),
    ],

    tl_plan: |n| format!("계획: {n}단계"),
    tl_roadmap: |n| format!("로드맵: {n}단계"),
    tl_cmd: |name| format!("명령 저장: {name}"),
    commands: "명령",
    roadmap: "로드맵",
    tl_you_were_in: |ago, project| format!("{ago} 전에는 {project}에 있었습니다"),
    tl_empty: "오늘은 아직 아무것도 없습니다.",
    tl_retracted: "잘못 보낸 메시지를 삭제했습니다",
    tl_moved_to: |project| format!("메시지를 {project}(으)로 옮겼습니다"),
    only: |name| format!("{name}만"),

    ib_waiting: "응답 대기 중",
    ib_questions: "열린 질문",
    ib_ideas: "나중에 볼 아이디어",
    ib_handoffs: "내 차례",
    ib_empty: "받은함이 비었습니다. 기다리는 것이 없습니다.",

    empty_no_hooks: &[
        "DeepDeck이 아직 설정되지 않았습니다.",
        "",
        "실행:  ddeck setup --dry-run   — 변경 사항 보기",
        "       ddeck setup             — 적용하기",
        "",
        "~/.claude/settings.json에 훅 6개, ~/.claude/CLAUDE.md에 짧은",
        "블록을 추가해 에이전트가 계획을 게시하게 합니다.",
        "먼저 백업하며, 다른 훅과 텍스트는 그대로 둡니다.",
    ],
    empty_no_events: &["Claude Code 세션을 시작하면 여기에 표시됩니다."],

    st_none: "아직 활동이 없습니다. Claude Code 세션을 시작하면 여기에 표시됩니다.",
    st_projects: |n| format!("DeepDeck · 프로젝트 {n}개"),
    st_waiting: |n| format!("{n}개 응답 대기 중"),
    st_notes_only: "메모만",
    st_sessions: |n| format!("(세션 {n}개)"),

    dr_binary: "바이너리",
    dr_data: "데이터",
    dr_claude: "claude",
    dr_claude_missing: "PATH에서 찾을 수 없음",
    dr_hooks_in: |p| format!("훅: {p}"),
    dr_hook_missing: "없음 — `ddeck setup`을 실행하세요",
    dr_not_this_binary: "(이 바이너리가 아님)",
    dr_instructions: "지침",
    dr_instr_old: |p, v, cur| format!("{p}: v{v}, 최신은 v{cur} — `ddeck setup`을 실행하세요"),
    dr_instr_none: |p| {
        format!("{p}에 없음 — `ddeck setup`을 실행하세요 (에이전트가 계획을 게시하지 않습니다)")
    },
    dr_version: "버전",
    dr_version_new: |cur, v, cmd| format!("{cur} 설치됨, {v} 출시 — {cmd}"),
    dr_version_ok: |cur| format!("{cur} (brew와 cargo가 아는 최신 버전)"),
    dr_events: "이벤트",
    dr_events_total: |p, n| format!("프로젝트 {p}개, 이벤트 {n}개"),
    dr_last: |ago| format!("마지막 {ago} 전"),
    dr_hook_errors: |n, p| format!("훅 오류 ({p}에 {n}개), 최근:"),
    dr_hook_errors_none: "훅 오류: 없음",

    su_dry_run: "(시험 실행 — 아무것도 쓰지 않음)",
    su_dry_remove: |p| format!("(시험 실행 — {p} 삭제 예정)"),
    su_removed: |p| format!("{p} 삭제됨"),
    su_wrote: |p| format!("{p} 저장됨"),
    su_wrote_backup: |p, b| format!("{p} 저장됨 (백업: {b})"),
    su_instr_current: |p| format!("{p}의 DeepDeck 지침이 최신입니다"),
    su_no_instr: |p| format!("{p}에 DeepDeck 지침이 없습니다"),
    su_hooks_present: |p| format!("DeepDeck 훅이 이미 {p}에 설치되어 있습니다"),
    su_no_hooks: |p| format!("{p}에 DeepDeck 훅이 없습니다"),
    su_restart: "변경 사항을 적용하려면 실행 중인 Claude Code 세션을 다시 시작하세요(또는 /hooks 열기).",
    su_done: "완료. 새 Claude Code 세션이 계획을 보드에 게시합니다. `ddeck`으로 여세요.",
    su_target_warning: |p| {
        format!(
            "경고: 훅이 target/ 안의 빌드({p})를 가리킵니다. 고정 경로를 위해 `cargo install --path crates/ddeck` 후 다시 실행하세요"
        )
    },

    lang_now: |name, code| format!("언어: {name} ({code})"),
    lang_set: |name| format!("언어: {name}"),
    lang_auto: "언어: 다시 시스템 설정을 따릅니다",
};
