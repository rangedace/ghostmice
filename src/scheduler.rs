use std::time::{Duration, Instant};

pub const INTERVAL: Duration = Duration::from_secs(30);

pub struct Scheduler {
    next: Instant,
}

impl Scheduler {
    pub fn new(now: Instant) -> Self {
        Self {
            next: now + INTERVAL,
        }
    }
    pub fn remaining(&self, now: Instant) -> Duration {
        self.next.saturating_duration_since(now)
    }
    pub fn advance(&mut self, now: Instant) {
        self.next += INTERVAL;
        // Skip missed deadlines after suspension; never emit a catch-up burst.
        if self.next <= now {
            self.next = now + INTERVAL;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadlines_do_not_accumulate_cycle_duration() {
        let start = Instant::now();
        let mut s = Scheduler::new(start);
        assert_eq!(s.remaining(start), INTERVAL);
        s.advance(start + Duration::from_millis(30100));
        assert_eq!(
            s.remaining(start + Duration::from_secs(31)),
            Duration::from_secs(29)
        );
        s.advance(start + Duration::from_secs(300));
        assert_eq!(s.remaining(start + Duration::from_secs(300)), INTERVAL);
    }
    #[test]
    #[ignore = "real 30-second blocking wait"]
    fn real_interval() {
        let start = Instant::now();
        let s = Scheduler::new(start);
        assert!(
            !crate::windows::StopEvent::create()
                .unwrap()
                .wait(s.remaining(Instant::now()))
                .unwrap()
        );
        let elapsed = start.elapsed();
        println!("real_interval_ms={}", elapsed.as_millis());
        assert!(elapsed >= INTERVAL);
        assert!(elapsed < INTERVAL + Duration::from_secs(2));
    }
}
