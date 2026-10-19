#![cfg_attr(not(test), windows_subsystem = "windows")]
#![cfg(windows)]

mod activity;
mod random;
mod scheduler;
mod windows;

use std::{
    fs::File,
    io::{self, Write},
    process::ExitCode,
    time::Instant,
};

fn run() -> io::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let arg = args.next();
    if args.next().is_some() {
        return Err(io::Error::other("unexpected arguments"));
    }
    if arg.as_deref() == Some(std::ffi::OsStr::new("--stop")) {
        return windows::stop();
    }
    let testing = arg.as_deref() == Some(std::ffi::OsStr::new("--test"));
    if arg.is_some() && !testing {
        return Err(io::Error::other("unknown argument"));
    }
    let Some(_instance) = windows::Instance::acquire()? else {
        return Ok(());
    };
    let stop = windows::StopEvent::create()?;
    let mut report = if testing {
        Some(File::create(
            std::env::temp_dir().join("GhostMice-test.log"),
        )?)
    } else {
        None
    };
    let started = Instant::now();
    let mut scheduler = scheduler::Scheduler::new(started);
    let mut cycles = 0;
    loop {
        if stop.wait(scheduler.remaining(Instant::now()))? {
            return Ok(());
        }
        let elapsed = started.elapsed();
        // Normal operation only calls SendInput. Observation is confined to diagnostics.
        let result = if let Some(file) = report.as_mut() {
            activity::diagnostic_cycle(file, elapsed)
        } else {
            activity::cycle()
        };
        if let Err(error) = result {
            if let Some(file) = report.as_mut() {
                writeln!(file, "ERROR: {error}")?;
            }
            // No hot retries: failures terminate silently with a nonzero exit code.
            return Err(error);
        }
        cycles += 1;
        if testing && cycles == 2 {
            return Ok(());
        }
        scheduler.advance(Instant::now());
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
