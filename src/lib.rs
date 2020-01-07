//! Chronologic version parsing.
//!
//! Chronologic versioning (see <https://chronver.org>) is a set of rules for assigning version
//! numbers.
//!
//! ## ChronVer overview
//!
//! Given a version number YEAR.MONTH.DAY.CHANGESET_IDENTIFIER, increment the:
//!
//! 1. YEAR version when the year changes,
//! 2. MONTH version when the month changes,
//! 3. DAY version when the day changes, and the
//! 4. CHANGESET_IDENTIFIER everytime you commit a change to your package/project.
//!
//! ## Versions
//!
//! ```
//! use chronver::Version;
//! use chrono::NaiveDate;
//!
//! assert!(Version::parse("2020.01.06") == Ok(Version {
//!     date: NaiveDate::from_ymd(2020, 1, 6),
//!     changeset: 0,
//!     label: None,
//! }));
//! ```
//!
//! ```
//! use chronver::Version;
//!
//! assert!(Version::parse("2020.01.06-alpha").unwrap() != Version::parse("2020.01.06-beta").unwrap());
//! assert!(Version::parse("2020.01.06-alpha").unwrap() > Version::parse("2020.01.06").unwrap());
//! ```
//!

#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::doc_markdown)]

use std::{convert::TryFrom, fmt::Display, str::FromStr};

use chrono::{Local, NaiveDate};
use thiserror::Error;

/// An error type for this crate.
#[derive(Error, Debug, Clone, Eq, PartialEq)]
pub enum ChronVerError {
    /// The version string was too short.
    #[error("Version string is too short")]
    TooShort,
    /// An error occurred while parsing the version component.
    #[error("Invalid version string")]
    InvalidVersion(#[from] chrono::ParseError),
    /// An error occurred while parsing the changeset component.
    #[error("Invalid changeset")]
    InvalidChangeset(#[from] std::num::ParseIntError),
    /// An error occurred while parsing the label component.
    #[error("Invalid label")]
    InvalidLabel,
}

/// Represents a version number conforming to the chronologic versioning scheme.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "&str"),
    serde(into = "String")
)]
pub struct Version {
    /// The date of release, to be updated whenever a new release is made on a different date than
    /// the last release.
    pub date: NaiveDate,
    /// The changeset number, to be incremented when a change was released on the same day.
    pub changeset: u32,
    /// The optional label, which can have any format or follow a branch formatting (see [`Label`]
    /// for more information).
    ///
    /// The special label `break` is reserved to signal a release with breaking changes.
    ///
    /// [`Label`]: enum.Label.html
    pub label: Option<Label>,
}

const DATE_LENGTH: usize = 10;
const DATE_FORMAT: &str = "%Y.%m.%d";

const BREAK_LABEL: &str = "break";

macro_rules! ensure {
    ($cond:expr, $err:expr $(,)?) => {
        if !$cond {
            return Err($err);
        }
    };
}

impl Version {
    /// Parse a string into a chronver object.
    pub fn parse(version: &str) -> Result<Self, ChronVerError> {
        ensure!(version.len() >= DATE_LENGTH, ChronVerError::TooShort);

        let date = NaiveDate::parse_from_str(&version[..DATE_LENGTH], DATE_FORMAT)
            .map_err(ChronVerError::from)?;

        let rem = &version[DATE_LENGTH..];

        let (changeset, label_pos) = if rem.starts_with('.') {
            let rem = &rem[1..];
            let end = rem
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or_else(|| rem.len());
            (rem[..end].parse().map_err(ChronVerError::from)?, end + 1)
        } else {
            ensure!(
                rem.is_empty() || rem.starts_with('-'),
                ChronVerError::InvalidLabel
            );
            (0, 0)
        };

        let rem = &rem[label_pos..];

        let label = if rem.starts_with('-') {
            Some(rem[1..].into())
        } else {
            ensure!(rem.is_empty(), ChronVerError::InvalidLabel);
            None
        };

        Ok(Self {
            date,
            changeset,
            label,
        })
    }

    /// Update the version to the current date or increment the changeset in case the date
    /// is the same. If a label exists, it will be removed.
    pub fn update(&mut self) {
        let new_date = Local::now().date().naive_local();
        if self.date == new_date {
            self.changeset += 1;
        } else {
            self.date = new_date;
            self.changeset = 0;
        }
        self.label = None;
    }

    /// Check whether the current version introduces breaking changes.
    #[must_use]
    pub fn is_breaking(&self) -> bool {
        if let Some(label) = &self.label {
            if let Label::Text(label) = &label {
                return label == BREAK_LABEL;
            }
        }
        false
    }
}

impl Default for Version {
    #[must_use]
    fn default() -> Self {
        Self {
            date: Local::now().date().naive_local(),
            changeset: 0,
            label: None,
        }
    }
}

impl FromStr for Version {
    type Err = ChronVerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.date.format(DATE_FORMAT).to_string())?;
        if self.changeset > 0 {
            write!(f, ".{}", self.changeset)?;
        }
        if let Some(label) = &self.label {
            write!(f, "-{}", label)?;
        }
        Ok(())
    }
}

impl From<NaiveDate> for Version {
    #[must_use]
    fn from(date: NaiveDate) -> Self {
        Self {
            date,
            changeset: 0,
            label: None,
        }
    }
}

impl From<(i32, u32, u32)> for Version {
    #[must_use]
    fn from(tuple: (i32, u32, u32)) -> Self {
        NaiveDate::from_ymd(tuple.0, tuple.1, tuple.2).into()
    }
}

impl TryFrom<&str> for Version {
    type Error = ChronVerError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Version> for String {
    #[inline]
    #[must_use]
    fn from(version: Version) -> Self {
        format!("{}", version)
    }
}

/// A label in the version metadata.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(from = "&str"),
    serde(into = "String")
)]
pub enum Label {
    /// A simple text label without a specific format.
    Text(String),
    /// A feature label in the format `BRANCH.CHANGESET`, where the changeset can be
    /// omitted when it is 0.
    Feature { branch: String, changeset: u32 },
}

impl Label {
    #[must_use]
    pub fn parse(label: &str) -> Self {
        if let Some(i) = label.rfind('.') {
            if let Ok(changeset) = label[i + 1..].parse() {
                return Self::Feature {
                    branch: label[..i].to_owned(),
                    changeset,
                };
            }
        }

        Self::Text(label.to_owned())
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Text(s) => f.write_str(s),
            Self::Feature { branch, changeset } => write!(f, "{}.{}", branch, changeset),
        }
    }
}

impl From<&str> for Label {
    #[inline]
    #[must_use]
    fn from(s: &str) -> Self {
        Self::parse(s)
    }
}

impl From<Label> for String {
    #[inline]
    #[must_use]
    fn from(label: Label) -> Self {
        format!("{}", label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_version() {
        let version = Version::parse("2019.01.06");
        assert_eq!(
            Version::from(NaiveDate::from_ymd(2019, 1, 6)),
            version.unwrap()
        );
    }

    #[test]
    fn with_changeset() {
        let version = Version::parse("2019.01.06.12");
        assert_eq!(
            Version {
                date: NaiveDate::from_ymd(2019, 1, 6),
                changeset: 12,
                label: None
            },
            version.unwrap()
        );
    }

    #[test]
    fn with_default_changeset() {
        let version = Version::parse("2019.01.06.0");
        assert_eq!(
            Version::from(NaiveDate::from_ymd(2019, 1, 6)),
            version.unwrap()
        );
    }

    #[test]
    fn with_label() {
        let version = Version::parse("2019.01.06-test");
        assert_eq!(
            Version {
                date: NaiveDate::from_ymd(2019, 1, 6),
                changeset: 0,
                label: Some(Label::Text("test".to_owned()))
            },
            version.unwrap()
        );
    }

    #[test]
    fn with_changeset_and_label() {
        let version = Version::parse("2019.01.06.1-test");
        assert_eq!(
            Version {
                date: NaiveDate::from_ymd(2019, 1, 6),
                changeset: 1,
                label: Some(Label::Text("test".to_owned()))
            },
            version.unwrap()
        );
    }

    #[test]
    fn with_default_changeset_and_label() {
        let version = Version::parse("2019.01.06.0-test");
        assert_eq!(
            Version {
                date: NaiveDate::from_ymd(2019, 1, 6),
                changeset: 0,
                label: Some(Label::Text("test".to_owned()))
            },
            version.unwrap()
        );
    }

    #[test]
    fn too_short() {
        let version = Version::parse("2019");
        assert_eq!(ChronVerError::TooShort, version.unwrap_err());
    }

    #[test]
    fn invalid_date() {
        let version = Version::parse("2019.30.01");
        assert!(match version.unwrap_err() {
            ChronVerError::InvalidVersion(_) => true,
            _ => false,
        });
    }

    #[test]
    fn invalid_changeset() {
        let version = Version::parse("2019.01.06+111");
        assert_eq!(ChronVerError::InvalidLabel, version.unwrap_err());
    }

    #[test]
    fn invalid_changeset_number() {
        let version = Version::parse("2019.01.06.a");
        assert!(match version.unwrap_err() {
            ChronVerError::InvalidChangeset(_) => true,
            _ => false,
        });
    }

    #[test]
    fn invalid_label() {
        let version = Version::parse("2019.01.06.1+test");
        assert_eq!(ChronVerError::InvalidLabel, version.unwrap_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialize() {
        let version = Version::parse("2019.01.06.1-test.2");
        assert_eq!(
            "\"2019.01.06.1-test.2\"",
            serde_json::to_string(&version.unwrap()).unwrap()
        );

        let version = Version::parse("2019.01.06.1-test");
        assert_eq!(
            "\"2019.01.06.1-test\"",
            serde_json::to_string(&version.unwrap()).unwrap()
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deserialize() {
        let version = Version::parse("2019.01.06.1-test.2");
        assert_eq!(
            serde_json::from_str::<Version>("\"2019.01.06.1-test.2\"").unwrap(),
            version.unwrap()
        );

        let version = Version::parse("2019.01.06.1-test");
        assert_eq!(
            serde_json::from_str::<Version>("\"2019.01.06.1-test\"").unwrap(),
            version.unwrap()
        );
    }
}
