use getargs::{Error, Opt, Options, Result};

#[derive(Clone, Debug, Default)]
pub struct Arguments<'a> {
    pub bad_usage: bool,
    pub show_help: bool,
    pub show_version: bool,
    pub subcommand: Subcommand<'a>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Subcommand<'a> {
    Build {
        program: Option<&'a str>,
    },
    Run {
        program: Option<&'a str>,
    },
    Test {
        program: Option<&'a str>,
        test: Option<&'a str>,
    },
    Debug {
        program: Option<&'a str>,
    },
    Clean {
        program: Option<&'a str>,
        all: bool,
    },
}

impl Default for Subcommand<'_> {
    fn default() -> Self {
        Subcommand::Build { program: None }
    }
}

pub fn parse_args<'a>(opts: &'a Options<'a, String>) -> Result<Arguments<'a>> {
    let mut res = Arguments::default();
    while let Some(opt) = opts.next() {
        match opt? {
            Opt::Long("help") => res.show_help = true,
            Opt::Long("version") => res.show_version = true,
            o => return Err(Error::UnknownOpt(o)),
        }
    }
    let subcommand = opts.arg_str().map(|s| &s[..]).unwrap_or("r");
    res.subcommand = match subcommand {
        "build" | "b" => parse_build_args(opts)?,
        "clean" | "c" => parse_clean_args(opts)?,
        "debug" | "d" => parse_debug_args(opts)?,
        "run" | "r" => parse_run_args(opts)?,
        "test" | "t" => parse_test_args(opts)?,
        _ => {
            res.bad_usage = true;
            return Ok(res);
        }
    };
    Ok(res)
}

fn parse_build_args<'a>(opts: &'a Options<'a, String>) -> Result<Subcommand<'a>> {
    Ok(Subcommand::Build {
        program: opts.arg_str().map(|s| &s[..]),
    })
}

fn parse_clean_args<'a>(opts: &'a Options<'a, String>) -> Result<Subcommand<'a>> {
    let mut all = false;
    while let Some(opt) = opts.next() {
        match opt? {
            Opt::Long("all") => all = true,
            o => return Err(Error::UnknownOpt(o)),
        }
    }
    let program = opts.arg_str().map(|s| &s[..]);
    Ok(Subcommand::Clean { all, program })
}

fn parse_debug_args<'a>(opts: &'a Options<'a, String>) -> Result<Subcommand<'a>> {
    Ok(Subcommand::Debug {
        program: opts.arg_str().map(|s| &s[..]),
    })
}

fn parse_run_args<'a>(opts: &'a Options<'a, String>) -> Result<Subcommand<'a>> {
    Ok(Subcommand::Run {
        program: opts.arg_str().map(|s| &s[..]),
    })
}

fn parse_test_args<'a>(opts: &'a Options<'a, String>) -> Result<Subcommand<'a>> {
    Ok(Subcommand::Test {
        program: opts.arg_str().map(|s| &s[..]),
        test: opts.arg_str().map(|s| &s[..]),
    })
}
