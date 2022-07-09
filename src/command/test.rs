use std::fs::{self, File};
use std::io::{self, Cursor, ErrorKind, Read};
use std::path::Path;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use xz2::read::XzDecoder;

use crate::command::{get_run_command, RunResult};
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

struct TestData {
    args: Vec<String>,
    in_file: Box<dyn Send + Read>,
    out_file: Box<dyn Send + Read>,
}

fn open_optional_test_file(
    prog: &Program,
    case: &str,
    extension: &str,
) -> Result<Option<Box<dyn Read + Send>>> {
    fn try_open(path: impl AsRef<Path>) -> Result<Option<File>> {
        let path = path.as_ref();
        match File::open(path) {
            Ok(f) => Ok(Some(f)),
            Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("failed to read file {:?}", path)),
        }
    }

    // Try the xz-compressed one
    let mut path = prog.test_path().join(format!("{case}.{extension}.xz"));

    let mut reader: Box<dyn Read + Send> = if let Some(file) = try_open(&path)? {
        Box::new(XzDecoder::new(file))
    } else {
        path.set_extension("");
        if let Some(file) = try_open(&path)? {
            Box::new(file)
        } else {
            return Ok(None);
        }
    };

    if prog.repository().config().buffering {
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes)?;
        Ok(Some(Box::new(Cursor::new(bytes))))
    } else {
        Ok(Some(reader))
    }
}

fn open_test_file(prog: &Program, case: &str, extension: &str) -> Result<Box<dyn Read + Send>> {
    match open_optional_test_file(prog, case, extension)? {
        Some(f) => Ok(f),
        None => bail!("could not find '{}.{}' file for {}", case, extension, prog),
    }
}

/// Open the input and output files for the test case.
fn load_test_data_for_case(prog: &Program, case: &str) -> Result<TestData> {
    let args = match open_optional_test_file(prog, case, "args")? {
        Some(mut f) => {
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            s.split_whitespace()
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        }
        None => vec![],
    };
    Ok(TestData {
        args,
        in_file: open_test_file(prog, case, "in")?,
        out_file: open_test_file(prog, case, "out")?,
    })
}

/// Compile and test the program. The program's output is compared
/// to the expected output, and its error stream is discarded.
pub fn test(prog: &Program, case: &str) -> Result<TestResult> {
    // Read the entire input file, to avoid slowdowns due to XZ decoding
    let TestData {
        args,
        mut in_file,
        mut out_file,
    } = load_test_data_for_case(prog, case)?;

    // Start the program
    let mut cmd = get_run_command(prog);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.args(&args);
    let begin = Instant::now();
    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to run command {:?}", cmd))?;

    // Feed input file into stdin
    let mut stdin = child.stdin.take().unwrap();
    let in_thread = thread::spawn(move || match io::copy(&mut in_file, &mut stdin) {
        // This thread copies the input data to the process's stdin.
        Ok(_) => Ok(()),
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

    // Capture the data from stderr
    let mut stderr = child.stderr.take().unwrap();
    let err_thread: JoinHandle<io::Result<Vec<u8>>> = thread::spawn(move || {
        let mut buf = vec![];
        stderr.read_to_end(&mut buf)?;
        Ok(buf)
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
            let run_status: RunResult = child.wait()?.into();
            if !run_status.is_success() {
                TestStatus::Crash(run_status)
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

    let stderr = err_thread
        .join()
        .unwrap()
        .context("error in stdout capturing thread")?;

    Ok(TestResult {
        status,
        time: dur,
        timeout,
        stderr,
    })
}

/// Result of a program test.
#[derive(Clone, Debug)]
pub struct TestResult {
    pub status: TestStatus,
    pub time: Duration,
    pub timeout: bool,
    pub stderr: Vec<u8>,
}

impl TestResult {
    pub fn passed(&self) -> bool {
        self.status == TestStatus::Pass && !self.timeout
    }
}

/// Result type of the test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestStatus {
    Pass,
    Wrong,
    Crash(RunResult),
    Timeout,
}
