use std::path::Path;

use crate::error::SproutError;

/// Load a template file from the template directory.
/// Falls back to a hardcoded default if the file doesn't exist.
pub fn load_template(template_dir: &Path, name: &str) -> Result<String, SproutError> {
    let path = template_dir.join(format!("{name}.md"));
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok("# {{title}}\n".to_string())
        }
        Err(e) => Err(SproutError::ParseError(format!(
            "failed to read template {}: {e}",
            path.display()
        ))),
    }
}

/// Expand template variables.
/// Built-in: `{{title}}` → title, `{{date}}` → YYYY-MM-DD.
/// Shell commands: `{{$(...)}}` → only when `allow_exec` is true.
pub fn expand(
    template: &str,
    title: &str,
    date: &str,
    allow_exec: bool,
) -> Result<String, SproutError> {
    let mut result = template
        .replace("{{title}}", title)
        .replace("{{date}}", date);

    if allow_exec {
        result = expand_exec(&result, title)?;
    }

    Ok(result)
}

/// Expand `{{$(...)}}` patterns by executing shell commands.
fn expand_exec(template: &str, title: &str) -> Result<String, SproutError> {
    use std::process::Command;
    use std::time::Duration;

    let mut result = String::new();
    let mut rest = template;

    while let Some(start) = rest.find("{{$(") {
        result.push_str(&rest[..start]);
        let after_start = &rest[start + 4..]; // skip "{{$("
        let end = after_start.find(")}}").ok_or_else(|| {
            SproutError::ParseError("unclosed {{$(...)}} in template".into())
        })?;
        let cmd = &after_start[..end];

        let child = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .env("SPROUT_TITLE", title)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| SproutError::ParseError(format!("failed to execute template command: {e}")))?;

        let output = wait_with_timeout(child, Duration::from_secs(5))?;

        if !output.status.success() {
            return Err(SproutError::ParseError(format!(
                "template command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let stdout = String::from_utf8(output.stdout).map_err(|_| {
            SproutError::ParseError("template command produced non-UTF-8 output".into())
        })?;

        result.push_str(stdout.trim_end_matches('\n'));
        rest = &after_start[end + 3..]; // skip ")}}"
    }

    result.push_str(rest);
    Ok(result)
}

fn wait_with_timeout(
    child: std::process::Child,
    timeout: std::time::Duration,
) -> Result<std::process::Output, SproutError> {
    use std::thread;

    let (tx, rx) = std::sync::mpsc::channel();
    let child = child;

    thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result.map_err(|e| {
            SproutError::ParseError(format!("template command I/O error: {e}"))
        }),
        Err(_) => Err(SproutError::ParseError(
            "template command timed out (5s)".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_template_fallback() {
        let dir = TempDir::new().unwrap();
        let result = load_template(dir.path(), "nonexistent").unwrap();
        assert_eq!(result, "# {{title}}\n");
    }

    #[test]
    fn test_load_template_from_file() {
        let dir = TempDir::new().unwrap();
        let content = "# {{title}}\n\nCreated: {{date}}\n";
        std::fs::write(dir.path().join("custom.md"), content).unwrap();
        let result = load_template(dir.path(), "custom").unwrap();
        assert_eq!(result, content);
    }

    #[test]
    fn test_expand_builtin_variables() {
        let template = "# {{title}}\n\nCreated: {{date}}\n";
        let result = expand(template, "My Note", "2026-02-27", false).unwrap();
        assert_eq!(result, "# My Note\n\nCreated: 2026-02-27\n");
    }

    #[test]
    fn test_expand_preserves_exec_when_disabled() {
        let template = "# {{title}}\nYear: {{$(date +%Y)}}\n";
        let result = expand(template, "Test", "2026-02-27", false).unwrap();
        assert!(result.contains("{{$(date +%Y)}}"));
    }

    #[test]
    fn test_expand_exec_when_enabled() {
        let template = "# {{title}}\nEcho: {{$(echo hello)}}\n";
        let result = expand(template, "Test", "2026-02-27", true).unwrap();
        assert_eq!(result, "# Test\nEcho: hello\n");
    }

    #[test]
    fn test_expand_exec_receives_sprout_title() {
        let template = "Title: {{$(echo $SPROUT_TITLE)}}\n";
        let result = expand(template, "My Note", "2026-02-27", true).unwrap();
        assert_eq!(result, "Title: My Note\n");
    }
}
