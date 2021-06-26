use std::process::Command;

use crate::Program;

pub use build::*;
pub use clean::*;
pub use cmake::*;
pub use debug::*;
pub use run::*;
pub use test::*;

mod build;
mod clean;
mod cmake;
mod debug;
mod run;
mod test;

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
