// Command handler functions, one submodule per command.

use crate::error::Result;

mod create;
pub use create::create;

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
