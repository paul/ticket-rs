// CLI argument definitions using clap derive.

use clap::{Parser, Subcommand, ValueEnum};

/// When to use colored output.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum ColorWhen {
    /// Defer to console's TTY and NO_COLOR/CLICOLOR detection.
    #[default]
    Auto,
    /// Force colors on.
    Always,
    /// Force colors off.
    Never,
}

/// A local-first issue tracker backed by plain-text files.
#[derive(Debug, Parser)]
#[command(name = "ticket", version)]
pub struct Cli {
    /// When to use colored output.
    #[arg(long, value_enum, default_value_t, global = true)]
    pub color: ColorWhen,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    // ── Phase 1: core commands ──────────────────────────────────────
    /// Create a new ticket.
    Create {
        /// Title for the new ticket (defaults to "Untitled").
        title: Option<String>,

        /// Description text.
        #[arg(short, long)]
        description: Option<String>,

        /// Design section text.
        #[arg(long)]
        design: Option<String>,

        /// Acceptance criteria section text.
        #[arg(long)]
        acceptance: Option<String>,

        /// Ticket type (bug, feature, task, epic, chore).
        #[arg(short = 't', long = "type", default_value = "task")]
        ticket_type: String,

        /// Priority (0-4).
        #[arg(short, long, default_value = "2")]
        priority: String,

        /// Assignee name (defaults to git user.name).
        #[arg(short, long)]
        assignee: Option<String>,

        /// External reference (e.g. GitHub issue URL).
        #[arg(long)]
        external_ref: Option<String>,

        /// Parent ticket ID.
        #[arg(long)]
        parent: Option<String>,

        /// Comma-separated tags.
        #[arg(long)]
        tags: Option<String>,
    },

    /// Display a ticket's full content.
    Show {
        /// Ticket ID (supports partial matching).
        id: String,
    },

    /// Set a ticket's status to in_progress.
    Start {
        /// Ticket ID (supports partial matching).
        id: String,
    },

    /// Set a ticket's status to closed.
    Close {
        /// Ticket ID (supports partial matching).
        id: String,
    },

    /// Set a ticket's status to open.
    Reopen {
        /// Ticket ID (supports partial matching).
        id: String,
    },

    /// Set a ticket's status explicitly.
    Status {
        /// Ticket ID (supports partial matching).
        id: String,

        /// New status (open, in_progress, closed).
        status: String,
    },

    // ── Phase 2: dependency & link management ───────────────────────
    /// Manage ticket dependencies (add, remove, tree view).
    Dep {
        #[command(subcommand)]
        command: DepCommands,
    },

    /// Create symmetric links between tickets.
    Link {
        /// Ticket IDs to link together (2 or more, supports partial matching).
        #[arg(required = true, num_args = 2..)]
        ids: Vec<String>,
    },

    /// Remove a symmetric link between tickets.
    Unlink {
        /// Source ticket ID (supports partial matching).
        id: String,
        /// Target ticket ID to unlink (supports partial matching).
        target_id: String,
    },

    // ── Phase 3: listing & querying ─────────────────────────────────
    /// List tickets with optional filters.
    Ls,

    /// Show tickets that are ready to work on (all deps closed).
    Ready,

    /// Show tickets blocked by unclosed dependencies.
    Blocked,

    /// Show recently closed tickets.
    Closed,

    // ── Phase 4: update & notes ─────────────────────────────────────
    /// Modify a ticket's fields.
    Update,

    /// Append a timestamped note to a ticket.
    AddNote,

    // ── Phase 5: display commands ───────────────────────────────────
    /// Display parent/child hierarchy tree.
    Tree,

    /// Serialize tickets to JSON (with optional jq filter).
    Query,

    // ── Phase 6: plugin & advanced ──────────────────────────────────
    /// Open a ticket in $EDITOR.
    Edit,

    /// Bypass plugin discovery and call a built-in command directly.
    Super,
}

/// Subcommands for `dep`.
#[derive(Debug, Subcommand)]
pub enum DepCommands {
    /// Add a dependency between two tickets.
    Add {
        /// Source ticket ID (supports partial matching).
        id: String,
        /// Dependency ticket ID to add (supports partial matching).
        dep_id: String,
    },

    /// Remove a dependency from a ticket.
    Remove {
        /// Source ticket ID (supports partial matching).
        id: String,
        /// Dependency ticket ID to remove (supports partial matching).
        dep_id: String,
    },

    /// Display the dependency tree for a ticket.
    Tree {
        /// Show all occurrences of each ticket (disable deduplication).
        #[arg(long)]
        full: bool,
        /// Root ticket ID (supports partial matching).
        id: String,
    },

    /// Detect dependency cycles among open and in-progress tickets.
    Cycle,
}
