use std::process;

use clap::{App, Arg};

pub use crate::config::*;
pub use crate::repo::*;

pub mod config;
pub mod repo;

#[derive(Clone, Debug)]
struct Arguments {
    action: Action,
    test: Option<String>,
    program: Option<String>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Action {
    Build,
    Run,
    Test,
    Debug,
    Clean,
}

fn parse_args() -> Arguments {
    let matches = App::new("coman")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Contest manager")
        .arg(
            Arg::with_name("build")
                .short("B")
                .long("build")
                .help("Build the solution"),
        )
        .arg(
            Arg::with_name("run")
                .short("R")
                .long("run")
                .help("Run the solution (default)"),
        )
        .arg(
            Arg::with_name("test")
                .short("T")
                .long("test")
                .help("Test the solution"),
        )
        .arg(
            Arg::with_name("debug")
                .short("D")
                .long("debug")
                .help("Debug the solution"),
        )
        .arg(
            Arg::with_name("clean")
                .short("C")
                .long("clean")
                .help("Clean built binaries")
        )
        .arg(
            Arg::with_name("test-name")
                .short("t")
                .long("test-name")
                .takes_value(true)
                .value_name("TEST")
                .help("Name of test to run"),
        )
        .arg(
            Arg::with_name("PROGRAM")
                .index(1)
                .help("Path to solution source file"),
        )
        .get_matches();

    let build = matches.is_present("build");
    let run = matches.is_present("run");
    let test = matches.is_present("test");
    let debug = matches.is_present("debug");
    let clean = matches.is_present("clean");
    let action = match (build, run, test, debug, clean) {
        (false, false, false, false, false) => Action::Run,
        (true, false, false, false, false) => Action::Build,
        (false, true, false, false, false) => Action::Run,
        (false, false, true, false, false) => Action::Test,
        (false, false, false, true, false) => Action::Debug,
        (false, false, false, false, true) => Action::Clean,
        _ => {
            eprintln!("coman: only one of -B, -R, -T, -D, and -C may be used at a time");
            process::exit(2);
        }
    };

    let test = matches.value_of("test-name").map(str::to_string);
    let program = matches.value_of("PROGRAM").map(str::to_string);

    Arguments {
        action,
        test,
        program,
    }
}

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
    let args = parse_args();
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

    let program = if let Some(ref prog) = args.program {
        if let Some(p) = repo.get_program(prog) {
            p
        } else {
            eprintln!("coman: {}: not found or outside repository", prog);
            process::exit(2);
        }
    } else if let Some(p) = repo.find_recent_program() {
        p
    } else {
        eprintln!("coman: cannot find any program to run");
        process::exit(2);
    };

    // Compiling the program
    if args.action != Action::Clean {
        stepln!("COMPILE", "{}", program.name());
        match program.compile(args.action == Action::Debug) {
            Ok(true) => (),
            Ok(false) => process::exit(2),
            Err(e) => {
                eprintln!("coman: compilation failed: {}", e);
                process::exit(3);
            }
        }
    }

    match args.action {
        Action::Test => {
            let mut all_ok = true;
            if let Some(ref case) = args.test {
                all_ok = do_test(&program, case);
            } else {
                // Testing all cases
                let mut cases = program.test_cases();
                cases.sort_unstable();
                for case in &cases {
                    all_ok = do_test(&program, case) && all_ok;
                }
            }
            if !all_ok {
                process::exit(1);
            }
        }

        Action::Run => {
            stepln!("RUN", "{}", program.name());
            match program.run() {
                Ok(true) => (),
                Ok(false) => process::exit(1),
                Err(e) => {
                    eprintln!("coman: running program failed: {}", e);
                    process::exit(2);
                }
            }
        }

        Action::Debug => {
            // Debugging the program
            stepln!("DEBUG", "{}", program.name());
            match program.debug() {
                Ok(true) => (),
                Ok(false) => process::exit(1),
                Err(e) => {
                    eprintln!("coman: debugging program failed: {}", e);
                    process::exit(2);
                }
            }
        }

        Action::Build => (), // Building is done above, so we have nothing else to do

        Action::Clean => {
            stepln!("CLEAN", "{}", program.name());
            match program.clean() {
                Ok(()) => (),
                Err(e) => {
                    eprintln!("coman: cleaning program failed: {}", e);
                    process::exit(2);
                }
            }
        }
    }
}
