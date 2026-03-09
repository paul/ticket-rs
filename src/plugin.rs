// External plugin discovery and dispatch.

use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;

use crate::store::TicketStore;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Metadata about a discovered external plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// The command name (without the `ticket-` prefix).
    pub name: String,
    /// Full path to the plugin executable.
    pub path: PathBuf,
    /// Description sourced from a `# tk-plugin: <desc>` comment in the first
    /// 10 lines (scripts), or from the output of `<plugin> --tk-describe`
    /// (compiled binaries).  `None` when neither source provides a value.
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Search `PATH` for a `ticket-<cmd>` executable and return its path, or
/// `None` if no such executable is found.
pub fn find_plugin(cmd: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let dirs: Vec<PathBuf> = std::env::split_paths(&path_var).collect();
    find_plugin_in_dirs(cmd, &dirs)
}

/// Execute a plugin, forwarding `args` to it.
///
/// Sets `TICKETS_DIR` (resolved from the current directory, best-effort) and
/// `TK_SCRIPT` (path to the current binary) in the child environment.
/// Waits for the child to finish and forwards its exit code.
pub fn exec_plugin(plugin_path: &Path, args: &[OsString]) {
    let env = build_plugin_env();

    let status = process::Command::new(plugin_path)
        .args(args)
        .envs(env)
        .status()
        .unwrap_or_else(|e| {
            eprintln!(
                "ticket: failed to execute plugin '{}': {e}",
                plugin_path.display()
            );
            process::exit(1);
        });

    process::exit(status.code().unwrap_or(1));
}

/// Scan `PATH` for all `ticket-*` executables and return their metadata.
pub fn discover_plugins() -> Vec<PluginInfo> {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let dirs: Vec<PathBuf> = std::env::split_paths(&path_var).collect();
    discover_plugins_in_dirs(&dirs)
}

// ---------------------------------------------------------------------------
// Internal helpers (pub(crate) for unit tests)
// ---------------------------------------------------------------------------

/// Same as [`find_plugin`] but accepts an explicit directory list instead of
/// reading `PATH`, making it straightforward to test.
pub(crate) fn find_plugin_in_dirs(cmd: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let file_name = format!("ticket-{cmd}");
    for dir in dirs {
        let candidate = dir.join(&file_name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Same as [`discover_plugins`] but accepts an explicit directory list.
pub(crate) fn discover_plugins_in_dirs(dirs: &[PathBuf]) -> Vec<PluginInfo> {
    // Keyed by name; first occurrence in PATH wins (standard PATH semantics).
    let mut seen: std::collections::HashMap<String, PluginInfo> = std::collections::HashMap::new();

    for dir in dirs {
        collect_prefixed(dir, "ticket-", &mut seen);
    }

    let mut plugins: Vec<PluginInfo> = seen.into_values().collect();
    plugins.sort_by(|a, b| a.name.cmp(&b.name));
    plugins
}

/// Build the environment additions passed to every plugin process.
///
/// Returns a `Vec` of `(key, value)` pairs so the env-building logic is
/// independently testable without spawning a child process.
pub(crate) fn build_plugin_env() -> Vec<(OsString, OsString)> {
    let mut env: Vec<(OsString, OsString)> = Vec::new();

    // TICKETS_DIR — best-effort; silently omit when no .tickets/ is found.
    if let Ok(store) = TicketStore::find(None) {
        env.push((
            OsString::from("TICKETS_DIR"),
            OsString::from(store.dir().as_os_str()),
        ));
    }

    // TK_SCRIPT — path to this binary so plugins can call `super`.
    if let Ok(exe) = std::env::current_exe() {
        env.push((OsString::from("TK_SCRIPT"), OsString::from(exe.as_os_str())));
    }

    env
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Walk `dir` for executables whose names start with `prefix`, strip the
/// prefix to obtain the plugin name, and insert into `seen` if not already
/// present (first-writer wins, matching standard PATH semantics).
fn collect_prefixed(
    dir: &Path,
    prefix: &str,
    seen: &mut std::collections::HashMap<String, PluginInfo>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name_str = match file_name.to_str() {
            Some(s) => s,
            None => continue,
        };

        // Must start with the prefix and have at least one character after it.
        let plugin_name = match name_str.strip_prefix(prefix) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };

        // First PATH entry wins — skip if already recorded.
        if seen.contains_key(&plugin_name) {
            continue;
        }

        let path = entry.path();
        if !is_executable(&path) {
            continue;
        }

        let description = extract_description(&path);
        seen.insert(
            plugin_name.clone(),
            PluginInfo {
                name: plugin_name,
                path,
                description,
            },
        );
    }
}

/// Return `true` if `path` is a regular file with at least one executable bit
/// set.
fn is_executable(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && (meta.permissions().mode() & 0o111 != 0),
        Err(_) => false,
    }
}

/// Extract a human-readable description from `path`.
///
/// Strategy:
/// 1. Read the first 10 lines of the file.  If any line contains a
///    `# tk-plugin: <desc>` comment, return that description.
/// 2. If no comment is found (e.g. a compiled binary), run the plugin with
///    `--tk-describe` and capture its trimmed stdout as the description.
/// 3. Return `None` if neither approach yields text.
fn extract_description(path: &Path) -> Option<String> {
    if let Some(desc) = read_comment_description(path) {
        return Some(desc);
    }
    run_tk_describe(path)
}

/// Read the first 10 lines of `path` looking for `# tk-plugin: <desc>`.
fn read_comment_description(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().take(10) {
        let line = line.ok()?;
        if let Some(rest) = line.trim().strip_prefix("# tk-plugin:") {
            let desc = rest.trim().to_string();
            if !desc.is_empty() {
                return Some(desc);
            }
        }
    }

    None
}

/// Run `<path> --tk-describe` with a short timeout and return the trimmed
/// stdout if the command exits successfully with non-empty output.
fn run_tk_describe(path: &Path) -> Option<String> {
    let output = process::Command::new(path)
        .arg("--tk-describe")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Write an executable file at `dir/name` with `content`.
    fn make_executable(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Write a non-executable file at `dir/name`.
    fn make_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        path
    }

    // -----------------------------------------------------------------------
    // Discovery — prefix
    // -----------------------------------------------------------------------

    #[test]
    fn ticket_prefix_discovered() {
        let dir = tempdir().unwrap();
        make_executable(
            dir.path(),
            "ticket-greet",
            "#!/usr/bin/env bash\necho greet\n",
        );

        let plugins = discover_plugins_in_dirs(&[dir.path().to_path_buf()]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "greet");
    }

    #[test]
    fn non_plugin_executables_ignored() {
        let dir = tempdir().unwrap();
        make_executable(dir.path(), "ticket", "#!/usr/bin/env bash\n");
        make_executable(dir.path(), "ticketd", "#!/usr/bin/env bash\n");
        // A legitimate plugin to confirm the function still works.
        make_executable(dir.path(), "ticket-real", "#!/usr/bin/env bash\n");

        let plugins = discover_plugins_in_dirs(&[dir.path().to_path_buf()]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "real");
    }

    #[test]
    fn non_executable_files_ignored() {
        let dir = tempdir().unwrap();
        make_file(
            dir.path(),
            "ticket-noexec",
            "#!/usr/bin/env bash\necho hi\n",
        );

        let plugins = discover_plugins_in_dirs(&[dir.path().to_path_buf()]);
        assert!(plugins.is_empty());
    }

    /// First entry in PATH wins when the same plugin name appears in multiple
    /// directories (standard PATH precedence semantics).
    #[test]
    fn first_path_dir_wins_for_same_name() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();
        make_executable(
            dir1.path(),
            "ticket-test",
            "#!/usr/bin/env bash\necho first\n",
        );
        make_executable(
            dir2.path(),
            "ticket-test",
            "#!/usr/bin/env bash\necho second\n",
        );

        let plugins =
            discover_plugins_in_dirs(&[dir1.path().to_path_buf(), dir2.path().to_path_buf()]);
        assert_eq!(plugins.len(), 1);
        // Should be the one from dir1.
        assert!(plugins[0].path.starts_with(dir1.path()));
    }

    // -----------------------------------------------------------------------
    // Discovery — descriptions
    // -----------------------------------------------------------------------

    #[test]
    fn no_description_returns_none() {
        let dir = tempdir().unwrap();
        // No comment; exits non-zero when called with --tk-describe so neither
        // description source yields a value.
        let content = "#!/usr/bin/env bash\n\
            if [ \"$1\" = \"--tk-describe\" ]; then exit 1; fi\n\
            echo hi\n";
        make_executable(dir.path(), "ticket-nodesc", content);

        let plugins = discover_plugins_in_dirs(&[dir.path().to_path_buf()]);
        assert_eq!(plugins.len(), 1);
        assert!(plugins[0].description.is_none());
    }

    #[test]
    fn description_from_tk_describe_flag() {
        let dir = tempdir().unwrap();
        // Script: outputs description when called with --tk-describe, otherwise
        // does something else.  No # tk-plugin: comment present.
        let content = "#!/usr/bin/env bash\n\
            if [ \"$1\" = \"--tk-describe\" ]; then\n\
              echo \"Binary plugin description\"\n\
              exit 0\n\
            fi\n\
            echo \"running\"\n";
        make_executable(dir.path(), "ticket-bindesc", content);

        let plugins = discover_plugins_in_dirs(&[dir.path().to_path_buf()]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(
            plugins[0].description.as_deref(),
            Some("Binary plugin description")
        );
    }

    /// Comment takes precedence over --tk-describe when both are present.
    #[test]
    fn comment_takes_precedence_over_tk_describe() {
        let dir = tempdir().unwrap();
        let content = "#!/usr/bin/env bash\n\
            # tk-plugin: Comment description\n\
            if [ \"$1\" = \"--tk-describe\" ]; then\n\
              echo \"Flag description\"\n\
              exit 0\n\
            fi\n\
            echo \"running\"\n";
        make_executable(dir.path(), "ticket-both", content);

        let plugins = discover_plugins_in_dirs(&[dir.path().to_path_buf()]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(
            plugins[0].description.as_deref(),
            Some("Comment description")
        );
    }

    // -----------------------------------------------------------------------
    // find_plugin tests
    // -----------------------------------------------------------------------

    #[test]
    fn command_not_in_path_returns_none() {
        let dir = tempdir().unwrap();
        let result = find_plugin_in_dirs("nonexistent", &[dir.path().to_path_buf()]);
        assert!(result.is_none());
    }

    #[test]
    fn find_plugin_returns_ticket_prefix_path() {
        let dir = tempdir().unwrap();
        make_executable(
            dir.path(),
            "ticket-hello",
            "#!/usr/bin/env bash\necho hello\n",
        );

        let result = find_plugin_in_dirs("hello", &[dir.path().to_path_buf()]);
        assert!(result.is_some());
        let name = result
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(name, "ticket-hello");
    }

    // -----------------------------------------------------------------------
    // build_plugin_env tests
    // -----------------------------------------------------------------------

    #[test]
    fn tk_script_set_in_env() {
        let env = build_plugin_env();
        let entry = env.iter().find(|(k, _)| k == "TK_SCRIPT");
        assert!(entry.is_some(), "TK_SCRIPT should be present in plugin env");
        let value = entry.unwrap().1.to_string_lossy();
        assert!(!value.is_empty(), "TK_SCRIPT should not be empty");
    }

    #[test]
    fn tickets_dir_set_when_store_found() {
        let dir = tempdir().unwrap();
        let tickets_dir = dir.path().join(".tickets");
        fs::create_dir(&tickets_dir).unwrap();

        // Override TICKETS_DIR so TicketStore::find() resolves to our temp dir.
        // SAFETY: single-threaded test; env var is restored before returning.
        unsafe { std::env::set_var("TICKETS_DIR", &tickets_dir) };
        let env = build_plugin_env();
        unsafe { std::env::remove_var("TICKETS_DIR") };

        let entry = env.iter().find(|(k, _)| k == "TICKETS_DIR");
        assert!(
            entry.is_some(),
            "TICKETS_DIR should be set when store is found"
        );
        let value = entry.unwrap().1.to_string_lossy();
        assert!(
            value.contains(".tickets"),
            "TICKETS_DIR should contain '.tickets', got: {value}"
        );
    }
}
