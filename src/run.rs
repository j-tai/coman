use std::fmt;
use std::process::{Command, ExitStatus};

use crate::Program;

pub use build::*;
pub use clean::*;
pub use cmake::*;
pub use debug::*;
pub use init::*;
pub use run::*;
pub use test::*;

mod build;
mod clean;
mod cmake;
mod debug;
mod init;
mod run;
mod test;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunResult {
    Success,
    ExitCode(i32),
    Signal(i32),
    Unknown,
}

impl RunResult {
    pub fn is_success(&self) -> bool {
        self == &RunResult::Success
    }

    pub fn as_code(&self) -> i32 {
        match self {
            RunResult::Success => 0,
            RunResult::ExitCode(code) => *code,
            RunResult::Signal(sig) => sig | 0x80,
            RunResult::Unknown => 255,
        }
    }
}

impl From<ExitStatus> for RunResult {
    fn from(status: ExitStatus) -> Self {
        if status.success() {
            RunResult::Success
        } else if let Some(code) = status.code() {
            RunResult::ExitCode(code)
        } else {
            // Grab the Unix signal, if possible
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                if let Some(sig) = status.signal() {
                    return RunResult::Signal(sig);
                }
            }
            RunResult::Unknown
        }
    }
}

impl fmt::Display for RunResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunResult::Success => write!(f, "success"),
            RunResult::ExitCode(code) => write!(f, "exit code {}", code),
            // Signals listed here must be commonly seen in a contest
            // programming environment and be portable among POSIX-compliant
            // systems.
            RunResult::Signal(6) => write!(f, "signal ABRT (Aborted)"),
            RunResult::Signal(8) => write!(f, "signal FPE (Floating point exception)"),
            RunResult::Signal(9) => write!(f, "signal KILL (Killed)"),
            RunResult::Signal(11) => write!(f, "signal SEGV (Segmentation fault)"),
            RunResult::Signal(sig) => write!(f, "signal {}", sig),
            RunResult::Unknown => write!(f, "unknown exit status"),
        }
    }
}

fn eval_command_template(prog: &Program, temp: &[String], debug: bool) -> Command {
    let mut c = Command::new(&temp[0]);
    for arg in &temp[1..] {
        match arg.as_str() {
            "{source}" => c.arg(prog.source_path()),
            "{build}" => c.arg(prog.build_path(debug)),
            "{root}" => c.arg(prog.repository().root()),
            a => c.arg(a),
        };
    }
    c
}
