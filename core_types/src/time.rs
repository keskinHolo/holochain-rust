//! The Iso8601 type is defined here. It is used in particular within ChainHeader to enforce that
//! their timestamps are defined in a useful and consistent way.

use chrono::{offset::FixedOffset, DateTime};
use error::HolochainError;
use json::JsonString;
use regex::Regex;
use std::{cmp::Ordering, convert::TryFrom, fmt, time::Duration};

/// Represents a timeout for an HDK function
#[derive(Clone, Deserialize, Debug, Eq, PartialEq, Hash, Serialize, DefaultJson)]
pub struct Timeout(usize);

impl Timeout {
    pub fn new(timeout_ms: usize) -> Self {
        Self(timeout_ms)
    }
}

impl Default for Timeout {
    fn default() -> Timeout {
        Timeout(60000)
    }
}

impl From<Timeout> for Duration {
    fn from(Timeout(millis): Timeout) -> Duration {
        Duration::from_millis(millis as u64)
    }
}

impl From<&Timeout> for Duration {
    fn from(Timeout(millis): &Timeout) -> Duration {
        Duration::from_millis(*millis as u64)
    }
}

impl From<usize> for Timeout {
    fn from(millis: usize) -> Timeout {
        Timeout::new(millis)
    }
}

/// This struct represents datetime data stored as a string in the ISO 8601 and RFC 3339 (more
/// restrictive) format.
///
/// More info on the relevant [wikipedia article](https://en.wikipedia.org/wiki/ISO_8601).
#[derive(Clone, Serialize, Deserialize)]
pub struct Iso8601(String);

/*
 * Note that the WASM target does not have a reliable and consistent means to obtain the local time,
 * so all chrono related `now()` methods are not usable.  Therefore, we do not implement a
 * `Iso8601::default()` or `::now()` method at this time.  In addition, supporting internal
 * generated current timestamps is an easy path to non-determinism in holochain Zome functions.  All
 * times should be externally generated, and only *evaluated* by the Zome functions, not generated
 * by them.
 *
 * /// Iso8601::now() and default() return the current Utc time.
 * impl Iso8601 {
 *     pub fn now() -> Iso8601 {
 *         Iso8601::from(Utc::now().to_rfc3339())
 *     }
 * }
 *
 * impl Default for Iso8601 {
 *     fn default() -> Iso8601 {
 *         Iso8601::now()
 *     }
 * }
 */

impl fmt::Display for Iso8601 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

impl fmt::Debug for Iso8601 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match DateTime::<FixedOffset>::try_from(self) {
            Ok(dt) => {
                let ts = dt.to_rfc3339();
                if self.0 != ts {
                    write!(f, "Iso8601 {{ \"{}\" <- \"{}\" }}", ts, &self.0)
                } else {
                    write!(f, "Iso8601 {{ \"{}\" }}", &self.0)
                }
            }
            Err(e) => write!(f, "Iso8601 {{ \"{}\" -> {} }}", &self.0, e),
        }
    }
}

/// A static string is considered an infallible conversion; also unchecked infallible String conversion.
///
/// Since we receive Iso8601 from many remote, untrusted sources, we don't want to always force
/// checking at initial creation.  However, later on, we need to validate the timestamp, and we may
/// want to create a validated Iso8601 that we know will work in comparisons, etc.  For that, we use
/// the TryFrom method.
///
/// In the future, any invalid static-&str errors could (should?) produce a `panic!`.  This is
/// reasonable, as it would indicate an error in the code of the application, not in the logic.
impl From<&'static str> for Iso8601 {
    fn from(s: &str) -> Iso8601 {
        Iso8601(s.to_owned())
    }
}

impl From<String> for Iso8601 {
    fn from(s: String) -> Iso8601 {
        Iso8601(s)
    }
}

/// Conversion try_from on a &Iso8601 are fallible conversions, which may produce a HolochainError
/// if the timestamp is not valid ISO 8601 / RFC 3339.  We will allow some flexibilty; strip
/// surrounding whitespace, a bare timestamp missing any timezone specifier will be assumed to be
/// UTC "Zulu", make internal separators optional if unambiguous.  If you keep to straight RFC 3339
/// timestamps, then parsing will be quick, otherwise we'll employ a regular expression to parse a
/// more flexible subset of the ISO 8601 standard from your supplied timestamp, and then use the RFC
/// 3339 parser again.
impl TryFrom<&Iso8601> for DateTime<FixedOffset> {
    type Error = HolochainError;
    fn try_from(lhs: &Iso8601) -> Result<DateTime<FixedOffset>, Self::Error> {
        lazy_static! {
            static ref ISO8601_RE: Regex = Regex::new(
                r"(?x)
                ^
                \s*
                (?P<Y>\d{4})
                (?:            # Always require 4-digit year and double-digit mon/day YYYY[[-]MM[[-]DD]]
                  -?
                  (?P<M>
                     0[1-9]
                   | 1[012]
                  )?
                  (?:
                    -?
                    (?P<D>
                        0[1-9]
                      | [12][0-9]
                      | 3[01]
                    )?
                  )?
                )?
                (?:
                  (?:           # Optional T or space(s)
                    [Tt]
                  | \s+
                  )
                  (?P<h>        # Requires two-digit HH[[:]MM[[:]SS]] w/ consistent optional separators
                    [01][0-9]
                  | 2[0-3]      # but do not support 24:00:00 to designate end-of-day midnight
                  )
                  (?:
                    :?
                    (?P<m>
                      [0-5][0-9]
                    )
                    (?:         # The whole seconds group is optional, implies 00
                      :?
                      (?P<s>
                        (?:
                          [0-5][0-9]
                        | 60    # Support leap-seconds for standards compliance
                        )
                      )
                      (?:
                        [.,]    # Optional subseconds, separated by either ./, (always supply ., below)
                        (?P<ss>
                          \d+
                        )
                      )?
                    )?
                  )?
                )?
                \s*
                (?P<Z>          # no timezone specifier implies Z         
                   [Zz]
                 | (?P<Zsgn>[+-−]) # Zone sign allows UTF8 minus or ASCII hyphen as per RFC/ISO
                   (?P<Zhrs>\d{2}) # and always double-digit hours offset required
                   (?:             # but if double-digit minutes supplied, colon optional
                     :?
                     (?P<Zmin>\d{2})
                   )?
                )?
                \s*
                $"
            )
            .unwrap();
        }
        DateTime::parse_from_rfc3339(&lhs.0)
            .or_else(
                |_| ISO8601_RE.captures(&lhs.0)
                    .map_or_else(
                        || Err(HolochainError::ErrorGeneric(
                            format!("Failed to find ISO 3339 or RFC 8601 timestamp in {:?}", lhs.0))),
                        |cap| {
                            let timestamp = &format!(
                                "{:0>4}-{:0>2}-{:0>2}T{:0>2}:{:0>2}:{:0>2}{}{}",
                                &cap["Y"],
                                cap.name("M").map_or( "1", |m| m.as_str()),
                                cap.name("D").map_or( "1", |m| m.as_str()),
                                cap.name("h").map_or( "0", |m| m.as_str()),
                                cap.name("m").map_or( "0", |m| m.as_str()),
                                cap.name("s").map_or( "0", |m| m.as_str()),
                                cap.name("ss").map_or( "".to_string(), |m| format!(".{}", m.as_str())),
                                cap.name("Z").map_or( "Z".to_string(), |m| match m.as_str() {
                                    "Z"|"z" => "Z".to_string(),
                                    _ => format!(
                                        "{}{}:{}",
                                        match &cap["Zsgn"] { "+" => "+", _ => "-" },
                                        &cap["Zhrs"],
                                        &cap.name("Zmin").map_or( "00", |m| m.as_str()))
                                }));

                            DateTime::parse_from_rfc3339(timestamp)
                                .map_err(|_| HolochainError::ErrorGeneric(
                                    format!("Attempting to convert RFC 3339 timestamp {:?} from ISO 8601 {:?} to a DateTime",
                                            timestamp, lhs.0)))
                        }
                    )
            )
    }
}

/// PartialEq and PartialCmp for ISO 8601 / RFC 3339 timestamps w/ timezone specification.  Note
/// that two timestamps that differ in time specification may be equal, because they are the same
/// time specified in two different timezones.  Therefore, a String-based Partial{Cmp,Eq} are not
/// correct.  If conversion of any Iso8601 String fails, returns false for every test; similarly to
/// how float NaN != NaN.  However, to ease sorting, we'll also provide an Ord implementation that
/// orders any invalid Iso8601s as equal, before all valid Iso8601s.
impl PartialEq for Iso8601 {
    fn eq(&self, rhs: &Iso8601) -> bool {
        match DateTime::<FixedOffset>::try_from(self) {
            Ok(dt_lhs) => match DateTime::<FixedOffset>::try_from(rhs) {
                Ok(dt_rhs) => (&dt_lhs).eq(&dt_rhs),
                Err(_e) => false,
            },
            Err(_e) => false,
        }
    }
}

/// The PartialEq implements a total order, where all invalid Iso8601 are considered equal to
/// each-other; equally invalid.  Needed to implement Ord.
impl Eq for Iso8601 {}

impl PartialOrd for Iso8601 {
    fn partial_cmp(&self, rhs: &Iso8601) -> Option<Ordering> {
        match DateTime::<FixedOffset>::try_from(self) {
            Ok(ts_lhs) => match DateTime::<FixedOffset>::try_from(rhs) {
                Ok(ts_rhs) => (&ts_lhs).partial_cmp(&ts_rhs),
                Err(_e) => None,
            },
            Err(_e) => None,
        }
    }
}

// Invalid timestamps are "greater-than" any valid timestamp.  This puts them last in an in-order
// sort, first in a reverse sort.
impl Ord for Iso8601 {
    fn cmp(&self, rhs: &Iso8601) -> Ordering {
        match DateTime::<FixedOffset>::try_from(self) {
            Ok(ts_lhs) => match DateTime::<FixedOffset>::try_from(rhs) {
                Ok(ts_rhs) => ts_lhs.cmp(&ts_rhs),
                Err(_) => Ordering::Greater, // lhs is good, rhs is invalid; lhs is always > rhs (invalid)
            },
            Err(_) => match DateTime::<FixedOffset>::try_from(rhs) {
                Ok(_) => Ordering::Less, // lhs is invalid, rhs is valid; lhs (invalid) is always < rhs
                Err(_) => Ordering::Equal, // lhs and rhs both invalid; always equal-to each-other
            },
        }
    }
}

pub fn test_iso_8601() -> Iso8601 {
    Iso8601::from("2018-10-11T03:23:38+00:00")
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_iso_8601_basic() {
        // Different ways of specifying UTC "Zulu".  A bare timestamp will be defaulted to "Zulu".
        vec![
            "2018-10-11T03:23:38 +00:00",
            "2018-10-11T03:23:38Z",
            "2018-10-11T03:23:38",
            "2018-10-11T03:23:38+00",
            "2018-10-11 03:23:38",
        ]
        .iter()
        .map(|ts| {
            DateTime::<FixedOffset>::try_from(&Iso8601::from(*ts)).and_then(|ts| {
                Ok(assert_eq!(
                    format!("{}", ts.to_rfc3339()),
                    "2018-10-11T03:23:38+00:00"
                ))
            })
        })
        .collect::<Result<(()), HolochainError>>()
        .map_err(|e| {
            panic!(
                "Unexpected failure of checked DateTime<FixedOffset> try_from: {:?}",
                e
            )
        })
        .unwrap();

        vec![
            "20180101 0323",
            "2018-01-01 0323",
            "2018 0323",
            "2018-- 0323",
            "2018-01-01 032300",
            "2018-01-01 03:23",
            "2018-01-01 03:23:00",
            "2018-01-01 03:23:00 Z",
            "2018-01-01 03:23:00 +00",
            "2018-01-01 03:23:00 +00:00",
        ]
        .iter()
        .map(|ts| {
            DateTime::<FixedOffset>::try_from(&Iso8601::from(*ts)).and_then(|ts| {
                Ok(assert_eq!(
                    format!("{}", ts.to_rfc3339()),
                    "2018-01-01T03:23:00+00:00"
                ))
            })
        })
        .collect::<Result<(()), HolochainError>>()
        .map_err(|e| {
            panic!(
                "Unexpected failure of checked DateTime<FixedOffset> try_from: {:?}",
                e
            )
        })
        .unwrap();

        // Leap-seconds and sub-second times, in both native RFC 3339 and (Regex-based) ISO 8601.
        // Also exercise the HHMM60 methods for specifying times that extend into the following time
        // period.  Specifically does not support the "24:00:00" times.  Also tests the use of UTF8
        // minus in addition to ASCII hyphen.
        vec![
            "2015-02-18T23:59:60.234567-05:00",
            "2015-02-18T23:59:60.234567−05:00",
            "2015-02-18 235960.234567 -05",
            "20150218 235960.234567 −05",
            "20150218 235960,234567 −05",
        ]
        .iter()
        .map(|ts| {
            DateTime::<FixedOffset>::try_from(&Iso8601::from(*ts)).and_then(|ts| {
                Ok(assert_eq!(
                    format!("{}", ts.to_rfc3339()),
                    "2015-02-18T23:59:60.234567-05:00"
                ))
            })
        })
        .collect::<Result<(()), HolochainError>>()
        .map_err(|e| {
            panic!(
                "Unexpected failure of checked DateTime<FixedOffset> try_from: {:?}",
                e
            )
        })
        .unwrap();

        // Now test a bunch that should fail
        vec![
            "boo",
            "2015-02-18T23:59:60.234567-5",
            "2015-02-18 3:59:60-05",
            "2015-2-18 03:59:60-05",
            "2015-2-18 03:59:60+25",
        ]
        .iter()
        .map(
            |ts| match DateTime::<FixedOffset>::try_from(&Iso8601::from(*ts)) {
                Ok(dt) => Err(HolochainError::ErrorGeneric(format!(
                    "Should not have succeeded in parsing {:?} into {:?}",
                    ts, dt
                ))),
                Err(_) => Ok(()),
            },
        )
        .collect::<Result<(()), HolochainError>>()
        .map_err(|e| {
            panic!(
                "Unexpected success of invalid checked DateTime<FixedOffset> try_from: {:?}",
                e
            )
        })
        .unwrap();

        // PartialEq and PartialOrd Comparison operators
        assert!(
            Iso8601::from("2018-10-11T03:23:38+00:00") == Iso8601::from("2018-10-11T03:23:38Z")
        );
        assert!(Iso8601::from("2018-10-11T03:23:38") == Iso8601::from("2018-10-11T03:23:38Z"));
        assert!(Iso8601::from(" 20181011  0323  Z ") == Iso8601::from("2018-10-11T03:23:00Z"));

        // Fixed-offset ISO 8601 are comparable to UTC times
        assert!(
            Iso8601::from("2018-10-11T03:23:38-08:00") == Iso8601::from("2018-10-11T11:23:38Z")
        );
        assert!(Iso8601::from("2018-10-11T03:23:39-08:00") > Iso8601::from("2018-10-11T11:23:38Z"));
        assert!(Iso8601::from("2018-10-11T03:23:37-08:00") < Iso8601::from("2018-10-11T11:23:38Z"));

        // Ensure PartialOrd respects persistent inequality of invalid ISO 8601 DateTime strings
        // TODO: Since we're potentially validating all Iso8601 w/ DateTime<Utc> upon creation, we
        // can use Eq/Ord instead of ParialEq/PartialOrd.  For now, we allow invalid Iso8601 data.
        assert!(Iso8601::from("boo") != Iso8601::from("2018-10-11T03:23:38Z"));
        assert!(Iso8601::from("2018-10-11T03:23:38Z") != Iso8601::from("boo"));
        assert!(Iso8601::from("boo") != Iso8601::from("boo"));
        assert!(!(Iso8601::from("2018-10-11T03:23:38Z") < Iso8601::from("boo")));
        assert!(!(Iso8601::from("boo") < Iso8601::from("2018-10-11T03:23:38Z")));
        assert!(!(Iso8601::from("boo") < Iso8601::from("boo")));

        match DateTime::<FixedOffset>::try_from(&Iso8601::from("boo")) {
            Ok(ts) => panic!(
                "Unexpected success of checked DateTime<FixedOffset> try_from: {:?}",
                &ts
            ),
            Err(e) => assert_eq!(
                format!("{}", e),
                "Failed to find ISO 3339 or RFC 8601 timestamp in \"boo\""
            ),
        }
    }

    #[test]
    fn test_iso_8601_sorting() {
        // Different ways of specifying UTC "Zulu".  A bare timestamp will be defaulted to "Zulu".
        let mut v: Vec<Iso8601> = vec![
            "2018-10-11T03:23:39-08:00".into(),
            "2018-10-11T03:23:39-07:00".into(),
            "2018-10-11 03:23:39+03:00".into(),
            "baz".into(),
            "2018-10-11T03:23:39-06:00".into(),
            "20181011 032339 +04:00".into(),
            "2018-10-11T03:23:39−09:00".into(), // note the UTF8 minus instead of ASCII hyphen
            "2018-10-11T03:23:39+11:00".into(),
            "2018-10-11 03:23:39Z".into(),
            "2018-10-11 03:23:40".into(),
            "boo".into(),
            "bar".into(),
        ];
        v.sort_by(|a, b| {
            let cmp = a.cmp(b);
            //println!( "{} {:?} {}", a, cmp, b );
            cmp
        });
        assert_eq!(
            v.iter()
                .map(|ts| format!("{:?}", &ts).to_string())
                .collect::<Vec<String>>()
                .join(", "),
            concat!(
                "Iso8601 { \"baz\" -> Failed to find ISO 3339 or RFC 8601 timestamp in \"baz\" }, ",
                "Iso8601 { \"boo\" -> Failed to find ISO 3339 or RFC 8601 timestamp in \"boo\" }, ",
                "Iso8601 { \"bar\" -> Failed to find ISO 3339 or RFC 8601 timestamp in \"bar\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39+11:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39+04:00\" <- \"20181011 032339 +04:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39+03:00\" <- \"2018-10-11 03:23:39+03:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39+00:00\" <- \"2018-10-11 03:23:39Z\" }, ",
                "Iso8601 { \"2018-10-11T03:23:40+00:00\" <- \"2018-10-11 03:23:40\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39-06:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39-07:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39-08:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39-09:00\" <- \"2018-10-11T03:23:39−09:00\" }"
            )
        );

        v.sort_by(|a, b| b.cmp(a)); // reverse
        assert_eq!(
            v.iter()
                .map(|ts| format!("{:?}", &ts).to_string())
                .collect::<Vec<String>>()
                .join(", "),
            concat!(
                "Iso8601 { \"2018-10-11T03:23:39-09:00\" <- \"2018-10-11T03:23:39−09:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39-08:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39-07:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39-06:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:40+00:00\" <- \"2018-10-11 03:23:40\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39+00:00\" <- \"2018-10-11 03:23:39Z\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39+03:00\" <- \"2018-10-11 03:23:39+03:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39+04:00\" <- \"20181011 032339 +04:00\" }, ",
                "Iso8601 { \"2018-10-11T03:23:39+11:00\" }, ",
                "Iso8601 { \"baz\" -> Failed to find ISO 3339 or RFC 8601 timestamp in \"baz\" }, ",
                "Iso8601 { \"boo\" -> Failed to find ISO 3339 or RFC 8601 timestamp in \"boo\" }, ",
                "Iso8601 { \"bar\" -> Failed to find ISO 3339 or RFC 8601 timestamp in \"bar\" }"
            )
        );
    }
}
