use std::fmt;

/// TimeDescription describes "t=", "r=" fields of the session description
/// which are used to specify the start and stop times for a session as well as
/// repeat intervals and durations for the scheduled session.
#[derive(Debug, Default)]
pub struct TimeDescription {
    /// `t=<start-time> <stop-time>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.9>
    pub timing: Timing,

    /// `r=<repeat interval> <active duration> <offsets from start-time>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.10>
    pub repeat_times: Vec<RepeatTime>,
}

/// Timing defines the "t=" field's structured representation for the start and
/// stop times.
#[derive(Debug, Default)]
pub struct Timing {
    pub start_time: u64,
    pub stop_time: u64,
}

impl fmt::Display for Timing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.start_time, self.stop_time)
    }
}

/// RepeatTime describes the "r=" fields of the session description which
/// represents the intervals and durations for repeated scheduled sessions.
#[derive(Debug, Default)]
pub struct RepeatTime {
    pub interval: i64,
    pub duration: i64,
    pub offsets: Vec<i64>,
}

impl fmt::Display for RepeatTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = vec![format!("{}", self.interval), format!("{}", self.duration)];
        for value in &self.offsets {
            fields.push(format!("{}", value));
        }
        write!(f, "{}", fields.join(" "))
    }
}