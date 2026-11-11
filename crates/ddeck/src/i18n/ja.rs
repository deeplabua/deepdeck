//! 日本語.

use super::Strings;
use crate::update::Notice;

pub const JA: Strings = Strings {
    app: "DeepDeck",
    tab_projects: "プロジェクト",
    tab_timeline: "タイムライン",
    tab_inbox: "受信箱",

    state_waiting: "あなたの返答待ち",
    state_working: "作業中",
    state_done: "完了",
    state_idle: "待機中",
    state_ended: "終了",

    plan: "計画",
    you: "あなた:",
    recap: "要約:",
    summary: "まとめ:",
    notes_questions: "質問",
    notes_ideas: "アイデア",
    notes_tasks: "タスク",
    notes_handoffs: "あなたの番",
    last_session: "前回のセッション:",
    quiet: "待機中",
    more_done: |n| format!("+ 完了 {n} 件"),
    more: |n| format!("+ ほか {n} 件"),
    more_sessions: |n| format!("+{n} セッション"),
    ago: |secs| match secs.max(0) {
        0..60 => "今".to_owned(),
        s @ 60..3600 => format!("{}分", s / 60),
        s @ 3600..86_400 => format!("{}時間", s / 3600),
        s => format!("{}日", s / 86_400),
    },

    kind_task: "タスク",
    kind_question: "質問",
    kind_idea: "アイデア",
    kind_handoff: "あなたの番",
    input_hint: "tab プロジェクト · ↵ 保存 · esc キャンセル",
    edit_label: "編集",
    copied: |how| format!("コピーしました · {how}"),
    sorted: |order| match order {
        deepdeck_core::Order::Name => "プロジェクトを名前順に — s: 注目順".to_owned(),
        deepdeck_core::Order::Attention => "プロジェクトを注目順に — s: 名前順".to_owned(),
    },
    notice: |n| match n {
        Notice::NewVersion(v, cmd) => format!("↑ DeepDeck {v} が出ました — {cmd}"),
        Notice::Setup { installed: Some(v) } => format!(
            "エージェントの指示は v{v}、この ddeck は v{} — ddeck setup を実行してください",
            crate::setup::BLOCK_VERSION
        ),
        Notice::Setup { installed: None } => {
            "エージェントに DeepDeck の指示がありません — ddeck setup を実行してください".to_owned()
        }
    },
    notice_copy: "  (u: コピー)",
    herdr_outside: "o は herdr のエージェントに移動します — herdr のペインで ddeck を起動してください",

    keys: &[
        ("o", "エージェント"),
        ("y", "コピー"),
        ("s", "並び順"),
        ("f", "絞り込み"),
        ("a", "タスク"),
        ("?", "質問"),
        ("i", "アイデア"),
        ("↵", "開く"),
        ("␣", "完了"),
        ("d", "削除"),
        ("e", "編集"),
        ("q", "終了"),
    ],
    keys_timeline: &[
        ("j/k", "スクロール"),
        ("f", "このプロジェクト / すべて"),
        ("q", "終了"),
    ],
    keys_inbox: &[
        ("o", "エージェント"),
        ("y", "コピー"),
        ("f", "絞り込み"),
        ("↵", "開く"),
        ("␣", "解決"),
        ("m", "→ タスク"),
        ("e", "編集"),
        ("?", "質問"),
        ("i", "アイデア"),
        ("q", "終了"),
    ],

    tl_plan: |n| format!("計画: {n} ステップ"),
    tl_roadmap: |n| format!("ロードマップ: {n} ステップ"),
    tl_cmd: |name| format!("コマンドを保存: {name}"),
    commands: "コマンド",
    roadmap: "ロードマップ",
    tl_you_were_in: |ago, project| format!("{ago}前は {project} にいました"),
    tl_empty: "今日はまだ何もありません。",
    tl_retracted: "宛先違いのメッセージを削除しました",
    tl_moved_to: |project| format!("メッセージを {project} に移動しました"),
    only: |name| format!("{name} のみ"),

    ib_waiting: "あなたの返答待ち",
    ib_questions: "未解決の質問",
    ib_ideas: "あとで検討するアイデア",
    ib_handoffs: "あなたの番",
    ib_empty: "受信箱は空です。待っているものはありません。",

    empty_no_hooks: &[
        "DeepDeck はまだ設定されていません。",
        "",
        "実行:  ddeck setup --dry-run   — 変更内容を確認",
        "       ddeck setup             — 変更を適用",
        "",
        "~/.claude/settings.json に 6 つのフックを、~/.claude/CLAUDE.md に",
        "短いブロックを追加し、エージェントが計画を投稿するようにします。",
        "先にバックアップを作成し、他のフックやテキストは変更しません。",
    ],
    empty_no_events: &["Claude Code のセッションを始めると、ここに表示されます。"],

    st_none: "まだアクティビティはありません。Claude Code のセッションを始めると、ここに表示されます。",
    st_projects: |n| format!("DeepDeck · {n} プロジェクト"),
    st_waiting: |n| format!("{n} 件が返答待ち"),
    st_notes_only: "メモのみ",
    st_sessions: |n| format!("({n} セッション)"),

    dr_binary: "バイナリ",
    dr_data: "データ",
    dr_claude: "claude",
    dr_claude_missing: "PATH に見つかりません",
    dr_hooks_in: |p| format!("フック: {p}"),
    dr_hook_missing: "ありません — `ddeck setup` を実行してください",
    dr_not_this_binary: "(このバイナリではありません)",
    dr_instructions: "指示",
    dr_instr_old: |p, v, cur| {
        format!("{p} は v{v}、最新は v{cur} — `ddeck setup` を実行してください")
    },
    dr_instr_none: |p| {
        format!(
            "{p} にありません — `ddeck setup` を実行してください(エージェントが計画を投稿しません)"
        )
    },
    dr_version: "バージョン",
    dr_version_new: |cur, v, cmd| format!("{cur} をインストール済み、{v} が出ています — {cmd}"),
    dr_version_ok: |cur| format!("{cur}(brew と cargo が知る最新版)"),
    dr_events: "イベント",
    dr_events_total: |p, n| format!("{p} プロジェクト、{n} イベント"),
    dr_last: |ago| format!("最終 {ago}前"),
    dr_hook_errors: |n, p| format!("フックのエラー({p} に {n} 件)、最新:"),
    dr_hook_errors_none: "フックのエラー: なし",

    su_dry_run: "(ドライラン — 何も書き込んでいません)",
    su_dry_remove: |p| format!("(ドライラン — {p} は削除されます)"),
    su_removed: |p| format!("{p} を削除しました"),
    su_wrote: |p| format!("{p} に書き込みました"),
    su_wrote_backup: |p, b| format!("{p} に書き込みました(バックアップ: {b})"),
    su_instr_current: |p| format!("{p} の DeepDeck の指示は最新です"),
    su_no_instr: |p| format!("{p} に DeepDeck の指示はありません"),
    su_hooks_present: |p| format!("DeepDeck のフックは {p} にインストール済みです"),
    su_no_hooks: |p| format!("{p} に DeepDeck のフックはありません"),
    su_restart: "変更を反映するには、実行中の Claude Code セッションを再起動してください(または /hooks を開く)。",
    su_done: "完了。新しい Claude Code セッションは計画をボードに投稿します。`ddeck` で開いてください。",
    su_target_warning: |p| {
        format!(
            "警告: フックが target/ 内のビルド({p})を指しています。`cargo install --path crates/ddeck` を実行して、安定したパスでやり直してください"
        )
    },

    lang_now: |name, code| format!("言語: {name} ({code})"),
    lang_set: |name| format!("言語: {name}"),
    lang_auto: "言語: システムの設定に戻しました",
};
