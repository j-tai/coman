use std::process::Command;

use anyhow::{bail, Result};

use crate::command::eval_command_template;
use crate::Program;

use super::RunResult;

/// Create a `Command` that can be used to run the program in a
/// debugger specified in the configuration. Assumes that the
/// program has already been compiled.
pub fn get_debug_command(prog: &Program) -> Result<Command> {
    let ext = prog.source_extension();
    if let Some(lang) = prog.language() {
        let debug = &lang.debug;
        if !debug.is_empty() {
            return Ok(eval_command_template(prog, debug, true));
        }
    }
    bail!("no debugger specified for file extension {:?}", ext);
}

/// Debug the program. The specified debugging program in the
/// configuration is called. This usually means that the user is
/// put into an interactive debugger like GDB. Returns true if the
/// debugger exited with success, or false otherwise. This assumes
/// that the program has already been compiled in debug mode.
pub fn debug(prog: &Program) -> Result<RunResult> {
    let mut cmd = get_debug_command(prog)?;
    let stat = cmd.status()?;
    Ok(stat.into())
}
