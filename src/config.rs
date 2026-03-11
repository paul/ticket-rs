// Project-local configuration loaded from `.tickets.toml`.
//
// Configuration is resolved once at startup and stored in a `LazyLock` so any
// module can access it without threading values through every call site.
//
// Precedence (highest wins):
//   1. Environment variables (`TICKET_DIR`, `TICKET_PREFIX`)
//   2. `.tickets.toml` found by walking upward from cwd
//   3. Built-in defaults (both fields absent)
//
// `TICKETS_DIR` is kept as a legacy alias for `TICKET_DIR`; `TICKET_DIR` takes
// priority when both are set.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Public config type
// ---------------------------------------------------------------------------

/// Project-local configuration for ticket-rs.
///
/// Loaded once via [`global`] and cached for the lifetime of the process.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Override the automatically derived ticket ID prefix.
    pub ticket_prefix: Option<String>,
    /// Override the default `.tickets/` directory path.
    pub ticket_dir: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Global instance
// ---------------------------------------------------------------------------

static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

/// Return a reference to the process-global [`Config`].
pub fn global() -> &'static Config {
    &CONFIG
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

impl Config {
    /// Load configuration by reading `.tickets.toml` (upward walk from cwd)
    /// and then overlaying environment variables.
    ///
    /// Errors during file discovery or parsing are silently ignored; the
    /// resulting config simply retains the defaults for any unparseable field.
    /// This matches the behaviour of tools like `rustfmt` and `cargo` that
    /// degrade gracefully when their config files are malformed.
    fn load() -> Self {
        let mut cfg = Self::from_file().unwrap_or_default();
        cfg.apply_env();
        cfg
    }

    /// Walk upward from the current working directory looking for
    /// `.tickets.toml`.  Returns `None` if no file is found or if the file
    /// cannot be parsed.
    fn from_file() -> Option<Self> {
        let cwd = std::env::current_dir().ok()?;
        let path = find_config_file(&cwd)?;
        let contents = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&contents).ok()
    }

    /// Override fields from environment variables.
    ///
    /// `TICKET_DIR` (or the legacy `TICKETS_DIR`) sets `ticket_dir`.
    /// `TICKET_PREFIX` sets `ticket_prefix`.
    fn apply_env(&mut self) {
        // TICKET_DIR takes priority; fall back to TICKETS_DIR for compat.
        if let Some(val) = std::env::var("TICKET_DIR")
            .ok()
            .or_else(|| std::env::var("TICKETS_DIR").ok())
        {
            self.ticket_dir = Some(PathBuf::from(val));
        }

        if let Ok(val) = std::env::var("TICKET_PREFIX") {
            self.ticket_prefix = Some(val);
        }
    }
}

// ---------------------------------------------------------------------------
// File discovery
// ---------------------------------------------------------------------------

/// Walk ancestor directories starting from `start`, returning the first
/// `.tickets.toml` found, or `None` if the filesystem root is reached.
fn find_config_file(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    loop {
        let candidate = current.join(".tickets.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_config(dir: &Path, contents: &str) {
        let path = dir.join(".tickets.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn parse_both_fields() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"
ticket_prefix = "myp"
ticket_dir = "/tmp/tickets"
"#,
        );
        let path = find_config_file(dir.path()).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let cfg: Config = toml::from_str(&contents).unwrap();
        assert_eq!(cfg.ticket_prefix.as_deref(), Some("myp"));
        assert_eq!(cfg.ticket_dir, Some(PathBuf::from("/tmp/tickets")));
    }

    #[test]
    fn parse_prefix_only() {
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "ticket_prefix = \"abc\"\n");
        let path = find_config_file(dir.path()).unwrap();
        let cfg: Config = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(cfg.ticket_prefix.as_deref(), Some("abc"));
        assert!(cfg.ticket_dir.is_none());
    }

    #[test]
    fn missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        // No .tickets.toml created.
        assert!(find_config_file(dir.path()).is_none());
    }

    #[test]
    fn finds_file_in_ancestor() {
        let root = TempDir::new().unwrap();
        write_config(root.path(), "ticket_prefix = \"root\"\n");
        // Create a nested subdirectory; the file should be found in root.
        let nested = root.path().join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        let found = find_config_file(&nested).unwrap();
        assert_eq!(found, root.path().join(".tickets.toml"));
    }

    #[test]
    fn env_ticket_dir_overrides_file() {
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "ticket_dir = \"/from/file\"\n");
        let path = find_config_file(dir.path()).unwrap();
        let mut cfg: Config = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        // Simulate env override.
        // SAFETY: single-threaded test process; no other threads reading env.
        unsafe { std::env::set_var("TICKET_DIR", "/from/env") };
        cfg.apply_env();
        unsafe { std::env::remove_var("TICKET_DIR") };
        assert_eq!(cfg.ticket_dir, Some(PathBuf::from("/from/env")));
    }

    #[test]
    fn legacy_tickets_dir_fallback() {
        let mut cfg = Config::default();
        // SAFETY: single-threaded test process; no other threads reading env.
        unsafe { std::env::set_var("TICKETS_DIR", "/legacy/path") };
        cfg.apply_env();
        unsafe { std::env::remove_var("TICKETS_DIR") };
        assert_eq!(cfg.ticket_dir, Some(PathBuf::from("/legacy/path")));
    }

    #[test]
    fn ticket_dir_takes_priority_over_tickets_dir() {
        let mut cfg = Config::default();
        // SAFETY: single-threaded test process; no other threads reading env.
        unsafe {
            std::env::set_var("TICKET_DIR", "/new");
            std::env::set_var("TICKETS_DIR", "/old");
        }
        cfg.apply_env();
        unsafe {
            std::env::remove_var("TICKET_DIR");
            std::env::remove_var("TICKETS_DIR");
        }
        assert_eq!(cfg.ticket_dir, Some(PathBuf::from("/new")));
    }

    #[test]
    fn env_ticket_prefix_overrides_file() {
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "ticket_prefix = \"file\"\n");
        let path = find_config_file(dir.path()).unwrap();
        let mut cfg: Config = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        // SAFETY: single-threaded test process; no other threads reading env.
        unsafe { std::env::set_var("TICKET_PREFIX", "env") };
        cfg.apply_env();
        unsafe { std::env::remove_var("TICKET_PREFIX") };
        assert_eq!(cfg.ticket_prefix.as_deref(), Some("env"));
    }
}
