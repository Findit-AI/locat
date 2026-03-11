use core::fmt;

use derive_more::{Display, IsVariant, TryUnwrap, Unwrap};

/// Sign of a coordinate component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, IsVariant)]
pub enum Sign {
  /// Positive (`+`): North for latitude, East for longitude.
  #[display("+")]
  Pos,
  /// Negative (`-`): South for latitude, West for longitude.
  #[display("-")]
  Neg,
}

impl Sign {
  /// Parse from the first byte of a signed numeric string.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub(crate) const fn from_byte(b: u8) -> Self {
    match b {
      b'-' => Self::Neg,
      _ => Self::Pos,
    }
  }

  /// Returns `1.0` for positive, `-1.0` for negative.
  #[cfg_attr(not(tarpaulin), inline(always))]
  const fn multiplier(self) -> f64 {
    match self {
      Self::Pos => 1.0,
      Self::Neg => -1.0,
    }
  }
}

// ---------------------------------------------------------------------------
// Latitude
// ---------------------------------------------------------------------------

/// Decimal degrees latitude: `±DD.DDD` (0.0–90.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatDeg {
  /// Sign (N=`+`, S=`-`).
  sign: Sign,
  /// Degrees value (0.0–90.0).
  degrees: f64,
}

impl LatDeg {
  /// Creates a new decimal degrees latitude.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(sign: Sign, degrees: f64) -> Self {
    Self { sign, degrees }
  }

  /// Returns the sign.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn sign(&self) -> Sign {
    self.sign
  }

  /// Returns the degrees value.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn degrees(&self) -> f64 {
    self.degrees
  }
}

impl fmt::Display for LatDeg {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}{:07.4}", self.sign, self.degrees)
  }
}

/// Degrees and decimal minutes latitude: `±DDMM.MMM`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatDegMin {
  /// Sign (N=`+`, S=`-`).
  sign: Sign,
  /// Integer degrees (0–90).
  degrees: u8,
  /// Minutes value (0.0–59.999…).
  minutes: f64,
}

impl LatDegMin {
  /// Creates a new degrees+minutes latitude.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(sign: Sign, degrees: u8, minutes: f64) -> Self {
    Self {
      sign,
      degrees,
      minutes,
    }
  }

  /// Returns the sign.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn sign(&self) -> Sign {
    self.sign
  }

  /// Returns the integer degrees.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn degrees(&self) -> u8 {
    self.degrees
  }

  /// Returns the minutes value.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn minutes(&self) -> f64 {
    self.minutes
  }
}

impl fmt::Display for LatDegMin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let min_int = self.minutes as u8;
    let min_frac = self.minutes - min_int as f64;
    if min_frac == 0.0 {
      write!(f, "{}{:02}{min_int:02}", self.sign, self.degrees)
    } else {
      write!(f, "{}{:02}{:07.4}", self.sign, self.degrees, self.minutes)
    }
  }
}

/// Degrees, minutes, and decimal seconds latitude: `±DDMMSS.SSS`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatDMS {
  /// Sign (N=`+`, S=`-`).
  sign: Sign,
  /// Integer degrees (0–90).
  degrees: u8,
  /// Integer minutes (0–59).
  minutes: u8,
  /// Seconds value (0.0–59.999…).
  seconds: f64,
}

impl LatDMS {
  /// Creates a new DMS latitude.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(sign: Sign, degrees: u8, minutes: u8, seconds: f64) -> Self {
    Self {
      sign,
      degrees,
      minutes,
      seconds,
    }
  }

  /// Returns the sign.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn sign(&self) -> Sign {
    self.sign
  }

  /// Returns the integer degrees.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn degrees(&self) -> u8 {
    self.degrees
  }

  /// Returns the integer minutes.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn minutes(&self) -> u8 {
    self.minutes
  }

  /// Returns the seconds value.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn seconds(&self) -> f64 {
    self.seconds
  }
}

impl fmt::Display for LatDMS {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let sec_int = self.seconds as u8;
    let sec_frac = self.seconds - sec_int as f64;
    if sec_frac == 0.0 {
      write!(
        f,
        "{}{:02}{:02}{sec_int:02}",
        self.sign, self.degrees, self.minutes
      )
    } else {
      write!(
        f,
        "{}{:02}{:02}{:09.6}",
        self.sign, self.degrees, self.minutes, self.seconds
      )
    }
  }
}

/// Latitude component of an ISO 6709 coordinate.
///
/// Preserves the original notation form (degrees, deg+min, or DMS)
/// for faithful round-trip serialization.
#[derive(Debug, Display, Clone, Copy, PartialEq, IsVariant, Unwrap, TryUnwrap)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
pub enum Latitude {
  /// Decimal degrees: `±DD.DDD`
  #[display("{_0}")]
  Deg(LatDeg),
  /// Degrees and decimal minutes: `±DDMM.MMM`
  #[display("{_0}")]
  DegMin(LatDegMin),
  /// Degrees, minutes, and decimal seconds: `±DDMMSS.SSS`
  #[display("{_0}")]
  DMS(LatDMS),
}

impl Latitude {
  /// Returns the sign.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn sign(&self) -> Sign {
    match self {
      Self::Deg(v) => v.sign,
      Self::DegMin(v) => v.sign,
      Self::DMS(v) => v.sign,
    }
  }

  /// Converts to a signed decimal degrees value.
  ///
  /// ```
  /// use locat::parse;
  ///
  /// let coord = parse("+40.7128-074.0060/").unwrap();
  /// let lat = coord.latitude().to_decimal_degrees();
  /// assert!((lat - 40.7128).abs() < 1e-10);
  /// ```
  pub fn to_decimal_degrees(&self) -> f64 {
    let (sign, val) = match self {
      Self::Deg(v) => (v.sign, v.degrees),
      Self::DegMin(v) => (v.sign, v.degrees as f64 + v.minutes / 60.0),
      Self::DMS(v) => (
        v.sign,
        v.degrees as f64 + v.minutes as f64 / 60.0 + v.seconds / 3600.0,
      ),
    };
    sign.multiplier() * val
  }
}

// ---------------------------------------------------------------------------
// Longitude
// ---------------------------------------------------------------------------

/// Decimal degrees longitude: `±DDD.DDD` (0.0–180.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LonDeg {
  /// Sign (E=`+`, W=`-`).
  sign: Sign,
  /// Degrees value (0.0–180.0).
  degrees: f64,
}

impl LonDeg {
  /// Creates a new decimal degrees longitude.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(sign: Sign, degrees: f64) -> Self {
    Self { sign, degrees }
  }

  /// Returns the sign.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn sign(&self) -> Sign {
    self.sign
  }

  /// Returns the degrees value.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn degrees(&self) -> f64 {
    self.degrees
  }
}

impl fmt::Display for LonDeg {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}{:08.4}", self.sign, self.degrees)
  }
}

/// Degrees and decimal minutes longitude: `±DDDMM.MMM`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LonDegMin {
  /// Sign (E=`+`, W=`-`).
  sign: Sign,
  /// Integer degrees (0–180).
  degrees: u16,
  /// Minutes value (0.0–59.999…).
  minutes: f64,
}

impl LonDegMin {
  /// Creates a new degrees+minutes longitude.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(sign: Sign, degrees: u16, minutes: f64) -> Self {
    Self {
      sign,
      degrees,
      minutes,
    }
  }

  /// Returns the sign.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn sign(&self) -> Sign {
    self.sign
  }

  /// Returns the integer degrees.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn degrees(&self) -> u16 {
    self.degrees
  }

  /// Returns the minutes value.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn minutes(&self) -> f64 {
    self.minutes
  }
}

impl fmt::Display for LonDegMin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let min_int = self.minutes as u8;
    let min_frac = self.minutes - min_int as f64;
    if min_frac == 0.0 {
      write!(f, "{}{:03}{min_int:02}", self.sign, self.degrees)
    } else {
      write!(f, "{}{:03}{:07.4}", self.sign, self.degrees, self.minutes)
    }
  }
}

/// Degrees, minutes, and decimal seconds longitude: `±DDDMMSS.SSS`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LonDMS {
  /// Sign (E=`+`, W=`-`).
  sign: Sign,
  /// Integer degrees (0–180).
  degrees: u16,
  /// Integer minutes (0–59).
  minutes: u8,
  /// Seconds value (0.0–59.999…).
  seconds: f64,
}

impl LonDMS {
  /// Creates a new DMS longitude.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(sign: Sign, degrees: u16, minutes: u8, seconds: f64) -> Self {
    Self {
      sign,
      degrees,
      minutes,
      seconds,
    }
  }

  /// Returns the sign.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn sign(&self) -> Sign {
    self.sign
  }

  /// Returns the integer degrees.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn degrees(&self) -> u16 {
    self.degrees
  }

  /// Returns the integer minutes.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn minutes(&self) -> u8 {
    self.minutes
  }

  /// Returns the seconds value.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn seconds(&self) -> f64 {
    self.seconds
  }
}

impl fmt::Display for LonDMS {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let sec_int = self.seconds as u8;
    let sec_frac = self.seconds - sec_int as f64;
    if sec_frac == 0.0 {
      write!(
        f,
        "{}{:03}{:02}{sec_int:02}",
        self.sign, self.degrees, self.minutes
      )
    } else {
      write!(
        f,
        "{}{:03}{:02}{:09.6}",
        self.sign, self.degrees, self.minutes, self.seconds
      )
    }
  }
}

/// Longitude component of an ISO 6709 coordinate.
///
/// Preserves the original notation form for faithful round-trip serialization.
#[derive(Debug, Display, Clone, Copy, PartialEq, IsVariant, Unwrap, TryUnwrap)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
pub enum Longitude {
  /// Decimal degrees: `±DDD.DDD`
  #[display("{_0}")]
  Deg(LonDeg),
  /// Degrees and decimal minutes: `±DDDMM.MMM`
  #[display("{_0}")]
  DegMin(LonDegMin),
  /// Degrees, minutes, and decimal seconds: `±DDDMMSS.SSS`
  #[display("{_0}")]
  DMS(LonDMS),
}

impl Longitude {
  /// Returns the sign.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn sign(&self) -> Sign {
    match self {
      Self::Deg(v) => v.sign,
      Self::DegMin(v) => v.sign,
      Self::DMS(v) => v.sign,
    }
  }

  /// Converts to a signed decimal degrees value.
  ///
  /// ```
  /// use locat::parse;
  ///
  /// let coord = parse("+40.7128-074.0060/").unwrap();
  /// let lon = coord.longitude().to_decimal_degrees();
  /// assert!((lon - (-74.0060)).abs() < 1e-10);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn to_decimal_degrees(&self) -> f64 {
    let (sign, val) = match self {
      Self::Deg(v) => (v.sign, v.degrees),
      Self::DegMin(v) => (v.sign, v.degrees as f64 + v.minutes / 60.0),
      Self::DMS(v) => (
        v.sign,
        v.degrees as f64 + v.minutes as f64 / 60.0 + v.seconds / 3600.0,
      ),
    };
    sign.multiplier() * val
  }
}

// ---------------------------------------------------------------------------
// Altitude & CRS
// ---------------------------------------------------------------------------

/// Altitude in meters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Altitude {
  sign: Sign,
  value: f64,
}

impl Altitude {
  /// Creates a new altitude.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(sign: Sign, value: f64) -> Self {
    Self { sign, value }
  }

  /// Returns the sign.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn sign(&self) -> Sign {
    self.sign
  }

  /// Returns the unsigned altitude value in meters.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn value(&self) -> f64 {
    self.value
  }

  /// Returns the signed altitude value in meters.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn to_meters(&self) -> f64 {
    self.sign.multiplier() * self.value
  }
}

impl fmt::Display for Altitude {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let int = self.value as u64;
    let frac = self.value - int as f64;
    if frac == 0.0 {
      write!(f, "{}{int}", self.sign)
    } else {
      write!(f, "{}{}", self.sign, self.value)
    }
  }
}

/// CRS (Coordinate Reference System) identifier.
///
/// Zero-copy: borrows the identifier string from the input.
/// The `CRS` prefix is stripped; e.g., for input `CRSepsg4326`, the
/// id is `epsg4326`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CrsId<'a>(&'a str);

impl<'a> CrsId<'a> {
  /// Creates a new CRS identifier.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(id: &'a str) -> Self {
    Self(id)
  }

  /// Returns the identifier string (without the `CRS` prefix).
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'a str {
    self.0
  }
}

impl fmt::Display for CrsId<'_> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "CRS{}", self.0)
  }
}

// ---------------------------------------------------------------------------
// Coordinate
// ---------------------------------------------------------------------------

/// A parsed ISO 6709 geographic coordinate.
///
/// ```
/// use locat::parse;
///
/// let coord = parse("+40.7128-074.0060/").unwrap();
/// assert!((coord.latitude().to_decimal_degrees() - 40.7128).abs() < 1e-10);
/// assert!((coord.longitude().to_decimal_degrees() - (-74.006)).abs() < 1e-10);
/// assert!(coord.altitude().is_none());
/// assert!(coord.crs().is_none());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Coordinate<'a> {
  latitude: Latitude,
  longitude: Longitude,
  altitude: Option<Altitude>,
  crs: Option<CrsId<'a>>,
}

impl<'a> Coordinate<'a> {
  /// Creates a new coordinate.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(
    latitude: Latitude,
    longitude: Longitude,
    altitude: Option<Altitude>,
    crs: Option<CrsId<'a>>,
  ) -> Self {
    Self {
      latitude,
      longitude,
      altitude,
      crs,
    }
  }

  /// Returns the latitude component.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn latitude(&self) -> &Latitude {
    &self.latitude
  }

  /// Returns the longitude component.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn longitude(&self) -> &Longitude {
    &self.longitude
  }

  /// Returns the altitude component, if present.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn altitude(&self) -> Option<&Altitude> {
    self.altitude.as_ref()
  }

  /// Returns the CRS identifier, if present.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn crs(&self) -> Option<&CrsId<'a>> {
    self.crs.as_ref()
  }

  /// Converts to `(latitude, longitude)` as signed decimal degrees.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn to_decimal_degrees(&self) -> (f64, f64) {
    (
      self.latitude.to_decimal_degrees(),
      self.longitude.to_decimal_degrees(),
    )
  }
}

impl fmt::Display for Coordinate<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}{}", self.latitude, self.longitude)?;
    if let Some(alt) = &self.altitude {
      write!(f, "{alt}")?;
    }
    if let Some(crs) = &self.crs {
      write!(f, "{crs}")?;
    }
    f.write_str("/")
  }
}
