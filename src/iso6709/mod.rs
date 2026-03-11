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

#[cfg_attr(not(tarpaulin), inline(always))]
fn num_err(position: usize) -> ParseError {
  ParseError::InvalidNumber { position }
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
  let sign = Sign::from_byte(s.as_bytes()[0]);
  let degrees: f64 = s[1..].parse().map_err(|_| num_err(0))?;
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
  let sign = Sign::from_byte(s.as_bytes()[0]);
  let degrees: u8 = s[1..3].parse().map_err(|_| num_err(0))?;
  if degrees > 90 {
    return Err(ParseError::OutOfRange {
      component: Component::Latitude,
      value: degrees as f64,
    });
  }
  let minutes: f64 = s[3..].parse().map_err(|_| num_err(0))?;
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
  let sign = Sign::from_byte(s.as_bytes()[0]);
  let degrees: u8 = s[1..3].parse().map_err(|_| num_err(0))?;
  if degrees > 90 {
    return Err(ParseError::OutOfRange {
      component: Component::Latitude,
      value: degrees as f64,
    });
  }
  let minutes: u8 = s[3..5].parse().map_err(|_| num_err(0))?;
  if minutes >= 60 {
    return Err(ParseError::OutOfRange {
      component: Component::Minutes,
      value: minutes as f64,
    });
  }
  let seconds: f64 = s[5..].parse().map_err(|_| num_err(0))?;
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
  let sign = Sign::from_byte(s.as_bytes()[0]);
  let degrees: f64 = s[1..].parse().map_err(|_| num_err(0))?;
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
  let sign = Sign::from_byte(s.as_bytes()[0]);
  let degrees: u16 = s[1..4].parse().map_err(|_| num_err(0))?;
  if degrees > 180 {
    return Err(ParseError::OutOfRange {
      component: Component::Longitude,
      value: degrees as f64,
    });
  }
  let minutes: f64 = s[4..].parse().map_err(|_| num_err(0))?;
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
  let sign = Sign::from_byte(s.as_bytes()[0]);
  let degrees: u16 = s[1..4].parse().map_err(|_| num_err(0))?;
  if degrees > 180 {
    return Err(ParseError::OutOfRange {
      component: Component::Longitude,
      value: degrees as f64,
    });
  }
  let minutes: u8 = s[4..6].parse().map_err(|_| num_err(0))?;
  if minutes >= 60 {
    return Err(ParseError::OutOfRange {
      component: Component::Minutes,
      value: minutes as f64,
    });
  }
  let seconds: f64 = s[6..].parse().map_err(|_| num_err(0))?;
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
  let sign = Sign::from_byte(s.as_bytes()[0]);
  let value: f64 = s[1..].parse().map_err(|_| num_err(0))?;
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
}
