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
// Source — tracks where a config value came from
// ---------------------------------------------------------------------------

/// The origin of a resolved configuration value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// No configuration was set; the built-in default applies.
    Default,
    /// Value came from a `.tickets.toml` file at the given path.
    File(PathBuf),
    /// Value came from an environment variable with the given name.
    Env(&'static str),
}

// ---------------------------------------------------------------------------
// Raw TOML shape (private — only used for deserialization)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    ticket_prefix: Option<String>,
    ticket_dir: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Public config type
// ---------------------------------------------------------------------------

/// Project-local configuration for ticket-rs, with per-field source tracking.
///
/// Loaded once via [`global`] and cached for the lifetime of the process.
#[derive(Debug)]
pub struct Config {
    /// Override the automatically derived ticket ID prefix.
    pub ticket_prefix: Option<String>,
    /// Where `ticket_prefix` came from.
    pub ticket_prefix_source: Source,

    /// Override the default `.tickets/` directory path.
    pub ticket_dir: Option<PathBuf>,
    /// Where `ticket_dir` came from.
    pub ticket_dir_source: Source,

    /// Path to the `.tickets.toml` file that was loaded, if any.
    pub config_file: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ticket_prefix: None,
            ticket_prefix_source: Source::Default,
            ticket_dir: None,
            ticket_dir_source: Source::Default,
            config_file: None,
        }
    }
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
    /// `.tickets.toml`. Returns `None` if no file is found or if the file
    /// cannot be parsed.
    fn from_file() -> Option<Self> {
        let cwd = std::env::current_dir().ok()?;
        let path = find_config_file(&cwd)?;
        let contents = std::fs::read_to_string(&path).ok()?;
        let raw: RawConfig = toml::from_str(&contents).ok()?;

        let mut cfg = Config {
            config_file: Some(path.clone()),
            ..Config::default()
        };

        if let Some(v) = raw.ticket_prefix {
            cfg.ticket_prefix = Some(v);
            cfg.ticket_prefix_source = Source::File(path.clone());
        }
        if let Some(v) = raw.ticket_dir {
            cfg.ticket_dir = Some(v);
            cfg.ticket_dir_source = Source::File(path);
        }

        Some(cfg)
    }

    /// Override fields from environment variables.
    ///
    /// `TICKET_DIR` (or the legacy `TICKETS_DIR`) sets `ticket_dir`.
    /// `TICKET_PREFIX` sets `ticket_prefix`.
    fn apply_env(&mut self) {
        // TICKET_DIR takes priority; fall back to TICKETS_DIR for compat.
        if let Ok(val) = std::env::var("TICKET_DIR") {
            self.ticket_dir = Some(PathBuf::from(val));
            self.ticket_dir_source = Source::Env("TICKET_DIR");
        } else if let Ok(val) = std::env::var("TICKETS_DIR") {
            self.ticket_dir = Some(PathBuf::from(val));
            self.ticket_dir_source = Source::Env("TICKETS_DIR");
        }

        if let Ok(val) = std::env::var("TICKET_PREFIX") {
            self.ticket_prefix = Some(val);
            self.ticket_prefix_source = Source::Env("TICKET_PREFIX");
        }
    }
}

// ---------------------------------------------------------------------------
// File discovery
// ---------------------------------------------------------------------------

/// Walk ancestor directories starting from `start`, returning the first
/// `.tickets.toml` found, or `None` if the filesystem root is reached.
pub fn find_config_file(start: &Path) -> Option<PathBuf> {
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
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Env var mutations are not thread-safe; serialize all tests that call
    // set_var/remove_var behind this mutex.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
        let raw: RawConfig = toml::from_str(&contents).unwrap();
        assert_eq!(raw.ticket_prefix.as_deref(), Some("myp"));
        assert_eq!(raw.ticket_dir, Some(PathBuf::from("/tmp/tickets")));
    }

    #[test]
    fn parse_prefix_only() {
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "ticket_prefix = \"abc\"\n");
        let path = find_config_file(dir.path()).unwrap();
        let raw: RawConfig = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(raw.ticket_prefix.as_deref(), Some("abc"));
        assert!(raw.ticket_dir.is_none());
    }

    #[test]
    fn missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        assert!(find_config_file(dir.path()).is_none());
    }

    #[test]
    fn finds_file_in_ancestor() {
        let root = TempDir::new().unwrap();
        write_config(root.path(), "ticket_prefix = \"root\"\n");
        let nested = root.path().join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        let found = find_config_file(&nested).unwrap();
        assert_eq!(found, root.path().join(".tickets.toml"));
    }

    #[test]
    fn from_file_records_file_source() {
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "ticket_prefix = \"fp\"\n");
        // Simulate loading by pointing find_config_file at the temp dir.
        let path = find_config_file(dir.path()).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let raw: RawConfig = toml::from_str(&contents).unwrap();
        // Build a Config manually the same way from_file does.
        let mut cfg = Config::default();
        cfg.config_file = Some(path.clone());
        if let Some(v) = raw.ticket_prefix {
            cfg.ticket_prefix = Some(v);
            cfg.ticket_prefix_source = Source::File(path.clone());
        }
        assert_eq!(cfg.ticket_prefix_source, Source::File(path));
        assert_eq!(cfg.ticket_dir_source, Source::Default);
    }

    #[test]
    fn env_ticket_dir_overrides_file() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "ticket_dir = \"/from/file\"\n");
        let path = find_config_file(dir.path()).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let raw: RawConfig = toml::from_str(&contents).unwrap();
        let mut cfg = Config::default();
        cfg.config_file = Some(path.clone());
        if let Some(v) = raw.ticket_dir {
            cfg.ticket_dir = Some(v);
            cfg.ticket_dir_source = Source::File(path);
        }
        unsafe { std::env::set_var("TICKET_DIR", "/from/env") };
        cfg.apply_env();
        unsafe { std::env::remove_var("TICKET_DIR") };
        assert_eq!(cfg.ticket_dir, Some(PathBuf::from("/from/env")));
        assert_eq!(cfg.ticket_dir_source, Source::Env("TICKET_DIR"));
    }

    #[test]
    fn legacy_tickets_dir_fallback() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut cfg = Config::default();
        unsafe { std::env::set_var("TICKETS_DIR", "/legacy/path") };
        cfg.apply_env();
        unsafe { std::env::remove_var("TICKETS_DIR") };
        assert_eq!(cfg.ticket_dir, Some(PathBuf::from("/legacy/path")));
        assert_eq!(cfg.ticket_dir_source, Source::Env("TICKETS_DIR"));
    }

    #[test]
    fn ticket_dir_takes_priority_over_tickets_dir() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut cfg = Config::default();
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
        assert_eq!(cfg.ticket_dir_source, Source::Env("TICKET_DIR"));
    }

    #[test]
    fn env_ticket_prefix_overrides_file() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "ticket_prefix = \"file\"\n");
        let path = find_config_file(dir.path()).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let raw: RawConfig = toml::from_str(&contents).unwrap();
        let mut cfg = Config::default();
        cfg.config_file = Some(path.clone());
        if let Some(v) = raw.ticket_prefix {
            cfg.ticket_prefix = Some(v);
            cfg.ticket_prefix_source = Source::File(path);
        }
        unsafe { std::env::set_var("TICKET_PREFIX", "env") };
        cfg.apply_env();
        unsafe { std::env::remove_var("TICKET_PREFIX") };
        assert_eq!(cfg.ticket_prefix.as_deref(), Some("env"));
        assert_eq!(cfg.ticket_prefix_source, Source::Env("TICKET_PREFIX"));
    }
}
