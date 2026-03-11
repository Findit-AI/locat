use core::fmt;

use derive_more::IsVariant;

/// Which coordinate component caused a range error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IsVariant)]
pub enum Component {
  /// Latitude degrees (must be 0–90).
  Latitude,
  /// Longitude degrees (must be 0–180).
  Longitude,
  /// Minutes (must be 0–59).
  Minutes,
  /// Seconds (must be 0–59.999…).
  Seconds,
}

impl fmt::Display for Component {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Latitude => f.write_str("latitude degrees"),
      Self::Longitude => f.write_str("longitude degrees"),
      Self::Minutes => f.write_str("minutes"),
      Self::Seconds => f.write_str("seconds"),
    }
  }
}

/// Errors that can occur when parsing an ISO 6709 coordinate string.
#[derive(Debug, Clone, PartialEq, thiserror::Error, IsVariant)]
pub enum ParseError {
  /// Unexpected token or end-of-input at the given byte offset.
  #[error("unexpected input at byte {position}, expected {expected}")]
  Unexpected {
    /// Byte offset in the input.
    position: usize,
    /// Human-readable description of what was expected.
    expected: &'static str,
  },
  /// A component value is outside its valid range.
  #[error("{component} value {value} out of range")]
  OutOfRange {
    /// Which component is out of range.
    component: Component,
    /// The actual value that was parsed.
    value: f64,
  },
  /// Failed to parse a numeric value from the input.
  #[error("invalid number at byte {position}")]
  InvalidNumber {
    /// Byte offset in the input.
    position: usize,
  },
}
