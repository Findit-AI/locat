use logos::Logos;

/// Tokens produced by the ISO 6709 DFA lexer.
///
/// Each `Signed*` variant matches a sign (`+`/`-`) followed by a fixed number
/// of integer digits and an optional decimal fraction. The digit count
/// determines the ISO 6709 component format:
///
/// | Variant | Pattern | ISO 6709 meaning (by position) |
/// |---------|---------|-------------------------------|
/// | `Signed2` | `±DD[.D+]` | Latitude degrees |
/// | `Signed3` | `±DDD[.D+]` | Longitude degrees |
/// | `Signed4` | `±DDMM[.M+]` | Latitude deg+min |
/// | `Signed5` | `±DDDMM[.M+]` | Longitude deg+min |
/// | `Signed6` | `±DDMMSS[.S+]` | Latitude DMS |
/// | `Signed7` | `±DDDMMSS[.S+]` | Longitude DMS |
/// | `SignedOther` | `±N+[.N+]` | Altitude (catch-all) |
#[derive(Logos, Debug, Clone, PartialEq)]
pub(crate) enum Token<'a> {
  /// `±DD` or `±DD.D+` — latitude degrees, or altitude.
  #[regex(r"[+-][0-9]{2}(\.[0-9]+)?", priority = 3)]
  Signed2(&'a str),

  /// `±DDD` or `±DDD.D+` — longitude degrees, or altitude.
  #[regex(r"[+-][0-9]{3}(\.[0-9]+)?", priority = 3)]
  Signed3(&'a str),

  /// `±DDMM` or `±DDMM.M+` — latitude deg+min, or altitude.
  #[regex(r"[+-][0-9]{4}(\.[0-9]+)?", priority = 3)]
  Signed4(&'a str),

  /// `±DDDMM` or `±DDDMM.M+` — longitude deg+min, or altitude.
  #[regex(r"[+-][0-9]{5}(\.[0-9]+)?", priority = 3)]
  Signed5(&'a str),

  /// `±DDMMSS` or `±DDMMSS.S+` — latitude DMS, or altitude.
  #[regex(r"[+-][0-9]{6}(\.[0-9]+)?", priority = 3)]
  Signed6(&'a str),

  /// `±DDDMMSS` or `±DDDMMSS.S+` — longitude DMS, or altitude.
  #[regex(r"[+-][0-9]{7}(\.[0-9]+)?", priority = 3)]
  Signed7(&'a str),

  /// Catch-all for signed numerics with digit counts outside 2–7
  /// (e.g. 1-digit or 8+ digit altitude values).
  #[regex(r"[+-][0-9]+(\.[0-9]+)?", priority = 1)]
  SignedOther(&'a str),

  /// CRS identifier, e.g. `CRSepsg4326`.
  #[regex(r"CRS[A-Za-z0-9]+")]
  CrsId(&'a str),

  /// Trailing solidus `/` terminator.
  #[token("/")]
  Solidus,
}

impl<'a> Token<'a> {
  /// Returns the raw string slice for any signed numeric token,
  /// or `None` for `CrsId`/`Solidus`.
  pub(crate) fn as_signed_str(&self) -> Option<&'a str> {
    match self {
      Self::Signed2(s)
      | Self::Signed3(s)
      | Self::Signed4(s)
      | Self::Signed5(s)
      | Self::Signed6(s)
      | Self::Signed7(s)
      | Self::SignedOther(s) => Some(s),
      _ => None,
    }
  }
}
