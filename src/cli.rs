// CLI argument definitions using clap derive.

use std::ffi::OsString;

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::engine::ArgValueCompleter;

use crate::complete::TicketIdCompleter;

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
#[command(name = "ticket", version = concat!(env!("TICKET_VERSION_DATE"), " (compiled at ", env!("TICKET_COMPILED_AT"), ")"))]
pub struct Cli {
    /// When to use colored output.
    #[arg(long, value_enum, default_value_t, global = true)]
    pub color: ColorWhen,

    /// Disable the pager and print output directly to stdout.
    #[arg(long, global = true)]
    pub no_pager: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    // ── Phase 1: core commands ──────────────────────────────────────
    /// Create a new ticket.
    #[command(visible_alias = "new", visible_alias = "add")]
    Create {
        /// Title for the new ticket (positional form).
        title: Option<String>,

        /// Title for the new ticket (flag form, alias for the positional argument).
        #[arg(long = "title", value_name = "TITLE", conflicts_with = "title")]
        title_flag: Option<String>,

        /// Description text.
        #[arg(short, long, visible_alias = "body")]
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
        #[arg(long, add = ArgValueCompleter::new(TicketIdCompleter))]
        parent: Option<String>,

        /// Comma-separated tags.
        #[arg(long)]
        tags: Option<String>,
    },

    /// Display a ticket's full content.
    #[command(trailing_var_arg = true)]
    Show {
        /// Ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,

        /// Unrecognised arguments — ignored with a warning on stderr.
        #[arg(hide = true, allow_hyphen_values = true, num_args = 0..)]
        extra: Vec<std::ffi::OsString>,
    },

    /// Set a ticket's status to in_progress.
    Start {
        /// Ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,
    },

    /// Set a ticket's status to closed.
    #[command(alias = "done")]
    Close {
        /// Ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,
    },

    /// Set a ticket's status to open.
    Reopen {
        /// Ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,
    },

    /// Set a ticket's status explicitly.
    Status {
        /// Ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
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
        #[arg(required = true, num_args = 2.., add = ArgValueCompleter::new(TicketIdCompleter))]
        ids: Vec<String>,
    },

    /// Remove a symmetric link between tickets.
    Unlink {
        /// Source ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,
        /// Target ticket ID to unlink (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        target_id: String,
    },

    // ── Phase 3: listing & querying ─────────────────────────────────
    /// List tickets with optional filters.
    #[command(alias = "list")]
    Ls {
        /// Filter by status (open, in_progress, closed).
        #[arg(long)]
        status: Option<String>,

        /// Filter by assignee name.
        #[arg(short = 'a', long)]
        assignee: Option<String>,

        /// Filter by tag.
        #[arg(short = 'T', long = "tags", visible_alias = "tag")]
        tag: Option<String>,
    },

    /// Show tickets that are ready to work on (all deps closed).
    Ready {
        /// Filter by assignee name.
        #[arg(short = 'a', long)]
        assignee: Option<String>,

        /// Filter by tag.
        #[arg(short = 'T', long = "tags", visible_alias = "tag")]
        tag: Option<String>,
    },

    /// Show tickets blocked by unclosed dependencies.
    Blocked {
        /// Filter by assignee name.
        #[arg(short = 'a', long)]
        assignee: Option<String>,

        /// Filter by tag.
        #[arg(short = 'T', long = "tags", visible_alias = "tag")]
        tag: Option<String>,
    },

    /// Show recently closed tickets.
    Closed {
        /// Maximum number of tickets to show (default 20).
        #[arg(long, default_value_t = 20)]
        limit: usize,

        /// Filter by assignee name.
        #[arg(short = 'a', long)]
        assignee: Option<String>,

        /// Filter by tag.
        #[arg(short = 'T', long = "tags", visible_alias = "tag")]
        tag: Option<String>,
    },

    // ── Phase 4: update & notes ─────────────────────────────────────
    /// Modify a ticket's fields.
    Update {
        /// Ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,

        /// New title (replaces the # heading).
        #[arg(long)]
        title: Option<String>,

        /// New description (replaces text between title and first ## heading).
        #[arg(short, long)]
        description: Option<String>,

        /// New ## Design section content (replaces or inserts the section).
        #[arg(long)]
        design: Option<String>,

        /// New ## Acceptance Criteria section content (replaces or inserts the section).
        #[arg(long)]
        acceptance: Option<String>,

        /// New priority (0-4).
        #[arg(short, long)]
        priority: Option<String>,

        /// New ticket type (bug, feature, task, epic, chore).
        #[arg(short = 't', long = "type")]
        ticket_type: Option<String>,

        /// New assignee name.
        #[arg(short, long)]
        assignee: Option<String>,

        /// New external reference (e.g. GitHub issue URL).
        #[arg(long)]
        external_ref: Option<String>,

        /// New parent ticket ID (validated to exist).
        #[arg(long, add = ArgValueCompleter::new(TicketIdCompleter))]
        parent: Option<String>,

        /// Replace all tags with this comma-separated list.
        #[arg(long, conflicts_with_all = ["add_tags", "remove_tags"])]
        tags: Option<String>,

        /// Merge these comma-separated tags (deduplicated).
        #[arg(long, conflicts_with = "tags")]
        add_tags: Option<String>,

        /// Remove these comma-separated tags (delete field if result is empty).
        #[arg(long, conflicts_with = "tags")]
        remove_tags: Option<String>,
    },

    /// Append a timestamped note to a ticket.
    #[command(visible_alias = "note")]
    AddNote {
        /// Ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,

        /// Note text. If omitted, reads from stdin.
        text: Option<String>,
    },

    // ── Phase 5: display commands ───────────────────────────────────
    /// Display parent/child hierarchy tree.
    Tree {
        /// Ticket ID to use as the subtree root (supports partial matching).
        /// If omitted, shows all root tickets.
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: Option<String>,

        /// Limit display depth (0 = root only, 1 = one level, …).
        #[arg(short = 'L', long)]
        max_depth: Option<usize>,

        /// Include closed tickets (default: open and in_progress only).
        #[arg(long)]
        all: bool,
    },

    /// Serialize tickets to JSON (with optional jq filter).
    Query {
        /// Optional jq filter expression (e.g. '.status == "open"').
        filter: Option<String>,
    },

    // ── Phase 6: plugin & advanced ──────────────────────────────────
    /// Display the resolved configuration and the source of each value.
    #[command(name = "show-config")]
    ShowConfig,

    /// Open a ticket in $EDITOR.
    Edit {
        /// Ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,
    },

    /// Bypass plugin discovery and call a built-in command directly.
    #[command(trailing_var_arg = true)]
    Super {
        /// Built-in command name and its arguments.
        #[arg(required = true, num_args = 1..)]
        args: Vec<std::ffi::OsString>,
    },

    /// An unknown subcommand — dispatched to an external plugin if one is
    /// found on `PATH`.
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

/// Subcommands for `dep`.
#[derive(Debug, Subcommand)]
pub enum DepCommands {
    /// Add a dependency between two tickets.
    Add {
        /// Source ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,
        /// Dependency ticket ID to add (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        dep_id: String,
    },

    /// Remove a dependency from a ticket.
    Remove {
        /// Source ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,
        /// Dependency ticket ID to remove (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        dep_id: String,
    },

    /// Display the dependency tree for a ticket.
    Tree {
        /// Show all occurrences of each ticket (disable deduplication).
        #[arg(long)]
        full: bool,
        /// Root ticket ID (supports partial matching).
        #[arg(add = ArgValueCompleter::new(TicketIdCompleter))]
        id: String,
    },

    /// Detect dependency cycles among open and in-progress tickets.
    Cycle,
}
