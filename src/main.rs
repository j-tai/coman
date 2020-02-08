use std::env;
use std::process;

use getargs::Options;

use crate::args::Subcommand;
pub use crate::config::*;
pub use crate::repo::*;

mod args;
pub mod config;
pub mod repo;

macro_rules! step {
    ($name:expr , $( $arg:tt )+) => {{
        eprint!("\x1b[1m{:>8}\x1b[m ", $name );
        eprint!( $( $arg)* );
    }};
    ($name:expr) => {{
        eprint!("\x1b[1m{:>8}\x1b[m", $name );
    }};
    ($name:expr ,) => { step!($name) };
}

macro_rules! stepln {
    ($name:expr , $( $arg:tt )+) => {{
        eprint!("\x1b[1m{:>8}\x1b[m ", $name );
        eprintln!( $( $arg)* );
    }};
    ($name:expr) => {{
        eprintln!("\x1b[1m{:>8}\x1b[m", $name );
    }};
    ($name:expr ,) => { step!($name) };
}

fn get_program<'a>(repo: &'a Repository, program: Option<&str>) -> Program<'a> {
    if let Some(name) = program {
        if let Some(prgm) = repo.get_program(name) {
            prgm
        } else {
            eprintln!("coman: {}: not found or outside repository", name);
            process::exit(2);
        }
    } else {
        if let Some(prgm) = repo.find_recent_program() {
            prgm
        } else {
            eprintln!("coman: no solutions found");
            process::exit(2);
        }
    }
}

fn do_build(program: &Program, debug: bool) -> i32 {
    stepln!("COMPILE", "{}", program.name());
    match program.compile(debug) {
        Ok(true) => 0,
        Ok(false) => 2,
        Err(e) => {
            eprintln!("coman: compilation failed: {}", e);
            3
        }
    }
}

fn do_test(prgm: &Program, case: &str) -> bool {
    step!("TEST", "{}: ", case);
    let result = prgm.test(case);
    match result {
        Ok(result) => {
            match result.status {
                TestStatus::Pass => eprint!("\x1b[1;32mpass\x1b[m "),
                TestStatus::Wrong => eprint!("\x1b[1;31mwrong\x1b[m "),
                TestStatus::Crash => eprint!("\x1b[1;31mcrash\x1b[m "),
                TestStatus::Timeout => eprint!("\x1b[1;33mtimeout\x1b[m "),
                TestStatus::PassTimeout => eprint!("\x1b[1;32mpass\x1b[m-\x1b[1;33mtimeout\x1b[m "),
                TestStatus::WrongTimeout => {
                    eprint!("\x1b[1;31mwrong\x1b[m-\x1b[1;33mtimeout\x1b[m ")
                }
                TestStatus::CrashTimeout => {
                    eprint!("\x1b[1;31mcrash\x1b[m-\x1b[1;33mtimeout\x1b[m ")
                }
            }
            let seconds = result.time.as_secs();
            let millis = result.time.subsec_millis();
            let micros = result.time.subsec_micros() % 1000;
            if seconds >= 100 {
                eprintln!("{} s", seconds);
            } else if seconds >= 10 {
                eprintln!("{}.{} s", seconds, millis / 100);
            } else if seconds >= 1 {
                eprintln!("{}.{:02} s", seconds, millis / 10);
            } else if millis >= 100 {
                eprintln!("{} ms", millis);
            } else if millis >= 10 {
                eprintln!("{}.{} ms", millis, micros / 100);
            } else if millis >= 1 {
                eprintln!("{}.{:02} ms", millis, micros / 10);
            } else {
                eprintln!("0.{:03} ms", micros);
            }
            result.status == TestStatus::Pass
        }
        Err(e) => {
            eprintln!("{}", e);
            false
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
    build|b [SOLUTION ...]
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

    let root = match find_root_dir() {
        Some(d) => d,
        None => {
            eprintln!("coman: cannot find repository root; make sure you have a Coman.toml");
            process::exit(3);
        }
    };
    let repo = match Repository::read(root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("coman: cannot open repository: {}", e);
            process::exit(3);
        }
    };

    let mut exit_code = 0;

    match args.subcommand {
        Subcommand::Build { programs } => {
            if programs.is_empty() {
                let program = get_program(&repo, None);
                exit_code = exit_code.max(do_build(&program, false));
            } else {
                for program in programs {
                    let program = get_program(&repo, Some(program));
                    exit_code = exit_code.max(do_build(&program, false));
                }
            }
        }

        Subcommand::Run { program } => {
            let program = get_program(&repo, program);
            exit_code = do_build(&program, false);
            if exit_code != 0 {
                process::exit(exit_code);
            }

            stepln!("RUN", "{}", program.name());
            exit_code = match program.run() {
                Ok(true) => 0,
                Ok(false) => 1,
                Err(e) => {
                    eprintln!("coman: running program failed: {}", e);
                    2
                }
            };
        }

        Subcommand::Test { program, tests } => {
            let program = get_program(&repo, program);
            exit_code = do_build(&program, false);
            if exit_code != 0 {
                process::exit(exit_code);
            }

            if tests.is_empty() {
                // Testing all cases
                let mut cases = program.test_cases();
                cases.sort_unstable();
                for case in &cases {
                    if !do_test(&program, case) {
                        exit_code = 1;
                    }
                }
            } else {
                for case in tests {
                    if !do_test(&program, case) {
                        exit_code = 1;
                    }
                }
            }
        }

        Subcommand::Debug { program } => {
            let program = get_program(&repo, program);
            exit_code = do_build(&program, true);
            if exit_code != 0 {
                process::exit(exit_code);
            }

            stepln!("DEBUG", "{}", program.name());
            exit_code = match program.debug() {
                Ok(true) => 0,
                Ok(false) => 1,
                Err(e) => {
                    eprintln!("coman: debugging program failed: {}", e);
                    2
                }
            }
        }

        Subcommand::Clean { program, all } => {
            if all {
                stepln!("CLEAN", "all binaries");
                exit_code = match repo.clean_all() {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("coman: cleaning all binaries failed: {}", e);
                        2
                    }
                }
            } else {
                let program = get_program(&repo, program);
                stepln!("CLEAN", "{}", program.name());
                exit_code = match program.clean() {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("coman: cleaning program failed: {}", e);
                        2
                    }
                }
            }
        }

        Subcommand::CMake => {
            stepln!("GENERATE", "CMakeLists.txt");
            let result = repo.write_cmake();
            if let Err(e) = result {
                eprintln!("coman: cannot create CMakeLists.txt: {}", e);
                exit_code = 2;
            }
        }
    }

    process::exit(exit_code);
}
