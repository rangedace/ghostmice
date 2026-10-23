#![cfg_attr(not(test), windows_subsystem = "windows")]
#![cfg(windows)]

mod activity;
mod random;
mod scheduler;
mod windows;

use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    process::ExitCode,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn log_error(context: &str, error: &io::Error) {
    // Best effort: a missing/unwritable TEMP must never stop the agent.
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::temp_dir().join("GhostMice-errors.log"))
    {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let _ = writeln!(
            file,
            "unix_ms={timestamp} pid={} {context}: {error}",
            std::process::id()
        );
    }
}

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
    run_loop(
        |duration| stop.wait(duration),
        |elapsed| {
            // Observation is confined to the explicitly requested diagnostic mode.
            if let Some(file) = report.as_mut() {
                activity::diagnostic_cycle(file, elapsed)
            } else {
                activity::cycle()
            }
        },
        |error| log_error("cycle (retry at next scheduled interval)", error),
        testing,
    )
}

fn run_loop(
    mut wait: impl FnMut(Duration) -> io::Result<bool>,
    mut cycle: impl FnMut(Duration) -> io::Result<()>,
    mut on_error: impl FnMut(&io::Error),
    testing: bool,
) -> io::Result<()> {
    let started = Instant::now();
    let mut scheduler = scheduler::Scheduler::new(started);
    let mut cycles = 0;
    loop {
        if wait(scheduler.remaining(Instant::now()))? {
            return Ok(());
        }
        let elapsed = started.elapsed();
        if let Err(error) = cycle(elapsed) {
            if testing {
                return Err(error);
            }
            on_error(&error);
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
        Err(error) => {
            log_error("fatal", &error);
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_failure_retries_on_schedule_and_can_recover() {
        let mut waits = Vec::new();
        let mut cycles = 0;
        let mut errors = 0;
        run_loop(
            |duration| {
                waits.push(duration);
                Ok(waits.len() == 3)
            },
            |_| {
                cycles += 1;
                if cycles == 1 {
                    Err(io::Error::other("temporary injection failure"))
                } else {
                    Ok(())
                }
            },
            |_| errors += 1,
            false,
        )
        .unwrap();
        assert_eq!(cycles, 2);
        assert_eq!(errors, 1);
        // Fake waits return immediately, but every retry must still request a wait.
        assert_eq!(waits.len(), 3);
        assert!(waits[1] > waits[0]);
        assert!(waits[2] > waits[1]);
    }

    #[test]
    fn repeated_cycle_failures_remain_stoppable() {
        let mut waits = 0;
        let mut errors = 0;
        run_loop(
            |_| {
                waits += 1;
                Ok(waits == 4)
            },
            |_| Err(io::Error::other("injection blocked")),
            |_| errors += 1,
            false,
        )
        .unwrap();
        assert_eq!(errors, 3);
    }

    #[test]
    fn diagnostic_failure_is_still_reported_as_failure() {
        let result = run_loop(
            |_| Ok(false),
            |_| Err(io::Error::other("diagnostic injection failure")),
            |_| panic!("diagnostics must return the error to main"),
            true,
        );
        assert_eq!(
            result.unwrap_err().to_string(),
            "diagnostic injection failure"
        );
    }
}
