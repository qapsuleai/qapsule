//! Embedded time zones: UTC offset transitions, integer arithmetic only.

/// One zone's offset history.
#[derive(Debug)]
pub(crate) struct ZoneData {
    pub(crate) name: &'static str,
    /// Offset (seconds east of UTC) before the first transition.
    pub(crate) initial: i32,
    /// `(utc_second, offset_seconds)`, ascending; the offset applies from that instant.
    pub(crate) transitions: &'static [(i64, i32)],
}

/// A time zone known to the core.
#[derive(Debug, Clone, Copy)]
pub struct Zone(&'static ZoneData);

/// Result of mapping a local wall time to UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTime {
    /// Exactly one instant.
    Unique(i64),
    /// The wall time occurs twice (clocks turned back); both instants, earlier first.
    Ambiguous(i64, i64),
    /// The wall time does not exist (clocks turned forward).
    Gap,
}

impl Zone {
    /// Looks up an embedded zone by IANA name (`UTC` is always available).
    #[must_use]
    pub fn get(name: &str) -> Option<Self> {
        if name == "UTC" || name == "Etc/UTC" {
            return Some(Self(&UTC));
        }
        crate::tzdata::ZONES
            .iter()
            .find(|z| z.name == name)
            .map(Self)
    }

    /// Names of the embedded zones.
    #[must_use]
    pub fn names() -> Vec<&'static str> {
        let mut v: Vec<&str> = crate::tzdata::ZONES.iter().map(|z| z.name).collect();
        v.push("UTC");
        v
    }

    /// IANA name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.0.name
    }

    /// Offset in seconds east of UTC at UTC second `t`.
    #[must_use]
    pub fn offset_at(&self, t_s: i64) -> i32 {
        let tr = self.0.transitions;
        // Number of transitions at or before t.
        let k = tr.partition_point(|&(at, _)| at <= t_s);
        if k == 0 {
            self.0.initial
        } else {
            tr[k - 1].1
        }
    }

    /// Maps local wall-clock second `local_s` (seconds since 1970-01-01T00:00 local) to UTC.
    #[must_use]
    pub fn local_to_utc(&self, local_s: i64) -> LocalTime {
        // Any valid offset is within ±18h; collect the offsets in force around that window
        // and keep those that map back to the same wall time.
        let lo = local_s - 18 * 3600;
        let hi = local_s + 18 * 3600;
        let mut candidates: Vec<i64> = Vec::with_capacity(2);
        let tr = self.0.transitions;
        let start = tr.partition_point(|&(at, _)| at <= lo);
        let mut offsets = vec![self.offset_at(lo)];
        offsets.extend(
            tr[start..]
                .iter()
                .take_while(|&&(at, _)| at <= hi)
                .map(|&(_, o)| o),
        );
        for o in offsets {
            let utc = local_s - i64::from(o);
            if self.offset_at(utc) == o && !candidates.contains(&utc) {
                candidates.push(utc);
            }
        }
        candidates.sort_unstable();
        match candidates.as_slice() {
            [] => LocalTime::Gap,
            [one] => LocalTime::Unique(*one),
            [a, .., b] => LocalTime::Ambiguous(*a, *b),
        }
    }
}

static UTC: ZoneData = ZoneData {
    name: "UTC",
    initial: 0,
    transitions: &[],
};
