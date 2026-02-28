use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::cli::OutputFormat;
use crate::config::Config;
use crate::error::SproutError;
use crate::note;
use crate::output;
use crate::template;

const PREVIEW_SCRIPT_BAT: &str = r#"#!/bin/sh
cache_dir="$2"
if [ -d "$cache_dir" ]; then
    hash=$(printf '%s' "$1" | cksum | cut -d' ' -f1)
    cached="$cache_dir/$hash.md"
    if [ ! -f "$cached" ]; then
        tmp="$cached.tmp.$$"
        cp -- "$1" "$tmp" 2>/dev/null && mv -f "$tmp" "$cached" || rm -f "$tmp"
    fi
    [ -f "$cached" ] && src="$cached" || src="$1"
else
    src="$1"
fi
end=$(awk 'NR==1 && !/^---/ { print 1; exit } /^---/ && NR>1 { print NR+1; exit } NR>200 { print 1; exit }' "$src")
bat --line-range="${end:-1}:+49" --style=plain --color=always --paging=never -- "$src"
"#;

const PREVIEW_SCRIPT_PLAIN: &str = r#"#!/bin/sh
awk 'NR==1&&/^---/{f=1;next} f&&/^---/{f=0;next} f{next} {if(++n>50)exit;print}' "$1"
"#;

pub fn run_list(
    vault: &Path,
    config: &Config,
    format: &OutputFormat,
) -> Result<(), SproutError> {
    let paths = note::scan_vault_paths(vault, &config.exclude_dirs())
        .map_err(|e| SproutError::VaultNotFound(e.to_string()))?;

    let mut candidates: Vec<(String, String)> = paths
        .into_iter()
        .map(|n| (n.path.to_string_lossy().to_string(), n.relative_path))
        .collect();

    candidates.sort_by(|a, b| a.1.cmp(&b.1));

    output::format_note_candidates(&candidates, format);
    Ok(())
}

pub fn run_create(
    title: &str,
    vault: &Path,
    config: &Config,
    template_name: Option<&str>,
    format: &OutputFormat,
) -> Result<(), SproutError> {
    // Validate title
    validate_title(title)?;

    // Strip .md suffix if present
    let title = title.strip_suffix(".md").unwrap_or(title);

    let vault_canonical = std::fs::canonicalize(vault)
        .map_err(|_| SproutError::VaultNotFound(vault.display().to_string()))?;
    let file_path = vault_canonical.join(format!("{title}.md"));
    let relative_path = format!("{title}.md");

    if file_path.exists() {
        // Idempotent: return existing file info
        output::format_note_created(
            &file_path.to_string_lossy(),
            &relative_path,
            false,
            false,
            format,
        );
        return Ok(());
    }

    // Load and expand template
    let tmpl_name = template_name.unwrap_or_else(|| config.default_template());
    let template_content = template::load_template(&config.template_dir(), tmpl_name)?;
    let today = chrono::Local::now().date_naive().to_string();
    let expanded = template::expand(
        &template_content,
        title,
        &today,
        config.allow_template_exec(),
    )?;

    // Write the file
    note::write_note(&file_path, &expanded)?;

    // Auto-init if configured
    let initialized = if config.auto_init() {
        match super::init::init_note(&file_path, vault, config) {
            Ok(_) => true,
            Err(SproutError::AlreadyInitialized(_)) => false,
            Err(e) => return Err(e),
        }
    } else {
        false
    };

    output::format_note_created(
        &file_path.to_string_lossy(),
        &relative_path,
        true,
        initialized,
        format,
    );

    Ok(())
}

fn cmd_available(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {name} >/dev/null 2>&1")])
        .status()
        .is_ok_and(|s| s.success())
}

fn resolve_editor() -> Result<String, SproutError> {
    std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| SproutError::EditorNotFound)
}

fn open_in_editor(editor: &str, path: &Path) -> Result<(), SproutError> {
    let path_str = path.to_string_lossy();
    let status = Command::new("sh")
        .args(["-c", &format!("{editor} \"$1\""), "--", &path_str])
        .status()
        .map_err(|e| SproutError::FzfError(format!("failed to launch editor: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(SproutError::FzfError(format!(
            "editor exited with {}",
            status
        )))
    }
}

pub fn run_interactive(
    vault: &Path,
    config: &Config,
    template_name: Option<&str>,
    format: &OutputFormat,
) -> Result<(), SproutError> {
    // Check fzf availability; fall back to list mode if missing
    if !cmd_available("fzf") {
        eprintln!("hint: install fzf for interactive note selection");
        return run_list(vault, config, format);
    }

    // Fail fast if no editor is configured
    let editor = resolve_editor()?;

    // Detect bat and set up preview
    let has_bat = cmd_available("bat");

    // bat prewarm: spawn in background, reap in a thread
    let bat_child = if has_bat {
        let child = Command::new("bat")
            .args(["--paging=never", "--style=plain", "--color=always", "/dev/null"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok();
        if let Some(mut c) = child {
            std::thread::spawn(move || {
                let _ = c.wait();
            });
        }
        None::<()> // just need to trigger the spawn
    } else {
        None
    };
    let _ = bat_child; // suppress unused warning

    // Create cache dir for bat preview (auto-cleaned on drop)
    let cache_dir = if has_bat {
        Some(
            tempfile::TempDir::new()
                .map_err(|e| SproutError::FzfError(format!("failed to create cache dir: {e}")))?,
        )
    } else {
        None
    };

    // Write preview script to a temp file
    let script_content = if has_bat {
        PREVIEW_SCRIPT_BAT
    } else {
        PREVIEW_SCRIPT_PLAIN
    };
    let mut script_file = tempfile::NamedTempFile::new()
        .map_err(|e| SproutError::FzfError(format!("failed to create preview script: {e}")))?;
    script_file
        .write_all(script_content.as_bytes())
        .map_err(|e| SproutError::FzfError(format!("failed to write preview script: {e}")))?;

    let script_path = script_file.path().to_string_lossy().to_string();

    // Scan vault for candidates
    let paths = note::scan_vault_paths(vault, &config.exclude_dirs())
        .map_err(|e| SproutError::VaultNotFound(e.to_string()))?;

    let mut candidates: Vec<(String, String)> = paths
        .into_iter()
        .map(|n| (n.path.to_string_lossy().to_string(), n.relative_path))
        .collect();
    candidates.sort_by(|a, b| a.1.cmp(&b.1));

    // Build preview command
    let preview_cmd = if let Some(ref cd) = cache_dir {
        let cd_path = cd.path().to_string_lossy();
        format!("sh '{}' {{1}} '{}'", script_path, cd_path)
    } else {
        format!("sh '{}' {{1}}", script_path)
    };

    // Launch fzf
    let mut fzf = Command::new("fzf")
        .args([
            "--delimiter=\t",
            "--with-nth=2..",
            "--print-query",
            &format!("--preview={preview_cmd}"),
            "--preview-window=right:50%:wrap",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| SproutError::FzfError(format!("failed to start fzf: {e}")))?;

    // Pipe candidates to fzf stdin
    if let Some(ref mut stdin) = fzf.stdin {
        for (abs_path, rel_path) in &candidates {
            let _ = writeln!(stdin, "{}\t{}", abs_path, rel_path);
        }
    }
    drop(fzf.stdin.take()); // close stdin so fzf can process

    let output = fzf
        .wait_with_output()
        .map_err(|e| SproutError::FzfError(format!("fzf wait failed: {e}")))?;

    let exit_code = output.status.code().unwrap_or(2);

    match exit_code {
        130 => {
            // Ctrl-C / Esc: do nothing
            return Ok(());
        }
        2 => {
            return Err(SproutError::FzfError("fzf encountered an error".into()));
        }
        0 | 1 => {
            // Parse output: line 1 = query, line 2 = selected item (if any)
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut lines = stdout.lines();
            let query = lines.next().unwrap_or("").trim();
            let selected = lines.next().unwrap_or("").trim();

            if !selected.is_empty() {
                // User selected an existing note; extract abs_path (first field before tab)
                let abs_path = selected.split('\t').next().unwrap_or(selected);
                let path = std::path::PathBuf::from(abs_path);
                return open_in_editor(&editor, &path);
            }

            if !query.is_empty() {
                // No selection, but query is non-empty: create new note
                run_create(query, vault, config, template_name, format)?;

                // Resolve the created file path and open in editor
                let vault_canonical = std::fs::canonicalize(vault)
                    .map_err(|_| SproutError::VaultNotFound(vault.display().to_string()))?;
                let title = query.strip_suffix(".md").unwrap_or(query);
                let file_path = vault_canonical.join(format!("{title}.md"));
                return open_in_editor(&editor, &file_path);
            }

            // Both empty: do nothing
            Ok(())
        }
        _ => Err(SproutError::FzfError(format!(
            "fzf exited with code {exit_code}"
        ))),
    }
}

fn validate_title(title: &str) -> Result<(), SproutError> {
    if title.is_empty() {
        return Err(SproutError::InvalidTitle("(empty)".into()));
    }
    if title.contains('/') || title.contains('\\') || title.contains('\0') {
        return Err(SproutError::InvalidTitle(title.into()));
    }
    if title.contains("..") {
        return Err(SproutError::InvalidTitle(title.into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_title_valid() {
        assert!(validate_title("My Note").is_ok());
        assert!(validate_title("日本語ノート").is_ok());
        assert!(validate_title("note-with-dashes").is_ok());
        assert!(validate_title("note.md").is_ok());
    }

    #[test]
    fn test_validate_title_empty() {
        assert!(validate_title("").is_err());
    }

    #[test]
    fn test_validate_title_slash() {
        assert!(validate_title("sub/note").is_err());
        assert!(validate_title("sub\\note").is_err());
    }

    #[test]
    fn test_validate_title_traversal() {
        assert!(validate_title("../escape").is_err());
        assert!(validate_title("a..b").is_err());
    }

    #[test]
    fn test_validate_title_null_byte() {
        assert!(validate_title("note\0bad").is_err());
    }
}
