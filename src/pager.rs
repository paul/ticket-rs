// Shared pager support for commands that produce multi-line output.
//
// When stdout is a TTY the output is piped through the user's preferred pager
// (`TICKET_PAGER`, then `PAGER`). When no pager is configured, when stdout is
// not a TTY, or when `--no-pager` has been set, output is printed directly.

use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::Result;

// ---------------------------------------------------------------------------
// Global pager-disabled flag
// ---------------------------------------------------------------------------

/// Set by `--no-pager` before any command runs.
static PAGER_DISABLED: AtomicBool = AtomicBool::new(false);

/// Disable (or re-enable) paging globally. Called once from `main` when
/// `--no-pager` is present. Mirrors the pattern used by
/// `console::set_colors_enabled`.
pub fn set_pager_disabled(disabled: bool) {
    PAGER_DISABLED.store(disabled, Ordering::Relaxed);
}

/// Returns `true` when paging has been globally disabled via `--no-pager`.
fn pager_disabled() -> bool {
    PAGER_DISABLED.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Print `text` through a pager when appropriate, otherwise print directly.
///
/// Paging is skipped when any of the following are true:
/// - `--no-pager` was passed (global flag set via `set_pager_disabled`)
/// - stdout is not a TTY (scripts, pipes, BDD tests)
/// - neither `TICKET_PAGER` nor `PAGER` env vars are set
///
/// Pager selection: `TICKET_PAGER` > `PAGER`.
/// The pager command is passed to `sh -c` so values like `"less -R"` work
/// without any special argument parsing.
///
/// A broken pipe from the pager (user pressed `q` in `less`) is treated as
/// success, not an error.
pub fn page_or_print(text: &str) -> Result<()> {
    if pager_disabled() || !console::Term::stdout().is_term() {
        print!("{text}");
        return Ok(());
    }

    let pager_cmd = std::env::var("TICKET_PAGER")
        .or_else(|_| std::env::var("PAGER"))
        .ok();

    match pager_cmd {
        None => print!("{text}"),
        Some(cmd) => {
            use std::process::{Command, Stdio};

            let mut child = Command::new("sh")
                .args(["-c", &cmd])
                .stdin(Stdio::piped())
                .spawn()?;

            // Write output to the pager's stdin. A broken pipe means the user
            // quit the pager early — that is not an error.
            if let Some(mut stdin) = child.stdin.take()
                && let Err(e) = stdin.write_all(text.as_bytes())
                && e.kind() != std::io::ErrorKind::BrokenPipe
            {
                return Err(e.into());
            }

            // Wait for the pager to exit. Ignore non-zero exit codes — the
            // user may have quit with `q`, which exits non-zero in some pagers.
            let _ = child.wait();
        }
    }

    Ok(())
}
