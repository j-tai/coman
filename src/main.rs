use std::env;
use std::fs;
use std::path::Path;
use std::process;

use anyhow::{bail, Context, Result};
use args::Arguments;
use getargs::Options;

use crate::args::Subcommand;
pub use crate::config::*;
pub use crate::repo::*;
use crate::run::TestStatus;

mod args;
pub mod config;
pub mod repo;
pub mod run;
mod ui;

fn get_program<'a>(repo: &'a Repository, program: Option<&str>) -> Result<Program<'a>> {
    if let Some(name) = program {
        repo.get_program(name)
    } else {
        repo.find_recent_program()
    }
}

fn do_build(program: &Program, debug: bool, output: Option<&str>) -> Result<()> {
    stepln!("COMPILE", "{}", program.name());
    run::compile(program, debug).context("compilation failed")?;

    if let Some(output) = output {
        let from = if debug {
            program.build_debug_path()
        } else {
            program.build_release_path()
        };
        let to = Path::new(output);
        let parent = to.parent().unwrap();

        fs::create_dir_all(parent).with_context(|| format!("failed to create dir {:?}", parent))?;
        fs::copy(from, to).with_context(|| format!("failed to copy {:?} to {:?}", from, to))?;
    }

    Ok(())
}

fn do_test(prog: &Program, case: &str) -> Result<bool> {
    step!("TEST", "{}: ", case);
    let result = run::test(prog, case)
        .with_context(|| format!("failed to run test case {:?} on program {}", case, prog))?;

    ui::print_test_result(&result);

    let passed = result.status == TestStatus::Pass && !result.timeout;
    if !passed {
        ui::print_n_lines("captured stderr", &result.stderr, 12);
    }
    Ok(passed)
}

fn try_main(args: Arguments) -> Result<bool> {
    // init is the only command that doesn't require an existing repository
    if args.subcommand == Subcommand::Init {
        stepln!("INIT", "coman repository");
        run::init()?;
        return Ok(true);
    }

    // For all other commands, load the repository:
    let root = find_root_dir()?;
    let repo = Repository::read(root)?;

    match args.subcommand {
        Subcommand::Init => unreachable!(),

        Subcommand::Build {
            programs,
            debug,
            output,
        } => {
            if programs.is_empty() {
                let prog = get_program(&repo, None)?;
                do_build(&prog, debug, output)?;
            } else {
                for prog in programs {
                    let program = get_program(&repo, Some(prog))?;
                    do_build(&program, debug, output)?;
                }
            }
            Ok(true)
        }

        Subcommand::Run { program } => {
            let prog = get_program(&repo, program)?;
            do_build(&prog, false, None)?;

            stepln!("RUN", "{}", prog.name());
            run::run(&prog).with_context(|| format!("failed to run program {}", prog))
        }

        Subcommand::Test { program, tests } => {
            let program = get_program(&repo, program)?;
            do_build(&program, false, None)?;

            let mut result = true;
            if tests.is_empty() {
                // Testing all cases
                let mut cases = run::get_test_cases(&program)?;
                if cases.is_empty() {
                    // No cases found
                    bail!("no test cases found in {:?}", program.test_path());
                } else {
                    alphanumeric_sort::sort_str_slice(&mut cases);
                    for case in &cases {
                        if !do_test(&program, case)? {
                            result = false;
                        }
                    }
                }
            } else {
                for case in tests {
                    if !do_test(&program, case)? {
                        result = false;
                    }
                }
            }
            Ok(result)
        }

        Subcommand::Debug { program } => {
            let program = get_program(&repo, program)?;
            do_build(&program, true, None)?;

            stepln!("DEBUG", "{}", program.name());
            run::debug(&program).with_context(|| format!("failed to debug program {}", program))
        }

        Subcommand::Clean { program, all } => {
            if all {
                stepln!("CLEAN", "all binaries");
                run::clean_all(&repo).context("failed to clean all binaries")?;
            } else {
                let program = get_program(&repo, program)?;
                stepln!("CLEAN", "{}", program.name());
                run::clean(&program)
                    .with_context(|| format!("failed to clean binary for {}", program))?;
            }
            Ok(true)
        }

        Subcommand::CMake => {
            stepln!("GENERATE", "CMakeLists.txt");
            run::write_cmake(&repo)
                .with_context(|| format!("failed to generate CMakeLists.txt"))?;
            Ok(true)
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let options = Options::new(&args);
    let args = match args::parse_args(&options) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("coman: usage error: {}", e);
            process::exit(3);
        }
    };

    if args.bad_usage || args.show_help {
        print!(
            "coman - Contest manager

Usage: coman [OPTIONS] COMMAND

Options:
    --version  Print version and exit

Commands:
    init
    build|b [-d] [-o OUTPUT] [SOLUTION ...]
    clean|c [SOLUTION | --all]
    debug|d [SOLUTION]
    run|r [SOLUTION]
    test|t [SOLUTION] [TEST ...]
    cmake
"
        );
        if args.bad_usage {
            process::exit(3);
        } else {
            return;
        }
    }

    if args.show_version {
        println!("coman v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let result = try_main(args);

    match result {
        Ok(true) => return,
        Ok(false) => process::exit(1),
        Err(e) => {
            eprintln!("coman: {:?}", e);
            process::exit(2);
        }
    }
}
