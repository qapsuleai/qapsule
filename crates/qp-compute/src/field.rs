//! [`Field`]: the bar fields a study can read (out of `lib.rs`).

use qp_schema::Bar;

use crate::ComputeError;

/// A bar field a study can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    /// Open price.
    Open,
    /// High price.
    High,
    /// Low price.
    Low,
    /// Close price.
    Close,
    /// Volume.
    Volume,
}

impl Field {
    /// All fields, in OHLCV order.
    pub const ALL: [Self; 5] = [Self::Open, Self::High, Self::Low, Self::Close, Self::Volume];

    /// The field's value in `bar`.
    #[must_use]
    pub fn of(self, bar: &Bar) -> f64 {
        match self {
            Self::Open => bar.o,
            Self::High => bar.h,
            Self::Low => bar.l,
            Self::Close => bar.c,
            Self::Volume => bar.v,
        }
    }

    /// Lower-case name: `open`, `high`, `low`, `close`, `volume`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::High => "high",
            Self::Low => "low",
            Self::Close => "close",
            Self::Volume => "volume",
        }
    }

    /// Parses a field name (see [`Field::name`]).
    ///
    /// # Errors
    /// Unknown name.
    pub fn parse(name: &str) -> Result<Self, ComputeError> {
        Self::ALL
            .into_iter()
            .find(|f| f.name() == name)
            .ok_or_else(|| ComputeError::new(format!("unknown field `{name}`")))
    }

    /// The column of this field over `bars`.
    #[must_use]
    pub fn column(self, bars: &[Bar]) -> Vec<f64> {
        bars.iter().map(|b| self.of(b)).collect()
    }
}
