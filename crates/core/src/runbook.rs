//! Saved project commands. The log is plain text in `~/.deepdeck/`, so a
//! command must refer to secrets (`dv run --item …`, `$VAR`), never hold them.

/// Why `command` looks like it contains a secret value, if it does.
pub fn secret_in(command: &str) -> Option<String> {
    const PREFIXES: [&str; 10] = [
        "ghp_",
        "gho_",
        "ghs_",
        "ghu_",
        "github_pat_",
        "glpat-",
        "sk-",
        "sk_live_",
        "xoxb-",
        "xoxp-",
    ];
    if command.contains("-----BEGIN") {
        return Some("a private key block".to_owned());
    }
    for word in command.split(|c: char| c.is_whitespace() || "\"'`(),;|&<>".contains(c)) {
        // `NAME=value`: a literal value assigned to a secret-sounding name.
        if let Some((name, value)) = word.split_once('=') {
            let upper = name.to_ascii_uppercase();
            let secret_name = ["TOKEN", "SECRET", "PASSWORD", "PASSWD", "API_KEY", "APIKEY"]
                .iter()
                .any(|s| upper.ends_with(s));
            if secret_name && !value.is_empty() && !value.starts_with('$') {
                return Some(format!("a literal value for {name}"));
            }
        }
        let value = word.rsplit('=').next().unwrap_or(word);
        if let Some(p) = PREFIXES
            .iter()
            .find(|p| value.starts_with(**p) && value.len() > p.len() + 8)
        {
            return Some(format!("a token starting with {p}"));
        }
        if is_aws_key(value) {
            return Some("an AWS access key".to_owned());
        }
        if is_random_looking(value) {
            return Some(format!("a {}-character token", value.len()));
        }
    }
    None
}

fn is_aws_key(w: &str) -> bool {
    w.len() == 20
        && (w.starts_with("AKIA") || w.starts_with("ASIA"))
        && w.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Long run of mixed-case letters and digits (crates.io, generic API keys).
/// Hex hashes (one case) and paths (`/`) do not count.
fn is_random_looking(w: &str) -> bool {
    w.len() >= 32
        && w.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_+=".contains(c))
        && w.chars().any(|c| c.is_ascii_uppercase())
        && w.chars().any(|c| c.is_ascii_lowercase())
        && w.chars().any(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_are_fine() {
        for ok in [
            "dv run --item \"DeepDeck release\" --sh 'printf %s \"$CARGO_REGISTRY_TOKEN\" | gh secret set CARGO_REGISTRY_TOKEN -R deeplabua/deepdeck'",
            "scripts/sync-public.sh main \"Release 0.3.0\"",
            "git checkout 3bb52911a6c0f1e2d4b5a6c7d8e9f0a1b2c3d4e5",
            "TOKEN=$(dv get Release TOKEN) make publish",
            "cargo publish -p deepdeck",
            "curl -H \"Authorization: Bearer $API_TOKEN\" https://example.com",
        ] {
            assert_eq!(secret_in(ok), None, "{ok}");
        }
    }

    #[test]
    fn values_are_refused() {
        // Fake tokens, split so secret scanners do not take the source for a leak.
        for bad in [
            concat!(
                "gh secret set X --body gh",
                "p_abcdefghijklmnopqrstuvwxyz0123456789"
            ),
            concat!(
                "CARGO_REGISTRY_TOKEN=ci",
                "oAbCdEf0123456789xyzXYZ0123456789 cargo publish"
            ),
            concat!("cargo login ci", "oAbCdEf0123456789xyzXYZ0123456789ab"),
            concat!(
                "aws configure set aws_access_key_id AK",
                "IAIOSFODNN7EXAMPLE"
            ),
            "export API_KEY=hunter2",
            "echo '-----BEGIN OPENSSH PRIVATE KEY-----'",
        ] {
            assert!(secret_in(bad).is_some(), "{bad}");
        }
    }
}
