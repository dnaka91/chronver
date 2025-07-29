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
//!         date: date!(2020 - 01 - 06).into(),
//!         changeset: None,
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
    num::NonZero,
    str::FromStr,
};

use thiserror::Error;
use time::OffsetDateTime;

/// An error type for this crate.
#[derive(Error, Debug, Clone, Eq, PartialEq)]
pub enum ChronVerError {
    /// The version string contains invalid characters.
    #[error("Version string contains non-ascii characters")]
    NonAscii,
    /// The version string was too short.
    #[error("Version string is too short")]
    TooShort,
    /// An error occurred while parsing the version component.
    #[error("Invalid version string")]
    InvalidVersion,
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
    pub changeset: Option<Changeset>,
    /// The kind, which can have any format or follow a branch formatting. It describes the kind of
    /// release and carries further semantics.
    pub kind: Kind,
}

/// Minimum length that a version must have to be further processed.
const DATE_LENGTH: usize = 10;

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
    /// use chronver::{Changeset, Kind, Version};
    /// use time::macros::date;
    ///
    /// // Basic version with just a date
    /// assert_eq!(
    ///     Version::parse("2020.03.05"),
    ///     Ok(Version {
    ///         date: date!(2020 - 03 - 05).into(),
    ///         changeset: None,
    ///         kind: Kind::Regular,
    ///     })
    /// );
    ///
    /// // Version with a changeset
    /// assert_eq!(
    ///     Version::parse("2020.03.05.2"),
    ///     Ok(Version {
    ///         date: date!(2020 - 03 - 05).into(),
    ///         changeset: Changeset::new(2),
    ///         kind: Kind::Regular,
    ///     })
    /// );
    ///
    /// // And with feature
    /// assert_eq!(
    ///     Version::parse("2020.03.05.2-new"),
    ///     Ok(Version {
    ///         date: date!(2020 - 03 - 05).into(),
    ///         changeset: Changeset::new(2),
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
        ensure!(version.is_ascii(), ChronVerError::NonAscii);
        ensure!(version.len() >= DATE_LENGTH, ChronVerError::TooShort);

        let date = version[..DATE_LENGTH].parse()?;
        let rem = &version[DATE_LENGTH..];

        let (changeset, kind_pos) = if let Some(rem) = rem.strip_prefix('.') {
            let end = rem.find(|c: char| !c.is_ascii_digit()).unwrap_or(rem.len());
            let changeset = rem[..end].parse().map_err(ChronVerError::from)?;
            (Changeset::new(changeset), end + 1)
        } else {
            ensure!(
                rem.is_empty() || rem.starts_with('-'),
                ChronVerError::InvalidKind
            );
            (None, 0)
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
        let new_date = OffsetDateTime::now_utc().date().into();
        if self.date == new_date {
            self.changeset = self.changeset.map_or_else(
                || Changeset::new(1),
                |cs| Some(cs.checked_add(1).unwrap_or(Changeset::MAX)),
            );
        } else {
            self.date = new_date;
            self.changeset = None;
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
            date: OffsetDateTime::now_utc().date().into(),
            changeset: None,
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
        write!(f, "{}", self.date)?;

        if let Some(changeset) = self.changeset {
            write!(f, ".{changeset}")?;
        }

        if !matches!(self.kind, Kind::Regular) {
            write!(f, "-{}", self.kind)?;
        }

        Ok(())
    }
}

impl From<time::Date> for Version {
    fn from(date: time::Date) -> Self {
        Self {
            date: date.into(),
            changeset: None,
            kind: Kind::Regular,
        }
    }
}

impl TryFrom<(i32, time::Month, u8)> for Version {
    type Error = ChronVerError;

    fn try_from(tuple: (i32, time::Month, u8)) -> Result<Self, Self::Error> {
        time::Date::from_calendar_date(tuple.0, tuple.1, tuple.2)
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

/// The date which is the main component of a chronologic version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Date(time::Date);

impl Date {
    /// Get the year component of the date.
    ///
    /// # Example
    ///
    /// ```
    /// let date = "2020.01.06".parse::<chronver::Date>().unwrap();
    /// assert_eq!(2020, date.year());
    /// ```
    #[must_use]
    pub const fn year(&self) -> i32 {
        self.0.year()
    }

    /// Get the month component of the date.
    ///
    /// # Example
    ///
    /// ```
    /// let date = "2020.01.06".parse::<chronver::Date>().unwrap();
    /// assert_eq!(1, date.month());
    /// ```
    #[must_use]
    pub const fn month(&self) -> u8 {
        self.0.month() as u8
    }

    /// Get the day component of the date.
    ///
    /// # Example
    ///
    /// ```
    /// let date = "2020.01.06".parse::<chronver::Date>().unwrap();
    /// assert_eq!(6, date.day());
    /// ```
    #[must_use]
    pub const fn day(&self) -> u8 {
        self.0.day()
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}.{:02}.{:02}",
            self.0.year(),
            u8::from(self.0.month()),
            self.0.day()
        )
    }
}

impl FromStr for Date {
    type Err = ChronVerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (year, rem) = s.split_once('.').ok_or(ChronVerError::InvalidVersion)?;
        let (month, day) = rem.split_once('.').ok_or(ChronVerError::InvalidVersion)?;

        let date = time::Date::from_calendar_date(
            year.parse()?,
            month.parse::<u8>()?.try_into()?,
            day.parse()?,
        )?;

        Ok(Self(date))
    }
}

impl TryFrom<&str> for Date {
    type Error = ChronVerError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<time::Date> for Date {
    fn from(value: time::Date) -> Self {
        Self(value)
    }
}

/// The changeset which is an incremental value in cases where multiple releases were done on the
/// same day.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Changeset(NonZero<u32>);

impl Changeset {
    /// The maximum possible value for a changeset.
    const MAX: Self = Self(NonZero::<u32>::MAX);

    /// Try crate a new changeset version from a raw [`u32`]. Changesets must be positive numbers or
    /// are omitted. Therefore, passing the literal `0` will yield `None` as return value.
    #[must_use]
    pub const fn new(value: u32) -> Option<Self> {
        match NonZero::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Get the raw changeset value.
    ///
    /// # Example
    ///
    /// ```
    /// let cs = chronver::Changeset::new(1).unwrap();
    /// assert_eq!(1, cs.get());
    /// ```
    #[must_use]
    pub const fn get(&self) -> u32 {
        self.0.get()
    }

    /// Perform a checked add on the changeset, which avoids wrapping around boundaries of the
    /// underlying raw `u32` value.
    const fn checked_add(self, value: u32) -> Option<Self> {
        match self.0.checked_add(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl FromStr for Changeset {
    type Err = ChronVerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NonZero::new(s.parse()?)
            .ok_or(ChronVerError::TooShort)
            .map(Self)
    }
}

impl Display for Changeset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
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
                date: date!(2019 - 01 - 06).into(),
                changeset: Changeset::new(12),
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
                date: date!(2019 - 01 - 06).into(),
                changeset: None,
                kind: Kind::Feature {
                    name: "test".to_owned()
                }
            },
            version.unwrap()
        );
    }

    #[test]
    fn with_changeset_and_label() {
        let version = Version::parse("2019.01.06.1-test");
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06).into(),
                changeset: Changeset::new(1),
                kind: Kind::Feature {
                    name: "test".to_owned()
                }
            },
            version.unwrap()
        );
    }

    #[test]
    fn with_default_changeset_and_feature() {
        let version = Version::parse("2019.01.06.0-test");
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06).into(),
                changeset: None,
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
            ChronVerError::InvalidComponents(_)
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
