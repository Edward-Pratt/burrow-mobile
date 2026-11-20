//! How long to wait before connecting again: 2 s, doubling to a minute, so
//! a computer that is off isn't dialled every second, and back to 2 s once
//! a connection works or the user brings the app back.

use std::time::Duration;

const FIRST: Duration = Duration::from_secs(2);
const MOST: Duration = Duration::from_secs(60);

pub struct Backoff {
    next: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Backoff { next: FIRST }
    }
}

impl Backoff {
    /// The wait before the next attempt, doubling the one after.
    pub fn next(&mut self) -> Duration {
        let wait = self.next;
        self.next = (self.next * 2).min(MOST);
        wait
    }

    pub fn reset(&mut self) {
        self.next = FIRST;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_doubles_to_a_minute_and_resets() {
        let mut backoff = Backoff::default();
        let waits: Vec<u64> = (0..7).map(|_| backoff.next().as_secs()).collect();
        assert_eq!(waits, [2, 4, 8, 16, 32, 60, 60]);
        backoff.reset();
        assert_eq!(backoff.next().as_secs(), 2);
    }
}
