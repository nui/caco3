#![allow(clippy::unnecessary_lazy_evaluations)]

use std::fmt::{self, Display, Write};

const MINUTE_SECONDS: u64 = 60;
const HOUR_SECONDS: u64 = 60 * MINUTE_SECONDS;
const DAY_SECONDS: u64 = 24 * HOUR_SECONDS;

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct HumanDuration(u64);

impl HumanDuration {
    pub const fn from_secs(secs: u64) -> Self {
        HumanDuration(secs)
    }

    pub fn days(self) -> Option<u64> {
        (self.0 >= DAY_SECONDS).then(|| self.0 / DAY_SECONDS)
    }

    pub fn hours(self) -> Option<u64> {
        (self.0 >= HOUR_SECONDS).then(|| self.0 / HOUR_SECONDS % 24)
    }

    pub fn minutes(self) -> Option<u64> {
        (self.0 >= MINUTE_SECONDS).then(|| self.0 / MINUTE_SECONDS % 60)
    }

    pub const fn secs(self) -> u64 {
        self.0 % MINUTE_SECONDS
    }

    pub fn format(self, num_components: u8) -> String {
        let capacity = (num_components.saturating_mul(4)).min(16).into();
        let mut buf = String::with_capacity(capacity);
        write!(&mut buf, "{}", self.display(num_components))
            .expect(HUMAN_DURATION_DISPLAY_IMPL_ERROR);
        buf
    }

    pub fn format_all(self) -> String {
        self.format(DurationComponent::ALL_COMPONENTS)
    }

    pub const fn components(self) -> DurationComponents {
        DurationComponents::new(self)
    }

    pub const fn display(self, num_components: u8) -> HumanDurationDisplay {
        HumanDurationDisplay {
            human_duration: self,
            num_components,
        }
    }

    pub const fn display_all(self) -> HumanDurationDisplay {
        self.display(DurationComponent::ALL_COMPONENTS)
    }
}

#[derive(Clone, Copy)]
pub struct DurationComponent {
    value: u64,
    unit: Unit,
}

impl DurationComponent {
    pub const ALL_COMPONENTS: u8 = 4;

    fn days(value: u64) -> Self {
        Self {
            value,
            unit: Unit::Day,
        }
    }

    fn hours(value: u64) -> Self {
        Self {
            value,
            unit: Unit::Hour,
        }
    }

    fn minutes(value: u64) -> Self {
        Self {
            value,
            unit: Unit::Minute,
        }
    }

    fn seconds(value: u64) -> Self {
        Self {
            value,
            unit: Unit::Second,
        }
    }
}

impl Display for DurationComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Unit::*;
        let Self { value, unit } = *self;
        match unit {
            Day => write!(f, "{value}d"),
            Hour => write!(f, "{value}h"),
            Minute => write!(f, "{value}m"),
            Second => write!(f, "{value}s"),
        }
    }
}

#[derive(Clone, Copy, Default)]
enum Unit {
    Day,
    Hour,
    Minute,
    #[default]
    Second,
}

impl Unit {
    pub const BIGGEST: Self = Self::Day;

    fn next_smaller(self) -> Option<Self> {
        use Unit::*;
        match self {
            Day => Some(Hour),
            Hour => Some(Minute),
            Minute => Some(Second),
            Second => None,
        }
    }
}

pub struct DurationComponents {
    time: HumanDuration,
    unit: Option<Unit>,
}

impl DurationComponents {
    const fn new(time: HumanDuration) -> Self {
        DurationComponents {
            time,
            unit: Some(Unit::BIGGEST),
        }
    }
}

impl Iterator for DurationComponents {
    type Item = DurationComponent;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let unit = self.unit?;
            self.unit = unit.next_smaller();
            let component = match unit {
                Unit::Day => self.time.days().map(DurationComponent::days),
                Unit::Hour => self.time.hours().map(DurationComponent::hours),
                Unit::Minute => self.time.minutes().map(DurationComponent::minutes),
                Unit::Second => Some(DurationComponent::seconds(self.time.secs())),
            };
            if component.is_some() {
                break component;
            }
        }
    }
}

impl Display for HumanDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.display(DurationComponent::ALL_COMPONENTS), f)
    }
}

#[derive(Copy, Clone)]
pub struct HumanDurationDisplay {
    human_duration: HumanDuration,
    num_components: u8,
}

const HUMAN_DURATION_DISPLAY_IMPL_ERROR: &str =
    "a HumanTime formatting implementation returned an error";

impl Display for HumanDurationDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut components = self
            .human_duration
            .components()
            .take(self.num_components.into());
        if let Some(c) = components.next() {
            write!(f, "{c}").expect(HUMAN_DURATION_DISPLAY_IMPL_ERROR);
        }
        for c in components {
            write!(f, " {c}").expect(HUMAN_DURATION_DISPLAY_IMPL_ERROR);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use time::ext::NumericalStdDuration;

    use super::*;

    #[test]
    fn test_to_human() {
        assert_eq!(HumanDuration(1).format(2), "1s");
        assert_eq!(HumanDuration(10).format(2), "10s");
        assert_eq!(HumanDuration(59).format(2), "59s");
        assert_eq!(HumanDuration(MINUTE_SECONDS).format(2), "1m 0s");
        assert_eq!(HumanDuration(HOUR_SECONDS).format(2), "1h 0m");
        assert_eq!(HumanDuration(HOUR_SECONDS - 1).format(2), "59m 59s");
        assert_eq!(HumanDuration(HOUR_SECONDS).format(2), "1h 0m");
        assert_eq!(HumanDuration(HOUR_SECONDS + 1).format(2), "1h 0m");
        assert_eq!(HumanDuration(DAY_SECONDS - 1).format(2), "23h 59m");
        assert_eq!(HumanDuration(DAY_SECONDS).format(2), "1d 0h");
        assert_eq!(HumanDuration(DAY_SECONDS + 1).format(2), "1d 0h");
    }

    #[test]
    fn test_iterator() {
        let secs = (1.std_days() + 5.std_hours() + 7.std_minutes() + 3.std_seconds()).as_secs();
        let components = HumanDuration::from_secs(secs)
            .components()
            .collect::<Vec<_>>();
        assert_eq!(components[0].to_string(), "1d");
        assert_eq!(components[1].to_string(), "5h");
        assert_eq!(components[2].to_string(), "7m");
        assert_eq!(components[3].to_string(), "3s");
    }
}
