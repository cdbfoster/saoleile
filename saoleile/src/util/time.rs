use std::cmp::Ordering;
use std::convert::From;
use std::f32::EPSILON;
use std::fmt;
use std::ops::{Add, Sub};

pub type Tick = u64;

impl PartialEq<Time> for Tick {
    fn eq(&self, time: &Time) -> bool {
        *self == time.tick
    }
}

impl PartialOrd<Time> for Tick {
    fn partial_cmp(&self, time: &Time) -> Option<Ordering> {
        self.partial_cmp(&time.tick)
    }
}

#[derive(Clone, Copy)]
pub struct Time {
    tick: Tick,
    fractional_tick: f32,
}

impl Time {
    pub fn new(tick: Tick, fractional_tick: f32) -> Self {
        Self {
            tick,
            fractional_tick,
        }
    }

    pub fn get_tick(&self) -> Tick {
        self.tick
    }

    pub fn get_fractional_tick(&self) -> f32 {
        self.fractional_tick
    }

    pub fn as_f32(&self) -> f32 {
        self.tick as f32 + self.fractional_tick
    }
}

impl Add<Tick> for Time {
    type Output = Self;

    fn add(self, tick: Tick) -> Self {
        Self {
            tick: self.tick + tick,
            fractional_tick: self.fractional_tick,
        }
    }
}

impl Add for Time {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let tick = self.tick + other.tick;
        let fraction = self.fractional_tick + other.fractional_tick;
        Self {
            tick: tick + fraction.trunc() as u64,
            fractional_tick: fraction.fract(),
        }
    }
}

impl fmt::Debug for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.tick)?;
        write!(f, "{}", &format!("{:.4}", self.fractional_tick)[1..])
    }
}

impl Eq for Time { }

impl From<f64> for Time {
    fn from(ticks: f64) -> Self {
        Self {
            tick: ticks.trunc() as u64,
            fractional_tick: ticks.fract() as f32,
        }
    }
}

impl From<Tick> for Time {
    fn from(tick: Tick) -> Self {
        Self {
            tick,
            fractional_tick: 0.0,
        }
    }
}

impl PartialEq<Tick> for Time {
    fn eq(&self, tick: &Tick) -> bool {
        self.tick == *tick && self.fractional_tick < EPSILON
    }
}

impl PartialEq<Time> for Time {
    fn eq(&self, other: &Time) -> bool {
        self.tick == other.tick && (self.fractional_tick - other.fractional_tick).abs() < EPSILON
    }
}

impl PartialOrd<Tick> for Time {
    fn partial_cmp(&self, tick: &Tick) -> Option<Ordering> {
        self.tick.partial_cmp(tick)
    }
}

impl PartialOrd<Time> for Time {
    fn partial_cmp(&self, other: &Time) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Time) -> Ordering {
        if self.tick != other.tick {
            self.tick.cmp(&other.tick)
        } else {
            self.fractional_tick.partial_cmp(&other.fractional_tick).unwrap_or(Ordering::Equal)
        }
    }
}

impl Sub<Tick> for Time {
    type Output = Self;

    fn sub(self, tick: Tick) -> Self {
        assert!(self >= tick);
        Self {
            tick: self.tick - tick,
            fractional_tick: self.fractional_tick,
        }
    }
}

impl Sub for Time {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        assert!(self >= other);
        let mut tick = self.tick - other.tick;
        let mut fractional_tick = self.fractional_tick - other.fractional_tick;

        if fractional_tick < 0.0 {
            tick -= 1;
            fractional_tick += 1.0;
        }

        Self {
            tick,
            fractional_tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_addition() {
        let base = Time::new(20, 0.5);
        assert_eq!(base + Tick::from(1u64), Time::new(21, 0.5));
        assert_eq!(base + Tick::from(7u64), Time::new(27, 0.5));
        assert_eq!(base + Time::from(0.5), Time::new(21, 0.0));
        assert_eq!(base + Time::from(0.567), Time::new(21, 0.067));
        assert_eq!(base + Time::from(3.234), Time::new(23, 0.734));
    }

    #[test]
    fn test_time_subtraction() {
        let base = Time::new(20, 0.5);
        assert_eq!(base - Tick::from(1u64), Time::new(19, 0.5));
        assert_eq!(base - Tick::from(7u64), Time::new(13, 0.5));
        assert_eq!(base - Time::from(0.5), Time::new(20, 0.0));
        assert_eq!(base - Time::from(0.567), Time::new(19, 0.933));
        assert_eq!(base - Time::from(3.234), Time::new(17, 0.266));
    }
}