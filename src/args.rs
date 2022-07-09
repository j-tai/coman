use getargs::{Opt, Options};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UsageError<'a> {
    #[error("--help was specified")]
    Help,
    #[error("--version was specified")]
    Version,
    #[error("{0}")]
    Getargs(getargs::Error<&'a str>),
    #[error("unknown option {0}")]
    UnknownOpt(Opt<&'a str>),
    #[error("unknown subcommand {0:?}")]
    UnknownSubcommand(&'a str),
}

impl<'a> From<getargs::Error<&'a str>> for UsageError<'a> {
    fn from(e: getargs::Error<&'a str>) -> Self {
        UsageError::Getargs(e)
    }
}

#[derive(Clone, Debug)]
pub struct Arguments<'a> {
    pub subcommand: Subcommand<'a>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Subcommand<'a> {
    Init,
    Build {
        programs: Vec<&'a str>,
        debug: bool,
        output: Option<&'a str>,
    },
    Run {
        program: Option<&'a str>,
        args: Vec<&'a str>,
    },
    Test {
        program: Option<&'a str>,
        tests: Vec<&'a str>,
    },
    Debug {
        program: Option<&'a str>,
    },
    Clean {
        program: Option<&'a str>,
        all: bool,
    },
    CMake,
}

pub fn parse_args<'a, I: Iterator<Item = &'a str>>(
    opts: &mut Options<&'a str, I>,
) -> Result<Arguments<'a>, UsageError<'a>> {
    while let Some(opt) = opts.next_opt()? {
        match opt {
            Opt::Short('h') | Opt::Long("help") => return Err(UsageError::Help),
            Opt::Long("version") => return Err(UsageError::Version),
            _ => return Err(UsageError::UnknownOpt(opt)),
        }
    }
    let subcommand_name = opts.next_positional().unwrap_or("r");
    let subcommand = match subcommand_name {
        "init" => Subcommand::Init,
        "build" | "b" => parse_build_args(opts)?,
        "clean" | "c" => parse_clean_args(opts)?,
        "debug" | "d" => parse_debug_args(opts)?,
        "run" | "r" => parse_run_args(opts)?,
        "test" | "t" => parse_test_args(opts)?,
        "cmake" => Subcommand::CMake,
        _ => return Err(UsageError::UnknownSubcommand(subcommand_name)),
    };
    Ok(Arguments { subcommand })
}

fn parse_build_args<'a, I: Iterator<Item = &'a str>>(
    opts: &mut Options<&'a str, I>,
) -> Result<Subcommand<'a>, UsageError<'a>> {
    let mut debug = false;
    let mut output = None;
    while let Some(opt) = opts.next_opt()? {
        match opt {
            Opt::Short('d') | Opt::Long("debug") => debug = true,
            Opt::Short('o') | Opt::Long("output") => output = Some(opts.value()?),
            _ => return Err(UsageError::UnknownOpt(opt)),
        }
    }
    Ok(Subcommand::Build {
        programs: opts.positionals().collect(),
        debug,
        output,
    })
}

fn parse_clean_args<'a, I: Iterator<Item = &'a str>>(
    opts: &mut Options<&'a str, I>,
) -> Result<Subcommand<'a>, UsageError<'a>> {
    let mut all = false;
    while let Some(opt) = opts.next_opt()? {
        match opt {
            Opt::Long("all") => all = true,
            _ => return Err(UsageError::UnknownOpt(opt)),
        }
    }
    let program = opts.next_positional();
    Ok(Subcommand::Clean { all, program })
}

fn parse_debug_args<'a, I: Iterator<Item = &'a str>>(
    opts: &mut Options<&'a str, I>,
) -> Result<Subcommand<'a>, UsageError<'a>> {
    Ok(Subcommand::Debug {
        program: opts.next_positional(),
    })
}

fn parse_run_args<'a, I: Iterator<Item = &'a str>>(
    opts: &mut Options<&'a str, I>,
) -> Result<Subcommand<'a>, UsageError<'a>> {
    let program = opts.next_positional();
    let args = opts.positionals().collect();
    Ok(Subcommand::Run { program, args })
}

fn parse_test_args<'a, I: Iterator<Item = &'a str>>(
    opts: &mut Options<&'a str, I>,
) -> Result<Subcommand<'a>, UsageError<'a>> {
    Ok(Subcommand::Test {
        program: opts.next_positional().map(|s| &s[..]),
        tests: opts.positionals().collect(),
    })
}
