use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{Error, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

use walkdir::WalkDir;
use xz2::read::XzDecoder;

use crate::Config;
use crate::Language;

/// Find the contest root directory by searching the cwd's parent
/// dirs. Returns `None` if no root directory was found or the current
/// directory cannot be fetched.
pub fn find_root_dir() -> Option<PathBuf> {
    let mut path = match env::current_dir() {
        Ok(d) => d,
        Err(_) => return None,
    };
    loop {
        path.push("Coman.toml");
        if path.exists() {
            path.pop();
            return Some(path);
        }
        path.pop();
        if !path.pop() {
            return None;
        }
    }
}

/// A struct representing a contest repository. This struct is
/// immutable. Also note that cloning this struct will simply copy a
/// reference to the same repository.
#[derive(Clone)]
pub struct Repository(Rc<RepoInner>);

struct RepoInner {
    config: Config,
    config_path: PathBuf,
    root: PathBuf,
    src: PathBuf,
    test: PathBuf,
    build: PathBuf,
    build_release: PathBuf,
    build_debug: PathBuf,
}

impl Repository {
    /// Create a new `Repository`.
    pub fn new<P: Into<PathBuf>>(root: P, config: Config) -> Repository {
        let root = root.into();
        let mut config_path = root.clone();
        config_path.push("Coman.toml");
        let mut src = root.clone();
        src.push(&config.src_dir);
        let mut test = root.clone();
        test.push(&config.test_dir);
        let mut build = root.clone();
        build.push(&config.build_dir);
        let mut build_release = build.clone();
        build_release.push("release");
        let mut build_debug = build.clone();
        build_debug.push("debug");
        Repository(Rc::new(RepoInner {
            config,
            config_path,
            root,
            src,
            test,
            build,
            build_release,
            build_debug,
        }))
    }

    /// Create a new `Repository`, reading the configuration files
    /// from the 'Coman.toml' file under the specified path.
    pub fn read<P: Into<PathBuf>>(root: P) -> io::Result<Repository> {
        let mut root = root.into();
        root.push("Coman.toml");
        let config = match File::open(&root) {
            Ok(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                toml::from_str::<Config>(&s).unwrap()
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => Config::default(),
            Err(e) => return Err(e),
        };
        root.pop();
        Ok(Repository::new(root, config))
    }

    /// Get the repository's configuration.
    pub fn config(&self) -> &Config {
        &self.0.config
    }

    /// Get the path to the repository's configuration.
    pub fn config_path(&self) -> &Path {
        &self.0.config_path
    }

    /// Get the repository's root directory path.
    pub fn root(&self) -> &Path {
        &self.0.root
    }

    /// Get the repository's source directory path.
    pub fn source_path(&self) -> &Path {
        &self.0.src
    }

    /// Get the repository's test directory path.
    pub fn test_path(&self) -> &Path {
        &self.0.test
    }

    /// Get the repository's build directory path.
    pub fn build_path(&self) -> &Path {
        &self.0.build
    }

    /// Get the repository's release build directory path.
    pub fn build_release_path(&self) -> &Path {
        &self.0.build_release
    }

    /// Get the repository's debug build directory path.
    pub fn build_debug_path(&self) -> &Path {
        &self.0.build_debug
    }

    /// Get a `Program` from the path to its source code. Returns
    /// `None` if the path is outside of the source directory or if it
    /// does not exist.
    pub fn get_program<P: AsRef<Path>>(&self, path: P) -> Option<Program> {
        let path = path.as_ref().canonicalize().ok()?;
        if !path.is_file() {
            return None;
        }
        let path = path.strip_prefix(self.source_path()).ok()?;
        let mut src = self.source_path().to_path_buf();
        src.push(&path);
        let mut test = self.test_path().to_path_buf();
        test.push(&path);
        let stem = test.file_stem().unwrap().to_os_string();
        test.set_file_name(stem);
        let mut build_release = self.build_release_path().to_path_buf();
        build_release.push(&path);
        let mut build_debug = self.build_debug_path().to_path_buf();
        build_debug.push(&path);
        Some(Program {
            repo: self.clone(),
            path: path.to_path_buf(),
            src,
            test,
            build_release,
            build_debug,
        })
    }

    /// Get the `Program` that was most recently modified. Returns
    /// `None` if no program could be found.
    pub fn find_recent_program(&self) -> Option<Program> {
        let mut best_time = SystemTime::UNIX_EPOCH;
        let mut best_prog = None;
        for ent in WalkDir::new(self.source_path()) {
            if let Ok(ent) = ent {
                if ent.file_type().is_file() {
                    if let Ok(meta) = ent.metadata() {
                        if let Ok(modified) = meta.modified() {
                            if modified > best_time {
                                best_time = modified;
                                best_prog = Some(ent.into_path());
                            }
                        }
                    }
                }
            }
        }
        self.get_program(best_prog?)
    }
}

pub struct Program {
    repo: Repository,
    path: PathBuf,
    src: PathBuf,
    test: PathBuf,
    build_release: PathBuf,
    build_debug: PathBuf,
}

impl Program {
    /// Get the name of the program.
    pub fn name(&self) -> &str {
        self.path.to_str().unwrap()
    }

    /// Get the path to the program's source file.
    pub fn source_path(&self) -> &Path {
        &self.src
    }

    /// Get the source file's extension. Returns an empty string if
    /// the file has no extension.
    pub fn source_extension(&self) -> &str {
        self.path.extension().and_then(|s| s.to_str()).unwrap_or("")
    }

    /// Get the path to the program's test directory.
    pub fn test_path(&self) -> &Path {
        &self.test
    }

    /// Get the path to the program's release build location.
    pub fn build_release_path(&self) -> &Path {
        &self.build_release
    }

    /// Get the path to the program's debug build location.
    pub fn build_debug_path(&self) -> &Path {
        &self.build_debug
    }

    /// Get the path to the program's build location.
    pub fn build_path(&self, debug: bool) -> &Path {
        if debug {
            self.build_debug_path()
        } else {
            self.build_release_path()
        }
    }

    /// Get the language that this program is written in.
    pub fn language(&self) -> Option<&Language> {
        self.repo.config().languages.get(self.source_extension())
    }

    fn command_from_template(&self, temp: &[String], debug: bool) -> Command {
        let mut c = Command::new(&temp[0]);
        for arg in &temp[1..] {
            match arg.as_str() {
                "{source}" => c.arg(self.source_path()),
                "{build}" => c.arg(self.build_path(debug)),
                "{root}" => c.arg(self.repo.root()),
                a => c.arg(a),
            };
        }
        c
    }

    /// Compile the program.
    pub fn recompile(&self, debug: bool) -> io::Result<bool> {
        let src = self.source_path();
        let dst = self.build_path(debug);
        let ext = self.source_extension();
        if let Some(lang) = self.language() {
            let cmd = if debug && !lang.compile_debug.is_empty() {
                &lang.compile_debug
            } else {
                &lang.compile
            };
            // Create destination parent directories
            fs::create_dir_all(dst.parent().unwrap())?;
            if cmd.is_empty() {
                // Copy src -> dst
                fs::copy(src, dst)?;
                // Set executable
                // let mut perm = fs::metadata(dst)?.permissions();
                // perm.set_mode(perm.mode() | 0o111);
                // fs::set_permissions(dst, perm)?;
                Ok(true)
            } else {
                // Run compilation command
                let stat = self.command_from_template(cmd, debug).status()?;
                Ok(stat.success())
            }
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                format!("unknown file extension '{}'", ext),
            ))
        }
    }

    /// Check if the source file needs a recompile, e.g. due to
    /// modification.
    pub fn dirty(&self, debug: bool) -> bool {
        fn try_dirty(dst: &Path, src: &Path) -> io::Result<bool> {
            let dst_time = dst.metadata()?.modified()?;
            let src_time = src.metadata()?.modified()?;
            Ok(dst_time < src_time)
        }

        fn is_dirty(dst: &Path, src: &Path) -> bool {
            match try_dirty(dst, src) {
                Ok(v) => v,
                Err(_) => true,
            }
        }

        is_dirty(self.build_path(debug), self.source_path())
            || is_dirty(self.build_path(debug), self.repo.config_path())
    }

    /// Compile the program if it has not already been compiled. If it
    /// does not need to be compiled, no action is performed and
    /// `Ok(true)` is returned.
    pub fn compile(&self, debug: bool) -> io::Result<bool> {
        if self.dirty(debug) {
            self.recompile(debug)
        } else {
            Ok(true)
        }
    }

    /// Get a list of the test cases. If the list of test cases cannot
    /// be accessed, then an empty vector is returned. Otherwise,
    /// returns a vector of the test case IDs.
    pub fn test_cases(&self) -> Vec<String> {
        fn _test_cases(dir: &Path) -> io::Result<Vec<String>> {
            let mut v = vec![];
            let read = fs::read_dir(dir)?;
            for ent in read {
                let ent: std::fs::DirEntry = ent?;
                if let Ok(mut s) = ent.file_name().into_string() {
                    if s.ends_with(".in") {
                        s.truncate(s.len() - ".in".len());
                        v.push(s);
                    } else if s.ends_with(".in.xz") {
                        s.truncate(s.len() - ".in.xz".len());
                        v.push(s);
                    }
                }
            }
            Ok(v)
        }
        match _test_cases(self.test_path()) {
            Ok(v) => v,
            Err(_) => vec![],
        }
    }

    /// Open the input and output files for the test case.
    pub fn test_files_for_case(
        &self,
        case: &str,
    ) -> io::Result<(Box<dyn Read + Send>, Box<dyn Read + Send>)> {
        let test_path = self.test_path();
        let mut in_path = test_path.to_path_buf();
        in_path.push(format!("{}.in", case));
        let in_file: Box<dyn Read + Send> = if in_path.is_file() {
            Box::new(File::open(in_path)?)
        } else {
            in_path.set_extension("in.xz");
            Box::new(XzDecoder::new(File::open(in_path)?))
        };
        let mut out_path = test_path.to_path_buf();
        out_path.push(format!("{}.out", case));
        let out_file: Box<dyn Read + Send> = if out_path.is_file() {
            Box::new(File::open(out_path)?)
        } else {
            out_path.set_extension("out.xz");
            Box::new(XzDecoder::new(File::open(out_path)?))
        };
        Ok((in_file, out_file))
    }

    /// Create a `Command` that can be used to run the
    /// program. Assumes that the program has already been compiled.
    pub fn run_command(&self) -> Command {
        if let Some(lang) = self.language() {
            let run = &lang.run;
            if !run.is_empty() {
                return self.command_from_template(run, false);
            }
        }
        Command::new(self.build_path(false))
    }

    /// Create a `Command` that can be used to run the program in a
    /// debugger specified in the configuration. Assumes that the
    /// program has already been compiled.
    pub fn debug_command(&self) -> io::Result<Command> {
        let ext = self.source_extension();
        if let Some(lang) = self.language() {
            let debug = &lang.debug;
            if !debug.is_empty() {
                return Ok(self.command_from_template(debug, true));
            }
        }
        Err(Error::new(
            ErrorKind::InvalidInput,
            format!("no debugger specified for file extension '{}'", ext),
        ))
    }

    /// Run the program in release mode. Returns true if the program
    /// exited with success, otherwise returns false. The program's
    /// stdin, stdout, and stderr are all inherited.
    pub fn run(&self) -> io::Result<bool> {
        let mut cmd = self.run_command();
        let stat = cmd.status()?;
        Ok(stat.success())
    }

    /// Compile and test the program. The program's output is compared
    /// to the expected output, and its error stream is discarded.
    pub fn test(&self, id: &str) -> io::Result<TestResult> {
        let mut cmd = self.run_command();
        let (mut in_file, mut out_file) = self.test_files_for_case(id)?;
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());
        let mut input = vec![];
        in_file.read_to_end(&mut input)?;

        let begin = Instant::now();
        let mut child = cmd.spawn()?;
        let mut stdin = child.stdin.take().unwrap();
        // This thread copies the input data to the process's stdin.
        let in_thread = thread::spawn(move || stdin.write_all(&input));
        let mut stdout = child.stdout.take().unwrap();
        let (send, recv) = mpsc::channel();
        // This thread reads the output from the child process.
        let out_thread: JoinHandle<io::Result<()>> = thread::spawn(move || {
            let mut act_output = vec![];
            stdout.read_to_end(&mut act_output)?;
            send.send(act_output).unwrap();
            Ok(())
        });
        let result = recv.recv_timeout(Duration::from_millis(self.repo.config().hard_timeout));
        let end = Instant::now();
        let dur = end - begin;
        let status = match result {
            Ok(act_output) => {
                // Program exited before the hard timeout
                let mut exp_output = vec![];
                out_file.read_to_end(&mut exp_output)?;
                let timed_out = (dur.as_secs() * 1000 + u64::from(dur.subsec_millis()))
                    >= self.repo.config().soft_timeout;
                if !child.wait()?.success() {
                    if timed_out {
                        TestStatus::CrashTimeout
                    } else {
                        TestStatus::Crash
                    }
                } else if act_output == exp_output {
                    if timed_out {
                        TestStatus::PassTimeout
                    } else {
                        TestStatus::Pass
                    }
                } else {
                    if timed_out {
                        TestStatus::WrongTimeout
                    } else {
                        TestStatus::Wrong
                    }
                }
            }
            Err(_) => {
                // Program did not exit in time
                child.kill()?;
                TestStatus::Timeout
            }
        };
        in_thread.join().unwrap()?;
        out_thread.join().unwrap()?;
        Ok(TestResult { status, time: dur })
    }

    /// Debug the program. The specified debugging program in the
    /// configuration is called. This usually means that the user is
    /// put into an interactive debugger like GDB. Returns true if the
    /// debugger exited with success, or false otherwise. This assumes
    /// that the program has already been compiled in debug mode.
    pub fn debug(&self) -> io::Result<bool> {
        let mut cmd = self.debug_command()?;
        let stat = cmd.status()?;
        Ok(stat.success())
    }

    /// Clean the program's binaries. This deletes the debug and
    /// release binaries if they exist.
    pub fn clean(&self) -> io::Result<()> {
        fn try_delete_file(path: &Path) -> io::Result<()> {
            match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            }
        }
        try_delete_file(&self.build_debug)?;
        try_delete_file(&self.build_release)?;
        Ok(())
    }
}

/// Result of a program test.
#[derive(Clone, Debug)]
pub struct TestResult {
    pub status: TestStatus,
    pub time: Duration,
}

/// Result type of the test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestStatus {
    Pass,
    Wrong,
    Crash,
    Timeout,
    PassTimeout,
    WrongTimeout,
    CrashTimeout,
}
