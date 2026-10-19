use crate::windows;
use std::{
    io::{self, Write},
    time::Duration,
};

pub fn cycle() -> io::Result<()> {
    windows::send_zero_delta()
}

pub fn diagnostic_cycle(output: &mut impl Write, elapsed: Duration) -> io::Result<()> {
    let before = windows::cursor()?;
    let last_before = windows::last_input()?;
    cycle()?;
    // Input processing can be asynchronous; this is an observation, not proof of causality.
    std::thread::sleep(Duration::from_millis(100));
    let after = windows::cursor()?;
    let last_after = windows::last_input()?;
    writeln!(
        output,
        "cycle_ms={} sent=1 dx=0 dy=0 before={before:?} after={after:?} unchanged={} last_input_before={last_before} last_input_after={last_after}",
        elapsed.as_millis(),
        before == after
    )?;
    output.flush()
}
