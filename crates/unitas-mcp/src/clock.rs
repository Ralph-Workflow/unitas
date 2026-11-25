use std::time::{SystemTime, UNIX_EPOCH};

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FixedClock {
    now_ms: u64,
}

impl FixedClock {
    pub fn new(now_ms: u64) -> Self {
        Self { now_ms }
    }

    pub fn advance(&mut self, delta_ms: u64) {
        self.now_ms += delta_ms;
    }
}

impl Clock for FixedClock {
    fn now_ms(&self) -> u64 {
        self.now_ms
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, FixedClock, SystemClock};

    #[test]
    fn fixed_clock_is_deterministic() {
        let mut clock = FixedClock::new(1_700_000_000_000);
        assert_eq!(clock.now_ms(), 1_700_000_000_000);
        clock.advance(25);
        assert_eq!(clock.now_ms(), 1_700_000_000_025);
    }

    #[test]
    fn system_clock_reports_a_positive_timestamp() {
        assert!(SystemClock.now_ms() > 0);
    }
}
