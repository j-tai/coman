use std::fs;
use std::fs::File;
use std::io;
use std::io::{ErrorKind, Read, Write};
use std::path::Path;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use xz2::read::XzDecoder;

use crate::run::get_run_command;
use crate::Program;

/// Get a list of the test cases. If the list of test cases cannot
/// be accessed, then an empty vector is returned. Otherwise,
/// returns a vector of the test case IDs.
pub fn get_test_cases(prog: &Program) -> Vec<String> {
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
    match _test_cases(prog.test_path()) {
        Ok(v) => v,
        Err(_) => vec![],
    }
}

/// Open the input and output files for the test case.
pub fn open_test_files_for_case(
    prog: &Program,
    case: &str,
) -> io::Result<(Box<dyn Read + Send>, Box<dyn Read + Send>)> {
    let test_path = prog.test_path();
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

/// Compile and test the program. The program's output is compared
/// to the expected output, and its error stream is discarded.
pub fn test(prog: &Program, case: &str) -> io::Result<TestResult> {
    let mut cmd = get_run_command(prog);
    let (mut in_file, mut out_file) = open_test_files_for_case(prog, case)?;
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    let mut input = vec![];
    in_file.read_to_end(&mut input)?;

    let begin = Instant::now();
    let mut child = cmd.spawn()?;
    let mut stdin = child.stdin.take().unwrap();
    // This thread copies the input data to the process's stdin.
    let in_thread = thread::spawn(move || match stdin.write_all(&input) {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e),
    });
    let mut stdout = child.stdout.take().unwrap();
    let (send, recv) = mpsc::channel();
    // This thread reads the output from the child process.
    let out_thread: JoinHandle<io::Result<()>> = thread::spawn(move || {
        let mut act_output = vec![];
        stdout.read_to_end(&mut act_output)?;
        send.send(act_output).unwrap();
        Ok(())
    });
    let result = recv.recv_timeout(Duration::from_millis(
        prog.repository().config().hard_timeout,
    ));
    let end = Instant::now();
    let dur = end - begin;
    let timeout = (dur.as_secs() * 1000 + u64::from(dur.subsec_millis()))
        >= prog.repository().config().soft_timeout;
    let status = match result {
        Ok(act_output) => {
            // Program exited before the hard timeout
            let mut exp_output = vec![];
            out_file.read_to_end(&mut exp_output)?;
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
            child.kill()?;
            TestStatus::Timeout
        }
    };
    in_thread.join().unwrap()?;
    out_thread.join().unwrap()?;
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