use std::io;
use std::process::Command;

use crate::run::eval_command_template;
use crate::Program;

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
pub fn run(prog: &Program) -> io::Result<bool> {
    let mut cmd = get_run_command(prog);
    let stat = cmd.status()?;
    Ok(stat.success())
}
