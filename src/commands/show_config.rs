// Implementation of the `show-config` subcommand.
//
// Prints each configuration key, its effective resolved value, and the source
// from which that value came. Defaults are shown as their computed form rather
// than an empty cell, so the user always sees exactly what the tool will use.

use std::path::{Path, PathBuf};

use crate::config::{self, Config, Source};
use crate::error::Result;
use crate::id::{derive_prefix, normalise_prefix};
use crate::store::TicketStore;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Print the resolved configuration and the source of each value.
pub fn show_config() -> Result<()> {
    print!("{}", build_output(config::global(), None));
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal implementation (testable with an explicit Config + start_dir)
// ---------------------------------------------------------------------------

/// Build the show-config output string from an explicit [`Config`] and an
/// optional directory override used for store/cwd resolution.
///
/// Accepting `cfg` directly (rather than calling [`config::global`] inside)
/// makes this fully testable without touching process-global state.
fn build_output(cfg: &Config, start_dir: Option<&Path>) -> String {
    let prefix_line = format_prefix(cfg, start_dir);
    let dir_line = format_dir(cfg, start_dir);
    format!("{prefix_line}\n{dir_line}\n")
}

// ---------------------------------------------------------------------------
// Per-field formatters
// ---------------------------------------------------------------------------

/// Format the `ticket_prefix` line.
///
/// When the value comes from a configured source the configured value is used.
/// When it is the default the effective prefix is derived from the parent
/// directory of the `.tickets/` store (or from cwd if no store exists yet),
/// mirroring what `create` would use.
fn format_prefix(cfg: &Config, start_dir: Option<&Path>) -> String {
    let key = "ticket_prefix";

    // The displayed value always has a trailing `-` so users see exactly what
    // will appear at the start of generated ticket IDs.
    let (value, annotation) = match &cfg.ticket_prefix_source {
        Source::Env(var) => {
            let p = cfg.ticket_prefix.as_deref().unwrap_or_default();
            (format!("{}-", normalise_prefix(p)), format!("env: {var}"))
        }
        Source::File(path) => {
            let p = cfg.ticket_prefix.as_deref().unwrap_or_default();
            (
                format!("{}-", normalise_prefix(p)),
                format!(".tickets.toml: {}", path.display()),
            )
        }
        Source::Default => {
            let derived = derive_default_prefix(start_dir);
            (format!("{derived}-"), "default".to_string())
        }
    };

    format!("{key:<15}  {value:<20}  ({annotation})")
}

/// Format the `ticket_dir` line.
///
/// When the value comes from a configured source it is used directly. When it
/// is the default the effective store path is resolved by walking from
/// `start_dir`, matching what every other command would use.
fn format_dir(cfg: &Config, start_dir: Option<&Path>) -> String {
    let key = "ticket_dir";

    let (value, annotation) = match &cfg.ticket_dir_source {
        Source::Env(var) => (
            cfg.ticket_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            format!("env: {var}"),
        ),
        Source::File(path) => (
            cfg.ticket_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            format!(".tickets.toml: {}", path.display()),
        ),
        Source::Default => {
            let cwd = start_dir
                .map(Path::to_path_buf)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_default();
            let abs = match TicketStore::find(start_dir) {
                Ok(store) => store.dir().to_path_buf(),
                // No store found yet; show what would be created.
                Err(_) => cwd.join(".tickets"),
            };
            // Show the path relative to cwd so it reads naturally in a
            // project context (e.g. `.tickets` rather than `/home/…/.tickets`).
            let rel = relative_to(&abs, &cwd);
            // Ensure the path is unambiguous: prepend `./` unless the path
            // already starts with `./`, `../`, or `/`.
            let display = {
                let s = rel.display().to_string();
                if s.starts_with("./") || s.starts_with("../") || s.starts_with('/') {
                    s
                } else {
                    format!("./{s}")
                }
            };
            (display, "default".to_string())
        }
    };

    format!("{key:<15}  {value:<20}  ({annotation})")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute a relative path from `base` to `target` without external crates.
///
/// Strips the common prefix between the two absolute paths and builds a
/// relative path using `..` components as needed. Falls back to `target` if
/// the paths share no common prefix (e.g. different drive letters on Windows).
fn relative_to(target: &Path, base: &Path) -> PathBuf {
    // Strip the longest common prefix.
    let mut base_iter = base.components();
    let mut target_iter = target.components();
    let mut common_len = 0;

    loop {
        match (base_iter.next(), target_iter.next()) {
            (Some(b), Some(t)) if b == t => common_len += 1,
            _ => break,
        }
    }

    if common_len == 0 {
        return target.to_path_buf();
    }

    let base_components: Vec<_> = base.components().collect();
    let target_components: Vec<_> = target.components().collect();

    let mut rel = PathBuf::new();
    for _ in &base_components[common_len..] {
        rel.push("..");
    }
    for c in &target_components[common_len..] {
        rel.push(c);
    }

    if rel.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        rel
    }
}

/// Derive the default prefix from the parent directory of the `.tickets/`
/// store.  If no store is found, fall back to cwd.
fn derive_default_prefix(start_dir: Option<&Path>) -> String {
    // Try to get the store's parent directory name — that's what `create` uses.
    if let Some(name) = TicketStore::find(start_dir).ok().and_then(|store| {
        store
            .dir()
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty())
            .map(|n| n.to_owned())
    }) {
        return derive_prefix(&name);
    }

    // No store yet — derive from cwd instead.
    let cwd = start_dir
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();

    if let Some(name) = cwd.file_name().and_then(|n| n.to_str()) {
        derive_prefix(name)
    } else {
        String::from("(unknown)")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn make_store(dir: &Path) -> PathBuf {
        let tickets = dir.join(".tickets");
        fs::create_dir_all(&tickets).unwrap();
        tickets
    }

    use std::path::PathBuf;

    #[test]
    fn default_output_contains_both_keys() {
        let dir = TempDir::new().unwrap();
        make_store(dir.path());
        let output = build_output(&Config::default(), Some(dir.path()));
        assert!(
            output.contains("ticket_prefix"),
            "missing ticket_prefix line"
        );
        assert!(output.contains("ticket_dir"), "missing ticket_dir line");
    }

    #[test]
    fn default_prefix_derived_from_dir_name() {
        let dir = TempDir::new().unwrap();
        // Create a nested dir whose name we control.
        let project = dir.path().join("my-project");
        fs::create_dir_all(&project).unwrap();
        make_store(&project);

        let output = build_output(&Config::default(), Some(&project));
        // "my-project" → prefix "mp", displayed as "mp-"
        assert!(
            output.contains("mp-"),
            "expected derived prefix 'mp-' in: {output}"
        );
        assert!(
            output.contains("(default)"),
            "expected '(default)' annotation"
        );
    }

    #[test]
    fn default_dir_shows_relative_path() {
        let dir = TempDir::new().unwrap();
        make_store(dir.path());

        let output = build_output(&Config::default(), Some(dir.path()));
        // Directly inside cwd → should display as "./.tickets".
        assert!(
            output.contains("./.tickets"),
            "expected './.tickets' in output: {output}"
        );
        // Should NOT show the full absolute path when using the default.
        assert!(
            !output.contains(dir.path().to_str().unwrap()),
            "expected relative (not absolute) path in output: {output}"
        );
    }

    #[test]
    fn configured_prefix_with_trailing_dash_normalised() {
        let dir = TempDir::new().unwrap();
        make_store(dir.path());

        let mut cfg = Config::default();
        cfg.ticket_prefix = Some("myp-".to_string());
        cfg.ticket_prefix_source = Source::Env("TICKET_PREFIX");

        let line = format_prefix(&cfg, Some(dir.path()));
        // Should display "myp-" not "myp--"
        assert!(line.contains("myp-"), "value missing: {line}");
        assert!(
            !line.contains("myp--"),
            "double dash should not appear: {line}"
        );
    }

    #[test]
    fn env_source_shown_for_prefix() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        make_store(dir.path());

        unsafe { std::env::set_var("TICKET_PREFIX", "envpfx") };

        // Build a Config manually with env source to avoid the LazyLock.
        let mut cfg = Config::default();
        cfg.ticket_prefix = Some("envpfx".to_string());
        cfg.ticket_prefix_source = Source::Env("TICKET_PREFIX");

        let line = format_prefix(&cfg, Some(dir.path()));

        unsafe { std::env::remove_var("TICKET_PREFIX") };

        assert!(line.contains("envpfx"), "value missing: {line}");
        assert!(
            line.contains("env: TICKET_PREFIX"),
            "source missing: {line}"
        );
    }

    #[test]
    fn file_source_shown_for_dir() {
        let dir = TempDir::new().unwrap();
        make_store(dir.path());
        let toml_path = dir.path().join(".tickets.toml");

        let mut cfg = Config::default();
        cfg.ticket_dir = Some(PathBuf::from("/custom/path"));
        cfg.ticket_dir_source = Source::File(toml_path.clone());

        let line = format_dir(&cfg, Some(dir.path()));

        assert!(line.contains("/custom/path"), "value missing: {line}");
        assert!(
            line.contains(".tickets.toml:"),
            "source annotation missing: {line}"
        );
        assert!(
            line.contains(toml_path.to_str().unwrap()),
            "toml path missing: {line}"
        );
    }
}
