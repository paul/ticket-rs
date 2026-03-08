// Command handler functions, one submodule per command.

use crate::error::Result;

#[allow(clippy::too_many_arguments)]
pub fn create(
    title: &str,
    description: Option<&str>,
    design: Option<&str>,
    acceptance: Option<&str>,
    ticket_type: &str,
    priority: &str,
    assignee: Option<&str>,
    external_ref: Option<&str>,
    parent: Option<&str>,
    tags: Option<&str>,
) -> Result<()> {
    let _ = (
        description,
        design,
        acceptance,
        ticket_type,
        priority,
        assignee,
        external_ref,
        parent,
        tags,
    );
    println!("create: not yet implemented (title: {title})");
    Ok(())
}

pub fn show(id: &str) -> Result<()> {
    println!("show: not yet implemented (id: {id})");
    Ok(())
}

pub fn start(id: &str) -> Result<()> {
    println!("start: not yet implemented (id: {id})");
    Ok(())
}

pub fn close(id: &str) -> Result<()> {
    println!("close: not yet implemented (id: {id})");
    Ok(())
}

pub fn reopen(id: &str) -> Result<()> {
    println!("reopen: not yet implemented (id: {id})");
    Ok(())
}

pub fn status(id: &str, status: &str) -> Result<()> {
    println!("status: not yet implemented (id: {id}, status: {status})");
    Ok(())
}
