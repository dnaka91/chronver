//! Chronologic version parsing.
//!
//! Chronologic versioning (see <https://chronver.org>) is a set of rules for assigning version
//! numbers.
//!
//! ## `ChronVer` overview
//!
//! Given a version number `YEAR.MONTH.DAY.CHANGESET_IDENTIFIER`, increment the:
//!
//! 1. YEAR version when the year changes,
//! 2. MONTH version when the month changes,
//! 3. DAY version when the day changes, and the
//! 4. `CHANGESET_IDENTIFIER` everytime you commit a change to your package/project.
//!
//! ## Versions
//!
//! A simple `Version` can be constructed by using the `parse` method:
//!
//! ```
//! use chronver::{Kind, Version};
//! use time::macros::date;
//!
//! assert_eq!(
//!     Version::parse("2020.01.06").unwrap(),
//!     Version {
//!         date: date!(2020 - 01 - 06),
//!         changeset: 0,
//!         kind: Kind::Regular,
//!     }
//! );
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

#![forbid(unsafe_code)]
#![deny(
    rust_2018_idioms,
    rust_2024_compatibility,
    clippy::all,
    clippy::pedantic
)]
#![warn(clippy::nursery)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

use std::{
    convert::{Infallible, TryFrom},
    fmt::{self, Display},
    str::FromStr,
};

use thiserror::Error;
pub use time::{Date, Month};
use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};

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
    /// An error occurred while parsing the feature component.
    #[error("Invalid kind")]
    InvalidKind,
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
    /// The kind, which can have any format or follow a branch formatting. It describes the kind of
    /// release and carries further semantics.
    pub kind: Kind,
}

/// Minimum length that a version must have to be further processed.
const DATE_LENGTH: usize = 10;
/// Format for the date part of a version.
const DATE_FORMAT: &[FormatItem<'static>] = format_description!("[year].[month].[day]");

/// Shorthand to return an error when a condition is invalid.
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
    /// use chronver::{Kind, Version};
    /// use time::macros::date;
    ///
    /// // Basic version with just a date
    /// assert_eq!(
    ///     Version::parse("2020.03.05"),
    ///     Ok(Version {
    ///         date: date!(2020 - 03 - 05),
    ///         changeset: 0,
    ///         kind: Kind::Regular,
    ///     })
    /// );
    ///
    /// // Version with a changeset
    /// assert_eq!(
    ///     Version::parse("2020.03.05.2"),
    ///     Ok(Version {
    ///         date: date!(2020 - 03 - 05),
    ///         changeset: 2,
    ///         kind: Kind::Regular,
    ///     })
    /// );
    ///
    /// // And with feature
    /// assert_eq!(
    ///     Version::parse("2020.03.05.2-new"),
    ///     Ok(Version {
    ///         date: date!(2020 - 03 - 05),
    ///         changeset: 2,
    ///         kind: Kind::Feature {
    ///             name: "new".to_owned()
    ///         },
    ///     })
    /// );
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

        let (changeset, kind_pos) = if let Some(rem) = rem.strip_prefix('.') {
            let end = rem.find(|c: char| !c.is_ascii_digit()).unwrap_or(rem.len());
            (rem[..end].parse().map_err(ChronVerError::from)?, end + 1)
        } else {
            ensure!(
                rem.is_empty() || rem.starts_with('-'),
                ChronVerError::InvalidKind
            );
            (0, 0)
        };

        let rem = &rem[kind_pos..];

        let kind = if let Some(rem) = rem.strip_prefix('-') {
            rem.into()
        } else {
            ensure!(rem.is_empty(), ChronVerError::InvalidKind);
            Kind::Regular
        };

        Ok(Self {
            date,
            changeset,
            kind,
        })
    }

    /// Update the version to the current date or increment the changeset in case the date
    /// is the same. The [`Kind`] will be reset to [`Regular`](Kind::Regular).
    pub fn update(&mut self) {
        let new_date = OffsetDateTime::now_utc().date();
        if self.date == new_date {
            self.changeset += 1;
        } else {
            self.date = new_date;
            self.changeset = 0;
        }
        self.kind = Kind::Regular;
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
    pub const fn is_breaking(&self) -> bool {
        matches!(self.kind, Kind::Breaking)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self {
            date: OffsetDateTime::now_utc().date(),
            changeset: 0,
            kind: Kind::default(),
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.date.format(&DATE_FORMAT).map_err(|_| fmt::Error)?)?;
        if self.changeset > 0 {
            write!(f, ".{}", self.changeset)?;
        }

        if !matches!(self.kind, Kind::Regular) {
            write!(f, "-{}", self.kind)?;
        }

        Ok(())
    }
}

impl From<Date> for Version {
    fn from(date: Date) -> Self {
        Self {
            date,
            changeset: 0,
            kind: Kind::Regular,
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

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Version> for String {
    fn from(version: Version) -> Self {
        format!("{version}")
    }
}

/// The kind of release, usually [`Self::Regular`].
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(from = "&str"),
    serde(into = "String")
)]
pub enum Kind {
    /// Normal release without any extras.
    #[default]
    Regular,
    /// Breaking changes included.
    Breaking,
    /// Feature release that is usually tied to some Git branch and not considered fully stable.
    Feature {
        /// Name of the feature.
        name: String,
    },
}

impl Kind {
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use chronver::Kind;
    ///
    /// assert_eq!(Kind::parse("break"), Kind::Breaking);
    /// assert_eq!(
    ///     Kind::parse("feature"),
    ///     Kind::Feature {
    ///         name: "feature".to_owned(),
    ///     }
    /// );
    /// ```
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "" => Self::Regular,
            "break" => Self::Breaking,
            _ => Self::Feature {
                name: value.to_owned(),
            },
        }
    }
}

impl Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Regular => Ok(()),
            Self::Breaking => f.write_str("break"),
            Self::Feature { name } => f.write_str(name),
        }
    }
}

impl FromStr for Kind {
    type Err = Infallible;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::parse(s))
    }
}

impl From<&str> for Kind {
    #[inline]
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<Kind> for String {
    #[inline]
    fn from(value: Kind) -> Self {
        format!("{value}")
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
                kind: Kind::Regular,
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
    fn with_feature() {
        let version = Version::parse("2019.01.06-test");
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06),
                changeset: 0,
                kind: Kind::Feature {
                    name: "test".to_owned()
                }
            },
            version.unwrap()
        );
    }

    #[test]
    fn with_changeset_and_kind() {
        let version = Version::parse("2019.01.06.1-test");
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06),
                changeset: 1,
                kind: Kind::Feature {
                    name: "test".to_owned()
                }
            },
            version.unwrap()
        );
    }

    #[test]
    fn with_default_changeset_and_kind() {
        let version = Version::parse("2019.01.06.0-test");
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06),
                changeset: 0,
                kind: Kind::Feature {
                    name: "test".to_owned()
                }
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
        assert_eq!(ChronVerError::InvalidKind, version.unwrap_err());
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
    fn invalid_kind() {
        let version = Version::parse("2019.01.06.1+test");
        assert_eq!(ChronVerError::InvalidKind, version.unwrap_err());
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
