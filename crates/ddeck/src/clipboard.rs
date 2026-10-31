//! Copy text to the system clipboard. The TUI captures the mouse, so the
//! terminal's own selection is unavailable there; `y` copies instead.
//!
//! A local clipboard program first (`pbcopy`, `wl-copy`, `xclip`, `xsel`,
//! `clip`); without one, the OSC 52 escape asks the terminal to do it (works
//! over SSH in most terminals, silently ignored by the rest).

use std::io::Write;
use std::process::{Command, Stdio};

/// Copy `text`; returns how, for the status line.
pub fn copy(text: &str) -> Result<&'static str, String> {
    let programs: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(windows) {
        &[("clip", &[])]
    } else {
        &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ]
    };
    for (program, args) in programs {
        if pipe(program, args, text).is_ok() {
            return Ok(program);
        }
    }
    let mut out = std::io::stdout();
    out.write_all(osc52(text).as_bytes())
        .and_then(|()| out.flush())
        .map_err(|e| format!("clipboard: {e}"))?;
    Ok("terminal")
}

fn pipe(program: &str, args: &[&str], text: &str) -> std::io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(text.as_bytes())?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("{program} failed")))
    }
}

/// `ESC ] 52 ; c ; <base64> BEL`.
fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648() {
        for (input, want) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foobar", "Zm9vYmFy"),
            ("привет", "0L/RgNC40LLQtdGC"),
        ] {
            assert_eq!(base64(input.as_bytes()), want, "{input}");
        }
        assert_eq!(osc52("hi"), "\x1b]52;c;aGk=\x07");
    }
}
