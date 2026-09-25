use std::{fs, io::Write};

use crate::engine::search::Reporter;

pub fn crashlog(s: impl AsRef<str>, reporter: &impl Reporter) {
    let extension = "err.log";

    let current_exe =
        std::env::current_exe().expect("Unable to determine current executable directory");

    let path = current_exe.with_extension(extension);

    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();

    writeln!(f, "[{}] {}", std::process::id(), s.as_ref()).unwrap();
    f.flush().unwrap();

    reporter.error(s.as_ref());
}

#[allow(unused, reason = "Used for debugging")]
pub fn trace(s: impl AsRef<str>) {
    let extension = format!("trace.{}.log", std::process::id());

    let current_exe = std::env::current_exe().expect("Unable to determine current executable");

    let path = current_exe.with_extension(extension);

    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();

    writeln!(f, "{}", s.as_ref()).unwrap();
    f.flush().unwrap();
}
