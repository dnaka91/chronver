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
//! A simple `Version` can be constructed by using the `parse` method:
//!
//! ```
//! use chronver::Version;
//! use time::macros::date;
//!
//! assert!(Version::parse("2020.01.06") == Ok(Version {
//!     date: date!(2020-01-06),
//!     changeset: 0,
//!     label: None,
//! }));
//! ```
//!
//! Versions can also be compared with each other:
//!
//! ```
//! use chronver::Version;
//!
//! assert_ne!(
//!     Version::parse("2020.01.06-alpha").unwrap(),
//!     Version::parse("2020.01.06-beta").unwrap()
//! );
//! ```
//!

#![doc(html_root_url = "https://docs.rs/chronver/0.1.0")]
#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(
    missing_docs,
    rustdoc::missing_doc_code_examples,
    clippy::missing_docs_in_private_items
)]

use std::{
    convert::TryFrom,
    fmt::{self, Display},
    str::FromStr,
};

use thiserror::Error;
use time::{format_description::FormatItem, macros::format_description, OffsetDateTime};
pub use time::{Date, Month};

/// An error type for this crate.
#[derive(Error, Debug, Clone, Eq, PartialEq)]
pub enum ChronVerError {
    /// The version string was too short.
    #[error("Version string is too short")]
    TooShort,
    /// An error occurred while parsing the version component.
    #[error("Invalid version string")]
    InvalidVersion(#[from] time::error::Parse),
    /// An error occurred while constructing an version from date components.
    #[error("Invalid date components")]
    InvalidComponents(#[from] time::error::ComponentRange),
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
    pub date: Date,
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

/// Minimum length that a version must have to be further processed.
const DATE_LENGTH: usize = 10;
/// Format for the date part of a version.
const DATE_FORMAT: &[FormatItem<'static>] = format_description!("[year].[month].[day]");
/// The special label to decide whether the version introduces breaking changes.
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
    ///
    /// # Examples
    ///
    /// ```
    /// use chronver::{Version, Label};
    /// use time::macros::date;
    ///
    /// // Basic version with just a date
    /// assert_eq!(Version::parse("2020.03.05"), Ok(Version {
    ///     date: date!(2020-03-05),
    ///     changeset: 0,
    ///     label: None,
    /// }));
    ///
    /// // Version with a changeset
    /// assert_eq!(Version::parse("2020.03.05.2"), Ok(Version {
    ///     date: date!(2020-03-05),
    ///     changeset: 2,
    ///     label: None,
    /// }));
    ///
    /// // And with label
    /// assert_eq!(Version::parse("2020.03.05.2-new"), Ok(Version {
    ///     date: date!(2020-03-05),
    ///     changeset: 2,
    ///     label: Some(Label::Text("new".to_owned())),
    /// }));
    /// ```
    ///
    /// # Errors
    ///
    /// An error can occur in two cases. First, when the very first part of the version is not a
    /// valid date in the format `YYYY.MM.DD`. Second, when a **changeset** follows the date but
    /// it is not a valid `u32` number.
    pub fn parse(version: &str) -> Result<Self, ChronVerError> {
        ensure!(version.len() >= DATE_LENGTH, ChronVerError::TooShort);

        let date =
            Date::parse(&version[..DATE_LENGTH], &DATE_FORMAT).map_err(ChronVerError::from)?;

        let rem = &version[DATE_LENGTH..];

        let (changeset, label_pos) = if let Some(rem) = rem.strip_prefix('.') {
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

        let label = if let Some(rem) = rem.strip_prefix('-') {
            Some(rem.into())
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
        let new_date = OffsetDateTime::now_utc().date();
        if self.date == new_date {
            self.changeset += 1;
        } else {
            self.date = new_date;
            self.changeset = 0;
        }
        self.label = None;
    }

    /// Check whether the current version introduces breaking changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use chronver::Version;
    ///
    /// assert!(Version::parse("2020.03.05-break").unwrap().is_breaking());
    /// assert!(!Version::parse("2020.03.05").unwrap().is_breaking());
    /// ```
    #[must_use]
    pub fn is_breaking(&self) -> bool {
        if let Some(Label::Text(label)) = &self.label {
            return label == BREAK_LABEL;
        }
        false
    }
}

impl Default for Version {
    #[must_use]
    fn default() -> Self {
        Self {
            date: OffsetDateTime::now_utc().date(),
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.date.format(&DATE_FORMAT).map_err(|_| fmt::Error)?)?;
        if self.changeset > 0 {
            write!(f, ".{}", self.changeset)?;
        }
        if let Some(label) = &self.label {
            write!(f, "-{}", label)?;
        }
        Ok(())
    }
}

impl From<Date> for Version {
    #[must_use]
    fn from(date: Date) -> Self {
        Self {
            date,
            changeset: 0,
            label: None,
        }
    }
}

impl TryFrom<(i32, Month, u8)> for Version {
    type Error = ChronVerError;

    fn try_from(tuple: (i32, Month, u8)) -> Result<Self, Self::Error> {
        Date::from_calendar_date(tuple.0, tuple.1, tuple.2)
            .map(Self::from)
            .map_err(Into::into)
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
    Feature {
        /// Name of the feature branch.
        branch: String,
        /// Changeset number, omitted if 0.
        changeset: u32,
    },
}

impl Label {
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use chronver::Label;
    ///
    /// assert_eq!(Label::parse("test"), Label::Text("test".to_owned()));
    /// assert_eq!(Label::parse("feature.1"), Label::Feature {
    ///     branch: "feature".to_owned(),
    ///     changeset: 1,
    /// });
    /// ```
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    use time::macros::date;

    use super::*;

    #[test]
    fn simple_version() {
        let version = Version::parse("2019.01.06");
        assert_eq!(Version::from(date!(2019 - 01 - 06)), version.unwrap());
    }

    #[test]
    fn with_changeset() {
        let version = Version::parse("2019.01.06.12");
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06),
                changeset: 12,
                label: None
            },
            version.unwrap()
        );
    }

    #[test]
    fn with_default_changeset() {
        let version = Version::parse("2019.01.06.0");
        assert_eq!(Version::from(date!(2019 - 01 - 06)), version.unwrap());
    }

    #[test]
    fn with_label() {
        let version = Version::parse("2019.01.06-test");
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06),
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
                date: date!(2019 - 01 - 06),
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
                date: date!(2019 - 01 - 06),
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
        assert!(matches!(
            version.unwrap_err(),
            ChronVerError::InvalidVersion(_)
        ));
    }

    #[test]
    fn invalid_changeset() {
        let version = Version::parse("2019.01.06+111");
        assert_eq!(ChronVerError::InvalidLabel, version.unwrap_err());
    }

    #[test]
    fn invalid_changeset_number() {
        let version = Version::parse("2019.01.06.a");
        assert!(matches!(
            version.unwrap_err(),
            ChronVerError::InvalidChangeset(_)
        ));
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
