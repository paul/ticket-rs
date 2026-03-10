use clap::CommandFactory;
use clap::Parser;
use console::style;
use std::process;

use ticket_rs::cli::{Cli, ColorWhen, Commands, DepCommands};
use ticket_rs::commands;
use ticket_rs::error::Error;
use ticket_rs::input::{resolve_input, validate_no_multiple_stdin};
use ticket_rs::pager;
use ticket_rs::plugin;
use ticket_rs::ticket::Status;

fn main() {
    // Intercept bare `ticket help` before clap gets a chance to handle it so
    // we can append a "Plugins" section listing discovered external commands.
    let raw_args: Vec<String> = std::env::args().collect();
    if raw_args.get(1).map(|s| s.as_str()) == Some("help") && raw_args.len() == 2 {
        print_help_with_plugins();
        process::exit(0);
    }

    let cli = Cli::parse();

    // Apply the --color flag globally before any output is produced.
    match cli.color {
        ColorWhen::Always => console::set_colors_enabled(true),
        ColorWhen::Never => console::set_colors_enabled(false),
        ColorWhen::Auto => {} // defer to console's TTY + NO_COLOR/CLICOLOR detection
    }

    if cli.no_pager {
        pager::set_pager_disabled(true);
    }

    let result = dispatch(cli.command);

    if let Err(err) = result {
        match &err {
            Error::TicketNotFound { id, suggestions } if !suggestions.is_empty() => {
                eprintln!("{}: ticket '{id}' not found", style("Error").red().bold());
                eprintln!();
                eprintln!("  did you mean?");
                eprintln!();
                for t in suggestions {
                    let colored_status = match t.status {
                        Status::Open => style("open").green().to_string(),
                        Status::InProgress => style("in_progress").yellow().to_string(),
                        Status::Closed => style("closed").dim().to_string(),
                    };
                    eprintln!("    {:<12}  [{}] - {}", t.id, colored_status, t.title);
                }
                eprintln!();
            }
            _ => eprintln!("{}: {err}", style("Error").red().bold()),
        }
        process::exit(1);
    }
}

/// Print clap's standard help output followed by a "Plugins" section that
/// lists all discovered external plugins whose names do not shadow a built-in.
fn print_help_with_plugins() {
    // Render clap's built-in long help (same as --help).
    let mut cmd = Cli::command();
    let help_text = cmd.render_long_help();
    print!("{help_text}");

    // Derive built-in names (and aliases) directly from the clap command tree
    // so this set stays in sync automatically as commands are added or renamed.
    let builtins: std::collections::HashSet<String> = Cli::command()
        .get_subcommands()
        .flat_map(|sub| {
            std::iter::once(sub.get_name().to_string()).chain(
                sub.get_all_aliases()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    // Discover plugins, excluding any that share a name with a built-in.
    let plugins: Vec<_> = plugin::discover_plugins()
        .into_iter()
        .filter(|p| !builtins.contains(&p.name))
        .collect();

    if plugins.is_empty() {
        return;
    }

    // Align descriptions to the same column as clap's command list.
    let max_name_len = plugins.iter().map(|p| p.name.len()).max().unwrap_or(0);
    let col_width = max_name_len.max(6); // at least 6 chars wide

    println!("\nPlugins:");
    for p in &plugins {
        let desc = p.description.as_deref().unwrap_or("(no description)");
        println!("  {:<width$}  {desc}", p.name, width = col_width);
    }
}

fn dispatch(command: Commands) -> ticket_rs::error::Result<()> {
    match command {
        Commands::Create {
            title,
            title_flag,
            description,
            design,
            acceptance,
            ticket_type,
            priority,
            assignee,
            external_ref,
            parent,
            tags,
        } => {
            validate_no_multiple_stdin(&[
                description.as_deref(),
                design.as_deref(),
                acceptance.as_deref(),
            ])?;
            let description = description.map(|v| resolve_input(&v)).transpose()?;
            let design = design.map(|v| resolve_input(&v)).transpose()?;
            let acceptance = acceptance.map(|v| resolve_input(&v)).transpose()?;
            commands::create(
                title_flag
                    .as_deref()
                    .or(title.as_deref())
                    .unwrap_or("Untitled"),
                description.as_deref(),
                design.as_deref(),
                acceptance.as_deref(),
                &ticket_type,
                &priority,
                assignee.as_deref(),
                external_ref.as_deref(),
                parent.as_deref(),
                tags.as_deref(),
            )
        }

        Commands::Show { id, extra } => {
            for arg in &extra {
                eprintln!(
                    "warning: unknown argument '{}' ignored",
                    arg.to_string_lossy()
                );
            }
            commands::show(&id)
        }
        Commands::Start { id } => commands::start(&id),
        Commands::Close { id } => commands::close(&id),
        Commands::Reopen { id } => commands::reopen(&id),
        Commands::Status { id, status } => commands::status(&id, &status),

        Commands::Dep { command } => match command {
            DepCommands::Add { id, dep_id } => commands::dep(&id, &dep_id),
            DepCommands::Remove { id, dep_id } => commands::dep_remove(&id, &dep_id),
            DepCommands::Tree { id, full } => commands::dep_tree(&id, full),
            DepCommands::Cycle => commands::dep_cycle(),
        },

        Commands::Link { ids } => commands::link(&ids),
        Commands::Unlink { id, target_id } => commands::unlink(&id, &target_id),

        Commands::Ls {
            status,
            assignee,
            tag,
        } => commands::ls(status.as_deref(), assignee.as_deref(), tag.as_deref()),

        Commands::Ready { assignee, tag } => commands::ready(assignee.as_deref(), tag.as_deref()),

        Commands::Blocked { assignee, tag } => {
            commands::blocked(assignee.as_deref(), tag.as_deref())
        }

        Commands::Closed {
            limit,
            assignee,
            tag,
        } => commands::closed(limit, assignee.as_deref(), tag.as_deref()),

        Commands::AddNote { id, text } => {
            let text = text.map(|v| resolve_input(&v)).transpose()?;
            commands::add_note(&id, text.as_deref())
        }

        Commands::Edit { id } => commands::edit(&id),

        Commands::Update {
            id,
            title,
            description,
            design,
            acceptance,
            priority,
            ticket_type,
            assignee,
            external_ref,
            parent,
            tags,
            add_tags,
            remove_tags,
        } => {
            validate_no_multiple_stdin(&[
                description.as_deref(),
                design.as_deref(),
                acceptance.as_deref(),
            ])?;
            let description = description.map(|v| resolve_input(&v)).transpose()?;
            let design = design.map(|v| resolve_input(&v)).transpose()?;
            let acceptance = acceptance.map(|v| resolve_input(&v)).transpose()?;
            commands::update(
                &id,
                title.as_deref(),
                description.as_deref(),
                design.as_deref(),
                acceptance.as_deref(),
                priority.as_deref(),
                ticket_type.as_deref(),
                assignee.as_deref(),
                external_ref.as_deref(),
                parent.as_deref(),
                tags.as_deref(),
                add_tags.as_deref(),
                remove_tags.as_deref(),
            )
        }

        Commands::Query { filter } => commands::query(filter.as_deref()),

        Commands::Tree { id, max_depth, all } => commands::tree(id.as_deref(), max_depth, all),

        Commands::Super { args } => {
            // Re-parse the trailing args as a top-level command, bypassing any
            // future plugin lookup. Prepend the binary name so clap sees a full
            // argv.
            let mut full_args = vec![std::ffi::OsString::from("ticket")];
            full_args.extend(args);
            let inner = Cli::try_parse_from(full_args).unwrap_or_else(|e| e.exit());
            dispatch(inner.command)?;
            Ok(())
        }

        Commands::External(args) => {
            // Extract the subcommand name (first element) and remaining args.
            let cmd = args[0].to_string_lossy();
            match plugin::find_plugin(&cmd) {
                Some(path) => {
                    plugin::exec_plugin(&path, &args[1..]);
                    Ok(()) // unreachable: exec_plugin exits the process
                }
                None => {
                    eprintln!(
                        "{}: unknown command '{cmd}'",
                        console::style("Error").red().bold()
                    );
                    process::exit(1);
                }
            }
        }
    }
}
