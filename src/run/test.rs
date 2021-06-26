use std::fs::{self, File};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use xz2::read::XzDecoder;

use crate::run::get_run_command;
use crate::Program;

/// Get a list of the test cases. If the list of test cases cannot
/// be accessed, then an empty vector is returned. Otherwise,
/// returns a vector of the test case IDs.
pub fn get_test_cases(prog: &Program) -> Result<Vec<String>> {
    let dir = prog.test_path();

    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(ref e) if e.kind() == ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e).with_context(|| format!("failed to read dir {:?}", dir)),
    };

    let mut v = vec![];
    for ent in read {
        let ent = ent.with_context(|| format!("failed to read dir {:?}", dir))?;
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

/// Open the input and output files for the test case.
pub fn open_test_files_for_case(
    prog: &Program,
    case: &str,
) -> Result<(Box<dyn Read + Send>, Box<dyn Read + Send>)> {
    fn try_open(path: impl AsRef<Path>) -> Result<Option<File>> {
        let path = path.as_ref();
        match File::open(path) {
            Ok(f) => Ok(Some(f)),
            Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("failed to read file {:?}", path)),
        }
    }

    fn open_test_file(mut path: PathBuf) -> Result<Box<dyn Read + Send>> {
        if let Some(file) = try_open(&path)? {
            Ok(Box::new(XzDecoder::new(file)))
        } else {
            path.set_extension("");
            if let Some(file) = try_open(&path)? {
                Ok(Box::new(file))
            } else {
                bail!("file not found: {:?}", path);
            }
        }
    }

    let test_path = prog.test_path();

    let mut in_path = test_path.to_path_buf();
    in_path.push(format!("{}.in.xz", case));
    let in_file = open_test_file(in_path)?;

    let mut out_path = test_path.to_path_buf();
    out_path.push(format!("{}.out.xz", case));
    let out_file = open_test_file(out_path)?;

    Ok((in_file, out_file))
}

/// Compile and test the program. The program's output is compared
/// to the expected output, and its error stream is discarded.
pub fn test(prog: &Program, case: &str) -> Result<TestResult> {
    // Read the entire input file, to avoid slowdowns due to XZ decoding
    let (mut in_file, mut out_file) = open_test_files_for_case(prog, case)?;
    let mut input = vec![];
    in_file
        .read_to_end(&mut input)
        .context("failed to read test input file")?;

    // Start the program
    let mut cmd = get_run_command(prog);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    let begin = Instant::now();
    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to run command {:?}", cmd))?;

    // Feed input file into stdin
    let mut stdin = child.stdin.take().unwrap();
    let in_thread = thread::spawn(move || match stdin.write_all(&input) {
        // This thread copies the input data to the process's stdin.
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e),
    });

    // Capture the data from stdout
    let mut stdout = child.stdout.take().unwrap();
    // The output is sent through this channel. We can't just return this value
    // when the thread exits, because that doesn't allow us to `recv_timeout`.
    let (send, recv) = mpsc::channel();
    let out_thread: JoinHandle<io::Result<()>> = thread::spawn(move || {
        // This thread reads the output from the child process.
        let mut act_output = vec![];
        stdout.read_to_end(&mut act_output)?;
        send.send(act_output).unwrap();
        Ok(())
    });

    // Get the result with the hard timeout
    let result = recv.recv_timeout(Duration::from_millis(
        prog.repository().config().hard_timeout,
    ));
    // Calculate the end time and time taken
    let end = Instant::now();
    let dur = end - begin;
    let timeout = (dur.as_secs() * 1000 + u64::from(dur.subsec_millis()))
        >= prog.repository().config().soft_timeout;

    // Test outcome
    let status = match result {
        Ok(act_output) => {
            // Program exited before the hard timeout
            let mut exp_output = vec![];
            out_file
                .read_to_end(&mut exp_output)
                .context("failed to read output file")?;
            if !child.wait()?.success() {
                TestStatus::Crash
            } else if act_output == exp_output {
                TestStatus::Pass
            } else {
                TestStatus::Wrong
            }
        }
        Err(_) => {
            // Program did not exit in time
            child.kill().context("failed to kill child process")?;
            TestStatus::Timeout
        }
    };

    // Let the threads finish
    in_thread
        .join()
        .unwrap()
        .context("error in input feeding thread")?;

    out_thread
        .join()
        .unwrap()
        .context("error in stdout capturing thread")?;

    Ok(TestResult {
        status,
        time: dur,
        timeout,
    })
}

/// Result of a program test.
#[derive(Clone, Debug)]
pub struct TestResult {
    pub status: TestStatus,
    pub time: Duration,
    pub timeout: bool,
}

/// Result type of the test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestStatus {
    Pass,
    Wrong,
    Crash,
    Timeout,
}
