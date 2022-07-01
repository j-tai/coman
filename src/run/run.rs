use std::process::Command;

use anyhow::{Context, Result};

use crate::Program;

use super::{eval_command_template, RunResult};

/// Create a `Command` that can be used to run the
/// program. Assumes that the program has already been compiled.
pub fn get_run_command(prog: &Program) -> Command {
    if let Some(lang) = prog.language() {
        let run = &lang.run;
        if !run.is_empty() {
            return eval_command_template(prog, run, false);
        }
    }
    Command::new(prog.build_path(false))
}

/// Run the program in release mode. Returns true if the program
/// exited with success, otherwise returns false. The program's
/// stdin, stdout, and stderr are all inherited.
pub fn run(prog: &Program, args: &[&str]) -> Result<RunResult> {
    let mut cmd = get_run_command(prog);
    cmd.args(args);
    let stat = cmd
        .status()
        .with_context(|| format!("failed to run command {:?}", cmd))?;
    Ok(stat.into())
}
