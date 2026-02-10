use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Sub};

#[derive(Debug, Clone, Copy, Ord, Eq, PartialEq, Serialize, Deserialize, PartialOrd)]
pub struct Time(pub u64);

impl Time {
    pub(crate) fn is_overlapping(time: &(Time, Time), window: &(Time, Time)) -> bool {
        time.0 < window.1 && time.1 > window.0
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let days = self.0 / 1440;
        let remaining = self.0 % 1440;
        let hours = remaining / 60;
        let mins = remaining % 60;
        write!(f, "DAY{} {:02}:{:02}", days + 1, hours, mins)
    }
}

impl Add<u64> for Time {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Time(self.0 + rhs)
    }
}

impl Add<Time> for Time {
    type Output = Self;

    fn add(self, rhs: Time) -> Self::Output {
        Time(self.0 + rhs.0)
    }
}

impl Sub<u64> for Time {
    type Output = Self;

    fn sub(self, rhs: u64) -> Self::Output {
        Time(self.0 - rhs)
    }
}

impl Sub<Time> for Time {
    type Output = Self;

    fn sub(self, rhs: Time) -> Self::Output {
        Time(self.0 - rhs.0)
    }
}

impl AddAssign<u64> for Time {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl Div<Time> for Time {
    type Output = Time;

    fn div(self, rhs: Time) -> Self::Output {
        Time(self.0 / rhs.0)
    }
}
