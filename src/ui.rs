use crate::command::{RunResult, TestResult, TestStatus};

mod step;

use crate::step;

pub fn print_n_lines(header: &str, data: &[u8], n: usize) {
    let string = String::from_utf8_lossy(data);
    let total_lines = string.lines().count();
    if total_lines == 0 {
        return;
    } else {
        eprintln!("--- {} ---", header);
    }

    if total_lines <= n {
        string.lines().for_each(|line| println!("{}", line));
    } else {
        string
            .lines()
            .take(n - 1)
            .for_each(|line| eprintln!("{}", line));
        eprintln!("... {} more lines", total_lines - (n - 1));
    }
}

pub fn print_run_result(result: &RunResult) {
    if !result.is_success() {
        eprintln!("--- process completed with {} ---", result);
    }
}

pub fn print_test_case(case: &str) {
    step!("TEST", "{}: ", case);
}

pub fn print_test_result(result: &TestResult) {
    match result.status {
        TestStatus::Pass => eprint!("\x1b[1;32mpass\x1b[m"),
        TestStatus::Wrong => eprint!("\x1b[1;31mwrong\x1b[m"),
        TestStatus::Crash(_) => eprint!("\x1b[1;31mcrash\x1b[m"),
        TestStatus::Timeout => eprint!("\x1b[1;33mtimeout\x1b[m"),
    }
    if result.timeout && result.status != TestStatus::Timeout {
        eprint!("-\x1b[1;33mtimeout\x1b[m");
    }
    eprint!(" ");

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

    if !result.passed() {
        print_n_lines("captured stderr", &result.stderr, 12);
    }
    if let TestStatus::Crash(run_result) = &result.status {
        print_run_result(run_result);
    }
}
