//! ISO 6709 geographic coordinate parser and writer.
//!
//! # Examples
//!
//! ```
//! use locat::parse;
//!
//! // Decimal degrees
//! let coord = parse("+40.7128-074.0060/").unwrap();
//! assert!((coord.latitude().to_decimal_degrees() - 40.7128).abs() < 1e-10);
//!
//! // Degrees, minutes, seconds
//! let coord = parse("+404243-0740002/").unwrap();
//!
//! // With altitude and CRS
//! let coord = parse("+27.5916+086.5640+8848CRSepsg4326/").unwrap();
//! assert!(coord.altitude().is_some());
//! assert_eq!(coord.crs().unwrap().as_str(), "epsg4326");
//!
//! // Round-trip
//! let input = "+40.7128-074.0060/";
//! let coord = parse(input).unwrap();
//! assert_eq!(coord.to_string(), input);
//! ```

mod error;
mod token;
mod types;

pub use error::{Component, ParseError};
pub use types::{
  Altitude, Coordinate, CrsId, LatDMS, LatDeg, LatDegMin, Latitude, LonDMS, LonDeg, LonDegMin,
  Longitude, Sign,
};

use logos::{Lexer, Logos};
use token::Token;

/// Parses an ISO 6709 coordinate string.
///
/// The input must be a single coordinate terminated by a solidus (`/`).
///
/// # Format
///
/// ```text
/// ±Latitude±Longitude[±Altitude][CRS<id>]/
/// ```
///
/// Where latitude/longitude can be in one of three forms:
/// - Decimal degrees: `±DD.DDD` / `±DDD.DDD`
/// - Degrees + minutes: `±DDMM.MMM` / `±DDDMM.MMM`
/// - DMS: `±DDMMSS.SSS` / `±DDDMMSS.SSS`
///
/// # Examples
///
/// ```
/// use locat::parse;
///
/// let coord = parse("+40.7128-074.0060/").unwrap();
/// let (lat, lon): (f64, f64) = coord.to_decimal_degrees();
/// assert!((lat - 40.7128).abs() < 1e-10);
/// assert!((lon - (-74.006)).abs() < 1e-10);
/// ```
#[cfg_attr(not(tarpaulin), inline(always))]
pub fn parse(input: &str) -> Result<Coordinate<'_>, ParseError> {
  let mut lexer = Token::lexer(input);
  parse_coordinate(&mut lexer)
}

fn parse_coordinate<'a>(lexer: &mut Lexer<'a, Token<'a>>) -> Result<Coordinate<'a>, ParseError> {
  let latitude = parse_latitude(lexer)?;
  let longitude = parse_longitude(lexer)?;
  let (altitude, crs) = parse_tail(lexer)?;
  Ok(Coordinate::new(latitude, longitude, altitude, crs))
}

fn next_token<'a>(
  lexer: &mut Lexer<'a, Token<'a>>,
  expected: &'static str,
) -> Result<Token<'a>, ParseError> {
  match lexer.next() {
    Some(Ok(tok)) => Ok(tok),
    Some(Err(())) => Err(ParseError::Unexpected {
      position: lexer.span().start,
      expected,
    }),
    None => Err(ParseError::Unexpected {
      position: lexer.span().end,
      expected,
    }),
  }
}

// ---------------------------------------------------------------------------
// Fast numeric helpers (byte-arithmetic, regex-validated input)
// ---------------------------------------------------------------------------

/// Parse exactly 2 ASCII digit bytes into a `u8`.
#[cfg_attr(not(tarpaulin), inline(always))]
const fn digit2(b: &[u8]) -> u8 {
  (b[0] - b'0') * 10 + (b[1] - b'0')
}

/// Parse exactly 3 ASCII digit bytes into a `u16`.
#[cfg_attr(not(tarpaulin), inline(always))]
const fn digit3(b: &[u8]) -> u16 {
  (b[0] - b'0') as u16 * 100 + (b[1] - b'0') as u16 * 10 + (b[2] - b'0') as u16
}

/// Fast f64 parser for regex-validated `DIGITS[.DIGITS]` strings.
///
/// Avoids the full `str::parse::<f64>()` which handles scientific notation,
/// infinity, NaN, etc. Since logos validates the format, we only need simple
/// integer + optional fractional digit parsing.
#[cfg_attr(not(tarpaulin), inline(always))]
fn fast_parse_f64(b: &[u8]) -> f64 {
  match memchr_dot(b) {
    None => {
      let mut val: u64 = 0;
      for &byte in b {
        val = val * 10 + (byte - b'0') as u64;
      }
      val as f64
    }
    Some(dot) => {
      let mut int_val: u64 = 0;
      for &byte in &b[..dot] {
        int_val = int_val * 10 + (byte - b'0') as u64;
      }
      let frac_bytes = &b[dot + 1..];
      let mut frac_val: u64 = 0;
      for &byte in frac_bytes {
        frac_val = frac_val * 10 + (byte - b'0') as u64;
      }
      // Use a lookup table for common fractional lengths to avoid pow().
      let divisor = match frac_bytes.len() {
        1 => 10.0,
        2 => 100.0,
        3 => 1_000.0,
        4 => 10_000.0,
        5 => 100_000.0,
        6 => 1_000_000.0,
        n => 10f64.powi(n as i32),
      };
      int_val as f64 + frac_val as f64 / divisor
    }
  }
}

#[cfg_attr(not(tarpaulin), inline(always))]
fn memchr_dot(b: &[u8]) -> Option<usize> {
  b.iter().position(|&c| c == b'.')
}

// ---------------------------------------------------------------------------
// Latitude parsing
// ---------------------------------------------------------------------------

fn parse_latitude<'a>(lexer: &mut Lexer<'a, Token<'a>>) -> Result<Latitude, ParseError> {
  let tok = next_token(lexer, "latitude (±DD, ±DDMM, or ±DDMMSS)")?;
  match tok {
    Token::Signed2(s) => parse_lat_deg(s),
    Token::Signed4(s) => parse_lat_deg_min(s),
    Token::Signed6(s) => parse_lat_dms(s),
    _ => Err(ParseError::Unexpected {
      position: lexer.span().start,
      expected: "latitude (±DD, ±DDMM, or ±DDMMSS)",
    }),
  }
}

/// Parse `±DD[.D+]` into `Latitude::Deg`.
fn parse_lat_deg(s: &str) -> Result<Latitude, ParseError> {
  let b = s.as_bytes();
  let sign = Sign::from_byte(b[0]);
  let degrees = fast_parse_f64(&b[1..]);
  if degrees > 90.0 {
    return Err(ParseError::OutOfRange {
      component: Component::Latitude,
      value: degrees,
    });
  }
  Ok(Latitude::Deg(LatDeg::new(sign, degrees)))
}

/// Parse `±DDMM[.M+]` into `Latitude::DegMin`.
fn parse_lat_deg_min(s: &str) -> Result<Latitude, ParseError> {
  let b = s.as_bytes();
  let sign = Sign::from_byte(b[0]);
  let degrees = digit2(&b[1..3]);
  if degrees > 90 {
    return Err(ParseError::OutOfRange {
      component: Component::Latitude,
      value: degrees as f64,
    });
  }
  let minutes = fast_parse_f64(&b[3..]);
  if minutes >= 60.0 {
    return Err(ParseError::OutOfRange {
      component: Component::Minutes,
      value: minutes,
    });
  }
  if degrees == 90 && minutes != 0.0 {
    return Err(ParseError::OutOfRange {
      component: Component::Latitude,
      value: degrees as f64 + minutes / 60.0,
    });
  }
  Ok(Latitude::DegMin(LatDegMin::new(sign, degrees, minutes)))
}

/// Parse `±DDMMSS[.S+]` into `Latitude::DMS`.
fn parse_lat_dms(s: &str) -> Result<Latitude, ParseError> {
  let b = s.as_bytes();
  let sign = Sign::from_byte(b[0]);
  let degrees = digit2(&b[1..3]);
  if degrees > 90 {
    return Err(ParseError::OutOfRange {
      component: Component::Latitude,
      value: degrees as f64,
    });
  }
  let minutes = digit2(&b[3..5]);
  if minutes >= 60 {
    return Err(ParseError::OutOfRange {
      component: Component::Minutes,
      value: minutes as f64,
    });
  }
  let seconds = fast_parse_f64(&b[5..]);
  if seconds >= 60.0 {
    return Err(ParseError::OutOfRange {
      component: Component::Seconds,
      value: seconds,
    });
  }
  if degrees == 90 && (minutes != 0 || seconds != 0.0) {
    return Err(ParseError::OutOfRange {
      component: Component::Latitude,
      value: degrees as f64 + minutes as f64 / 60.0 + seconds / 3600.0,
    });
  }
  Ok(Latitude::DMS(LatDMS::new(sign, degrees, minutes, seconds)))
}

// ---------------------------------------------------------------------------
// Longitude parsing
// ---------------------------------------------------------------------------

fn parse_longitude<'a>(lexer: &mut Lexer<'a, Token<'a>>) -> Result<Longitude, ParseError> {
  let tok = next_token(lexer, "longitude (±DDD, ±DDDMM, or ±DDDMMSS)")?;
  match tok {
    Token::Signed3(s) => parse_lon_deg(s),
    Token::Signed5(s) => parse_lon_deg_min(s),
    Token::Signed7(s) => parse_lon_dms(s),
    _ => Err(ParseError::Unexpected {
      position: lexer.span().start,
      expected: "longitude (±DDD, ±DDDMM, or ±DDDMMSS)",
    }),
  }
}

/// Parse `±DDD[.D+]` into `Longitude::Deg`.
fn parse_lon_deg(s: &str) -> Result<Longitude, ParseError> {
  let b = s.as_bytes();
  let sign = Sign::from_byte(b[0]);
  let degrees = fast_parse_f64(&b[1..]);
  if degrees > 180.0 {
    return Err(ParseError::OutOfRange {
      component: Component::Longitude,
      value: degrees,
    });
  }
  Ok(Longitude::Deg(LonDeg::new(sign, degrees)))
}

/// Parse `±DDDMM[.M+]` into `Longitude::DegMin`.
fn parse_lon_deg_min(s: &str) -> Result<Longitude, ParseError> {
  let b = s.as_bytes();
  let sign = Sign::from_byte(b[0]);
  let degrees = digit3(&b[1..4]) as u16;
  if degrees > 180 {
    return Err(ParseError::OutOfRange {
      component: Component::Longitude,
      value: degrees as f64,
    });
  }
  let minutes = fast_parse_f64(&b[4..]);
  if minutes >= 60.0 {
    return Err(ParseError::OutOfRange {
      component: Component::Minutes,
      value: minutes,
    });
  }
  if degrees == 180 && minutes != 0.0 {
    return Err(ParseError::OutOfRange {
      component: Component::Longitude,
      value: degrees as f64 + minutes / 60.0,
    });
  }
  Ok(Longitude::DegMin(LonDegMin::new(sign, degrees, minutes)))
}

/// Parse `±DDDMMSS[.S+]` into `Longitude::DMS`.
fn parse_lon_dms(s: &str) -> Result<Longitude, ParseError> {
  let b = s.as_bytes();
  let sign = Sign::from_byte(b[0]);
  let degrees = digit3(&b[1..4]) as u16;
  if degrees > 180 {
    return Err(ParseError::OutOfRange {
      component: Component::Longitude,
      value: degrees as f64,
    });
  }
  let minutes = digit2(&b[4..6]);
  if minutes >= 60 {
    return Err(ParseError::OutOfRange {
      component: Component::Minutes,
      value: minutes as f64,
    });
  }
  let seconds = fast_parse_f64(&b[6..]);
  if seconds >= 60.0 {
    return Err(ParseError::OutOfRange {
      component: Component::Seconds,
      value: seconds,
    });
  }
  if degrees == 180 && (minutes != 0 || seconds != 0.0) {
    return Err(ParseError::OutOfRange {
      component: Component::Longitude,
      value: degrees as f64 + minutes as f64 / 60.0 + seconds / 3600.0,
    });
  }
  Ok(Longitude::DMS(LonDMS::new(sign, degrees, minutes, seconds)))
}

// ---------------------------------------------------------------------------
// Tail: optional altitude, CRS, solidus
// ---------------------------------------------------------------------------

fn parse_tail<'a>(
  lexer: &mut Lexer<'a, Token<'a>>,
) -> Result<(Option<Altitude>, Option<CrsId<'a>>), ParseError> {
  let tok = next_token(lexer, "altitude, CRS identifier, or '/'")?;

  match tok {
    Token::Solidus => Ok((None, None)),
    Token::CrsId(s) => {
      let crs = CrsId::new(&s[3..]); // strip "CRS" prefix
      expect_solidus(lexer)?;
      Ok((None, Some(crs)))
    }
    ref t if t.as_signed_str().is_some() => {
      let s = t.as_signed_str().unwrap();
      let altitude = parse_altitude(s)?;

      let next = next_token(lexer, "CRS identifier or '/'")?;
      match next {
        Token::CrsId(cs) => {
          let crs = CrsId::new(&cs[3..]);
          expect_solidus(lexer)?;
          Ok((Some(altitude), Some(crs)))
        }
        Token::Solidus => Ok((Some(altitude), None)),
        _ => Err(ParseError::Unexpected {
          position: lexer.span().start,
          expected: "CRS identifier or '/'",
        }),
      }
    }
    _ => Err(ParseError::Unexpected {
      position: lexer.span().start,
      expected: "altitude, CRS identifier, or '/'",
    }),
  }
}

fn parse_altitude(s: &str) -> Result<Altitude, ParseError> {
  let b = s.as_bytes();
  let sign = Sign::from_byte(b[0]);
  let value = fast_parse_f64(&b[1..]);
  Ok(Altitude::new(sign, value))
}

fn expect_solidus<'a>(lexer: &mut Lexer<'a, Token<'a>>) -> Result<(), ParseError> {
  match next_token(lexer, "'/'")? {
    Token::Solidus => Ok(()),
    _ => Err(ParseError::Unexpected {
      position: lexer.span().start,
      expected: "'/'",
    }),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // ---- Decimal degrees ----

  #[test]
  fn parse_decimal_degrees() {
    let coord = parse("+40.7128-074.0060/").unwrap();
    let lat = coord.latitude().to_decimal_degrees();
    let lon = coord.longitude().to_decimal_degrees();
    assert!((lat - 40.7128).abs() < 1e-10);
    assert!((lon - (-74.006)).abs() < 1e-10);
  }

  #[test]
  fn parse_integer_degrees() {
    let coord = parse("+40-074/").unwrap();
    assert!((coord.latitude().to_decimal_degrees() - 40.0).abs() < 1e-10);
    assert!((coord.longitude().to_decimal_degrees() - (-74.0)).abs() < 1e-10);
  }

  // ---- Degrees + minutes ----

  #[test]
  fn parse_deg_min() {
    let coord = parse("+4042.77-07400.36/").unwrap();
    match coord.latitude() {
      Latitude::DegMin(v) => {
        assert_eq!(v.degrees(), 40);
        assert!((v.minutes() - 42.77).abs() < 1e-10);
      }
      other => panic!("expected DegMin, got {other:?}"),
    }
    match coord.longitude() {
      Longitude::DegMin(v) => {
        assert_eq!(v.degrees(), 74);
        assert!((v.minutes() - 0.36).abs() < 1e-10);
      }
      other => panic!("expected DegMin, got {other:?}"),
    }
  }

  // ---- DMS ----

  #[test]
  fn parse_dms() {
    let coord = parse("+404243-0740002/").unwrap();
    match coord.latitude() {
      Latitude::DMS(v) => {
        assert_eq!(v.degrees(), 40);
        assert_eq!(v.minutes(), 42);
        assert!((v.seconds() - 43.0).abs() < 1e-10);
      }
      other => panic!("expected DMS, got {other:?}"),
    }
  }

  #[test]
  fn parse_dms_decimal_seconds() {
    let coord = parse("+404243.123-0740002.456/").unwrap();
    match coord.latitude() {
      Latitude::DMS(v) => {
        assert!((v.seconds() - 43.123).abs() < 1e-10);
      }
      other => panic!("expected DMS, got {other:?}"),
    }
    match coord.longitude() {
      Longitude::DMS(v) => {
        assert!((v.seconds() - 2.456).abs() < 1e-10);
      }
      other => panic!("expected DMS, got {other:?}"),
    }
  }

  // ---- Altitude + CRS ----

  #[test]
  fn parse_with_altitude_and_crs() {
    let coord = parse("+27.5916+086.5640+8848CRSepsg4326/").unwrap();
    let alt = coord.altitude().unwrap();
    assert!((alt.to_meters() - 8848.0).abs() < 1e-10);
    assert_eq!(coord.crs().unwrap().as_str(), "epsg4326");
  }

  #[test]
  fn parse_negative_altitude() {
    let coord = parse("+31.5000+035.5000-422CRSwgs84/").unwrap();
    let alt = coord.altitude().unwrap();
    assert!((alt.to_meters() - (-422.0)).abs() < 1e-10);
  }

  #[test]
  fn parse_crs_without_altitude() {
    let coord = parse("+40.7128-074.0060CRSepsg4326/").unwrap();
    assert!(coord.altitude().is_none());
    assert_eq!(coord.crs().unwrap().as_str(), "epsg4326");
  }

  // ---- Boundary values ----

  #[test]
  fn parse_poles_and_meridians() {
    // North pole, prime meridian
    let coord = parse("+90+000/").unwrap();
    assert!((coord.latitude().to_decimal_degrees() - 90.0).abs() < 1e-10);
    assert!((coord.longitude().to_decimal_degrees() - 0.0).abs() < 1e-10);

    // South pole, antimeridian
    let coord = parse("-90-180/").unwrap();
    assert!((coord.latitude().to_decimal_degrees() - (-90.0)).abs() < 1e-10);
    assert!((coord.longitude().to_decimal_degrees() - (-180.0)).abs() < 1e-10);

    // +180 longitude
    let coord = parse("+00+180/").unwrap();
    assert!((coord.longitude().to_decimal_degrees() - 180.0).abs() < 1e-10);
  }

  // ---- Validation errors ----

  #[test]
  fn reject_latitude_over_90() {
    let err = parse("+91+000/").unwrap_err();
    assert!(err.is_out_of_range());
    match err {
      ParseError::OutOfRange { component, value } => {
        assert!(component.is_latitude());
        assert!((value - 91.0).abs() < 1e-10);
      }
      _ => unreachable!(),
    }
  }

  #[test]
  fn reject_longitude_over_180() {
    let err = parse("+00+181/").unwrap_err();
    assert!(err.is_out_of_range());
    match err {
      ParseError::OutOfRange { component, value } => {
        assert!(component.is_longitude());
        assert!((value - 181.0).abs() < 1e-10);
      }
      _ => unreachable!(),
    }
  }

  #[test]
  fn reject_minutes_over_59() {
    let err = parse("+0060+00000/").unwrap_err();
    match err {
      ParseError::OutOfRange { component, value } => {
        assert!(component.is_minutes());
        assert!((value - 60.0).abs() < 1e-10);
      }
      _ => panic!("expected OutOfRange, got {err:?}"),
    }
  }

  #[test]
  fn reject_seconds_over_59() {
    let err = parse("+000060-0000060/").unwrap_err();
    match err {
      ParseError::OutOfRange { component, value } => {
        assert!(component.is_seconds());
        assert!((value - 60.0).abs() < 1e-10);
      }
      _ => panic!("expected OutOfRange, got {err:?}"),
    }
  }

  #[test]
  fn reject_missing_solidus() {
    let err = parse("+40.7128-074.0060").unwrap_err();
    assert!(err.is_unexpected());
  }

  // ---- Round-trip ----

  #[test]
  fn roundtrip_decimal_degrees() {
    extern crate std;

    use std::string::ToString;

    let input = "+40.7128-074.0060/";
    let coord = parse(input).unwrap();
    assert_eq!(coord.to_string(), input);
  }

  #[test]
  fn roundtrip_dms_integer() {
    extern crate std;

    use std::string::ToString;

    let input = "+404243-0740002/";
    let coord = parse(input).unwrap();
    assert_eq!(coord.to_string(), input);
  }

  #[test]
  fn roundtrip_with_altitude_crs() {
    extern crate std;

    use std::string::ToString;

    let input = "+27.5916+086.5640+8848CRSepsg4326/";
    let coord = parse(input).unwrap();
    assert_eq!(coord.to_string(), input);
  }

  // ---- Mount Everest (classic ISO 6709 example) ----

  #[test]
  fn parse_everest() {
    let coord = parse("+27.5916+086.5640/").unwrap();
    let (lat, lon) = coord.to_decimal_degrees();
    assert!((lat - 27.5916).abs() < 1e-10);
    assert!((lon - 86.5640).abs() < 1e-10);
  }

  // ---- Struct accessor coverage ----

  #[test]
  fn lat_deg_accessors() {
    let coord = parse("+40.7128-074.0060/").unwrap();
    match coord.latitude() {
      Latitude::Deg(v) => {
        assert!(v.sign().is_pos());
        assert!((v.degrees() - 40.7128).abs() < 1e-10);
      }
      other => panic!("expected Deg, got {other:?}"),
    }
  }

  #[test]
  fn lat_deg_min_accessors() {
    let coord = parse("+4042.77-07400.36/").unwrap();
    match coord.latitude() {
      Latitude::DegMin(v) => {
        assert!(v.sign().is_pos());
        assert_eq!(v.degrees(), 40);
        assert!((v.minutes() - 42.77).abs() < 1e-10);
      }
      other => panic!("expected DegMin, got {other:?}"),
    }
  }

  #[test]
  fn lat_dms_accessors() {
    let coord = parse("+404243.5-0740002/").unwrap();
    match coord.latitude() {
      Latitude::DMS(v) => {
        assert!(v.sign().is_pos());
        assert_eq!(v.degrees(), 40);
        assert_eq!(v.minutes(), 42);
        assert!((v.seconds() - 43.5).abs() < 1e-10);
      }
      other => panic!("expected DMS, got {other:?}"),
    }
  }

  #[test]
  fn lon_deg_accessors() {
    let coord = parse("+40.7128-074.0060/").unwrap();
    match coord.longitude() {
      Longitude::Deg(v) => {
        assert!(v.sign().is_neg());
        assert!((v.degrees() - 74.006).abs() < 1e-10);
      }
      other => panic!("expected Deg, got {other:?}"),
    }
  }

  #[test]
  fn lon_deg_min_accessors() {
    let coord = parse("+4042.77-07400.36/").unwrap();
    match coord.longitude() {
      Longitude::DegMin(v) => {
        assert!(v.sign().is_neg());
        assert_eq!(v.degrees(), 74);
        assert!((v.minutes() - 0.36).abs() < 1e-10);
      }
      other => panic!("expected DegMin, got {other:?}"),
    }
  }

  #[test]
  fn lon_dms_accessors() {
    let coord = parse("+404243-0740002.5/").unwrap();
    match coord.longitude() {
      Longitude::DMS(v) => {
        assert!(v.sign().is_neg());
        assert_eq!(v.degrees(), 74);
        assert_eq!(v.minutes(), 0);
        assert!((v.seconds() - 2.5).abs() < 1e-10);
      }
      other => panic!("expected DMS, got {other:?}"),
    }
  }

  #[test]
  fn altitude_accessors() {
    let coord = parse("+27.5916+086.5640+8848.86CRSepsg4326/").unwrap();
    let alt = coord.altitude().unwrap();
    assert!(alt.sign().is_pos());
    assert!((alt.value() - 8848.86).abs() < 1e-10);
    assert!((alt.to_meters() - 8848.86).abs() < 1e-10);
  }

  // ---- Longitude sign/to_decimal_degrees for DegMin and DMS ----

  #[test]
  fn lon_deg_min_to_decimal() {
    let coord = parse("+4042.77-07400.36/").unwrap();
    let lon = coord.longitude().to_decimal_degrees();
    assert!((lon - (-74.006)).abs() < 1e-3);
    assert!(coord.longitude().sign().is_neg());
  }

  #[test]
  fn lon_dms_to_decimal() {
    let coord = parse("+404243-0740002/").unwrap();
    let lon = coord.longitude().to_decimal_degrees();
    assert!(lon < -73.0 && lon > -75.0);
    assert!(coord.longitude().sign().is_neg());
  }

  #[test]
  fn lat_sign_south() {
    let coord = parse("-40.7128+074.0060/").unwrap();
    assert!(coord.latitude().sign().is_neg());
    assert!((coord.latitude().to_decimal_degrees() - (-40.7128)).abs() < 1e-10);
  }

  // ---- Display with decimal fractions (DegMin/DMS branches) ----

  #[test]
  fn roundtrip_deg_min_decimal() {
    extern crate std;
    use std::string::ToString;

    let input = "+4042.7700-07400.3600/";
    let coord = parse(input).unwrap();
    assert_eq!(coord.to_string(), input);
  }

  #[test]
  fn roundtrip_dms_decimal_seconds() {
    extern crate std;
    use std::string::ToString;

    let input = "+404243.123000-0740002.456000/";
    let coord = parse(input).unwrap();
    assert_eq!(coord.to_string(), input);
  }

  #[test]
  fn roundtrip_altitude_decimal() {
    extern crate std;
    use std::string::ToString;

    let input = "+27.5916+086.5640+8848.86CRSepsg4326/";
    let coord = parse(input).unwrap();
    assert_eq!(coord.to_string(), input);
  }

  // ---- Error formatting coverage ----

  #[test]
  fn error_display_messages() {
    extern crate std;
    use std::string::ToString;

    // OutOfRange display
    let err = parse("+91+000/").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("latitude degrees"));
    assert!(msg.contains("91"));

    // Unexpected display
    let err = parse("+40.7128-074.0060").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unexpected"));

    // Component display for all variants
    assert_eq!(Component::Latitude.to_string(), "latitude degrees");
    assert_eq!(Component::Longitude.to_string(), "longitude degrees");
    assert_eq!(Component::Minutes.to_string(), "minutes");
    assert_eq!(Component::Seconds.to_string(), "seconds");
  }

  // ---- Invalid input / error path coverage ----

  #[test]
  fn reject_totally_invalid_input() {
    // Should trigger lexer error or unexpected token
    assert!(parse("hello/").is_err());
  }

  #[test]
  fn reject_empty_input() {
    let err = parse("").unwrap_err();
    assert!(err.is_unexpected());
  }

  #[test]
  fn reject_wrong_token_for_latitude() {
    // 3-digit signed = longitude token in latitude position
    assert!(parse("+000+000/").is_err());
  }

  #[test]
  fn reject_wrong_token_for_longitude() {
    // 2-digit signed in longitude position
    assert!(parse("+40+00/").is_err());
  }

  #[test]
  fn reject_lat_90_with_nonzero_minutes() {
    let err = parse("+9030+00000/").unwrap_err();
    assert!(err.is_out_of_range());
  }

  #[test]
  fn reject_lon_180_with_nonzero_minutes() {
    let err = parse("+00+18030/").unwrap_err();
    assert!(err.is_out_of_range());
  }

  #[test]
  fn reject_lat_90_with_nonzero_dms() {
    let err = parse("+900001+0000000/").unwrap_err();
    assert!(err.is_out_of_range());
  }

  #[test]
  fn reject_lon_180_with_nonzero_dms() {
    let err = parse("+000000+1800001/").unwrap_err();
    assert!(err.is_out_of_range());
  }

  #[test]
  fn reject_garbage_after_longitude() {
    // Should fail in parse_tail
    assert!(parse("+40-074hello/").is_err());
  }

  #[test]
  fn reject_garbage_after_altitude() {
    // Altitude then invalid token (not CRS or solidus)
    assert!(parse("+40-074+100hello/").is_err());
  }

  #[test]
  fn reject_missing_solidus_after_crs() {
    assert!(parse("+40-074CRSfoo").is_err());
  }

  #[test]
  fn reject_missing_solidus_after_alt_crs() {
    assert!(parse("+40-074+100CRSfoo").is_err());
  }

  // ---- Latitude DegMin to_decimal_degrees ----

  #[test]
  fn lat_deg_min_to_decimal() {
    let coord = parse("+4042.77-07400.36/").unwrap();
    let lat = coord.latitude().to_decimal_degrees();
    // 40 + 42.77/60 ≈ 40.7128
    assert!((lat - 40.71283333).abs() < 1e-4);
  }

  #[test]
  fn lat_dms_to_decimal() {
    let coord = parse("+404243-0740002/").unwrap();
    let lat = coord.latitude().to_decimal_degrees();
    // 40 + 42/60 + 43/3600 ≈ 40.7119
    assert!((lat - 40.71194).abs() < 1e-4);
  }

  // ---- Validation error branches for DegMin/DMS ----

  #[test]
  fn reject_lat_deg_min_over_90() {
    let err = parse("+9100+00000/").unwrap_err();
    match err {
      ParseError::OutOfRange { component, .. } => assert!(component.is_latitude()),
      _ => panic!("expected OutOfRange, got {err:?}"),
    }
  }

  #[test]
  fn reject_lat_dms_degrees_over_90() {
    let err = parse("+910000+0000000/").unwrap_err();
    match err {
      ParseError::OutOfRange { component, .. } => assert!(component.is_latitude()),
      _ => panic!("expected OutOfRange, got {err:?}"),
    }
  }

  #[test]
  fn reject_lat_dms_minutes_over_59() {
    let err = parse("+006000+0000000/").unwrap_err();
    match err {
      ParseError::OutOfRange { component, .. } => assert!(component.is_minutes()),
      _ => panic!("expected OutOfRange, got {err:?}"),
    }
  }

  #[test]
  fn reject_lon_deg_min_over_180() {
    let err = parse("+00+18100/").unwrap_err();
    match err {
      ParseError::OutOfRange { component, .. } => assert!(component.is_longitude()),
      _ => panic!("expected OutOfRange, got {err:?}"),
    }
  }

  #[test]
  fn reject_lon_deg_min_minutes_over_59() {
    let err = parse("+0000+00060/").unwrap_err();
    match err {
      ParseError::OutOfRange { component, .. } => assert!(component.is_minutes()),
      _ => panic!("expected OutOfRange, got {err:?}"),
    }
  }

  #[test]
  fn reject_lon_dms_degrees_over_180() {
    let err = parse("+000000+1810000/").unwrap_err();
    match err {
      ParseError::OutOfRange { component, .. } => assert!(component.is_longitude()),
      _ => panic!("expected OutOfRange, got {err:?}"),
    }
  }

  #[test]
  fn reject_lon_dms_minutes_over_59() {
    let err = parse("+000000+0006000/").unwrap_err();
    match err {
      ParseError::OutOfRange { component, .. } => assert!(component.is_minutes()),
      _ => panic!("expected OutOfRange, got {err:?}"),
    }
  }

  #[test]
  fn reject_lon_dms_seconds_over_59() {
    let err = parse("+000000+0000060/").unwrap_err();
    match err {
      ParseError::OutOfRange { component, .. } => assert!(component.is_seconds()),
      _ => panic!("expected OutOfRange, got {err:?}"),
    }
  }

  // ---- Display coverage for integer DegMin/DMS longitude ----

  #[test]
  fn roundtrip_deg_min_integer() {
    extern crate std;
    use std::string::ToString;

    let input = "+4042-07400/";
    let coord = parse(input).unwrap();
    assert_eq!(coord.to_string(), input);
  }

  #[test]
  fn roundtrip_dms_integer_lon() {
    extern crate std;
    use std::string::ToString;

    let input = "+404243-0740002/";
    let coord = parse(input).unwrap();
    assert_eq!(coord.to_string(), input);
  }

  // ---- Latitude sign for DegMin/DMS south ----

  #[test]
  fn lat_deg_min_sign_south() {
    let coord = parse("-4042+07400/").unwrap();
    assert!(coord.latitude().sign().is_neg());
  }

  #[test]
  fn lat_dms_sign_south() {
    let coord = parse("-404243+0740002/").unwrap();
    assert!(coord.latitude().sign().is_neg());
  }

  // ---- Positive longitude sign/to_decimal for DegMin/DMS ----

  #[test]
  fn lon_deg_min_sign_positive() {
    let coord = parse("+4042+07400/").unwrap();
    assert!(coord.longitude().sign().is_pos());
    let lon = coord.longitude().to_decimal_degrees();
    assert!(lon > 0.0);
  }

  #[test]
  fn lon_dms_sign_positive() {
    let coord = parse("+404243+0740002/").unwrap();
    assert!(coord.longitude().sign().is_pos());
    let lon = coord.longitude().to_decimal_degrees();
    assert!(lon > 0.0);
  }
}
