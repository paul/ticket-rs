use clap::Parser;
use console::style;
use std::process;

use ticket_rs::cli::{Cli, ColorWhen, Commands};
use ticket_rs::commands;

fn main() {
    let cli = Cli::parse();

    // Apply the --color flag globally before any output is produced.
    match cli.color {
        ColorWhen::Always => console::set_colors_enabled(true),
        ColorWhen::Never => console::set_colors_enabled(false),
        ColorWhen::Auto => {} // defer to console's TTY + NO_COLOR/CLICOLOR detection
    }

    let result = dispatch(cli.command);

    if let Err(err) = result {
        eprintln!("{}: {err}", style("error").red().bold());
        process::exit(1);
    }
}

fn dispatch(command: Commands) -> ticket_rs::error::Result<()> {
    match command {
        Commands::Create {
            title,
            description,
            design,
            acceptance,
            ticket_type,
            priority,
            assignee,
            external_ref,
            parent,
            tags,
        } => commands::create(
            title.as_deref().unwrap_or("Untitled"),
            description.as_deref(),
            design.as_deref(),
            acceptance.as_deref(),
            &ticket_type,
            &priority,
            assignee.as_deref(),
            external_ref.as_deref(),
            parent.as_deref(),
            tags.as_deref(),
        ),

        Commands::Show { id } => commands::show(&id),
        Commands::Start { id } => commands::start(&id),
        Commands::Close { id } => commands::close(&id),
        Commands::Reopen { id } => commands::reopen(&id),
        Commands::Status { id, status } => commands::status(&id, &status),

        Commands::Dep
        | Commands::Undep
        | Commands::Link
        | Commands::Unlink
        | Commands::Ls
        | Commands::Ready
        | Commands::Blocked
        | Commands::Closed
        | Commands::Update
        | Commands::AddNote
        | Commands::Tree
        | Commands::Query
        | Commands::Edit
        | Commands::Super => {
            eprintln!("not yet implemented");
            process::exit(1);
        }
    }
}
