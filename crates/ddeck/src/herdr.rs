//! Jump to an agent's pane in herdr. Only ever called on a key press — never
//! from a hook or the polling loop.
//!
//! Facts (herdr, 31 October 2026): `HERDR_PANE_ID` in an agent's environment is
//! the same id `herdr agent list` reports as `pane_id` (`wE:p1`), and
//! `herdr agent focus <pane_id>` switches to it. Errors exit 1 with a JSON
//! `{"error":{"message":…}}` on stderr.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// Whether this process runs inside herdr (it sets `HERDR_PANE_ID` in every pane).
pub fn available() -> bool {
    std::env::var_os("HERDR_PANE_ID").is_some_and(|v| !v.is_empty())
}

/// One agent from `herdr agent list`.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    pub pane_id: String,
    pub cwd: String,
}

/// Agents from `herdr agent list` output; anything unexpected is skipped.
pub fn parse_agents(json: &str) -> Vec<Agent> {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    v["result"]["agents"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let cwd = a["foreground_cwd"].as_str().or(a["cwd"].as_str())?;
            Some(Agent {
                pane_id: a["pane_id"].as_str()?.to_owned(),
                cwd: cwd.to_owned(),
            })
        })
        .collect()
}

/// The agent working in `root`: same directory first, else one inside it.
pub fn pick_by_root<'a>(agents: &'a [Agent], root: &Path) -> Option<&'a Agent> {
    agents
        .iter()
        .find(|a| Path::new(&a.cwd) == root)
        .or_else(|| agents.iter().find(|a| Path::new(&a.cwd).starts_with(root)))
}

/// The `message` of a herdr error, else the text as is.
pub fn error_message(stderr: &str) -> String {
    serde_json::from_str::<Value>(stderr)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
        .unwrap_or_else(|| stderr.trim().to_owned())
}

/// Focus the agent: by its recorded pane, else by the project directory (an
/// agent started before the hooks, or a pane that has since moved).
pub fn focus(pane: Option<&str>, root: &Path) -> Result<(), String> {
    let mut last_err = None;
    if let Some(pane) = pane {
        match run(&["agent", "focus", pane]) {
            Ok(_) => return Ok(()),
            Err(e) => last_err = Some(e),
        }
    }
    let agents = parse_agents(&run(&["agent", "list"])?);
    match pick_by_root(&agents, root) {
        Some(a) if Some(a.pane_id.as_str()) != pane => {
            run(&["agent", "focus", &a.pane_id]).map(|_| ())
        }
        _ => Err(last_err.unwrap_or_else(|| "no herdr agent in this project".to_owned())),
    }
}

fn run(args: &[&str]) -> Result<String, String> {
    let out = Command::new("herdr")
        .args(args)
        .output()
        .map_err(|e| format!("herdr: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!(
            "herdr: {}",
            error_message(&String::from_utf8_lossy(&out.stderr))
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = r#"{"id":"cli:agent:list","result":{"agents":[
        {"agent":"claude","agent_status":"idle","cwd":"/Users/dev/Projects/api","foreground_cwd":"/Users/dev/Projects/api","pane_id":"w8:p1","tab_id":"w8:t1","workspace_id":"w8"},
        {"agent":"claude","agent_status":"working","cwd":"/Users/dev/Projects/web","foreground_cwd":"/Users/dev/Projects/web/app","pane_id":"wD:p2"},
        {"agent":"claude","cwd":"/Users/dev/Projects/broken"}
    ]},"type":"agent_list"}"#;

    #[test]
    fn parses_agent_list() {
        let agents = parse_agents(LIST);
        assert_eq!(
            agents,
            [
                Agent {
                    pane_id: "w8:p1".into(),
                    cwd: "/Users/dev/Projects/api".into()
                },
                Agent {
                    pane_id: "wD:p2".into(),
                    cwd: "/Users/dev/Projects/web/app".into()
                },
            ]
        );
        assert!(parse_agents("not json").is_empty());
        assert!(parse_agents(r#"{"error":{"message":"x"}}"#).is_empty());
    }

    #[test]
    fn picks_the_project_agent() {
        let agents = parse_agents(LIST);
        let pick = |root: &str| pick_by_root(&agents, Path::new(root)).map(|a| a.pane_id.as_str());
        assert_eq!(pick("/Users/dev/Projects/api"), Some("w8:p1"));
        assert_eq!(pick("/Users/dev/Projects/web"), Some("wD:p2"));
        assert_eq!(pick("/Users/dev/Projects/ap"), None);
        assert_eq!(pick("/Users/dev/Projects/docs"), None);
    }

    #[test]
    fn herdr_errors_read_well() {
        assert_eq!(
            error_message(
                r#"{"error":{"code":"agent_not_found","message":"agent target nope not found"},"id":"cli:agent:focus"}"#
            ),
            "agent target nope not found"
        );
        assert_eq!(error_message("boom\n"), "boom");
    }
}
