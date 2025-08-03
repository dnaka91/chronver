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
//!     Version::try_from("2020.01.06").unwrap(),
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
//!     Version::try_from("2020.01.06-alpha").unwrap(),
//!     Version::try_from("2020.01.06-beta").unwrap()
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

pub mod error;

use std::{
    convert::TryFrom,
    fmt::{self, Display},
    num::NonZero,
    str::FromStr,
};

use time::OffsetDateTime;

use self::error::{ParseChangesetError, ParseDateError, ParseError, ParseKindError};

/// Represents a version number conforming to the chronologic versioning scheme.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "&str")
)]
pub struct Version {
    /// The date of release, to be updated whenever a new release is made on a different date than
    /// the last release.
    pub date: Date,
    /// The changeset number, to be incremented when a change was released on the same day.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub changeset: Option<Changeset>,
    /// The kind, which can have any format or follow a branch formatting. It describes the kind of
    /// release and carries further semantics.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Kind::is_regular"))]
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
    /// Increment the version to the current date or increment the changeset in case the date
    /// is the same. The [`Kind`] will be reset to [`Regular`](Kind::Regular).
    #[must_use]
    pub fn increment(&self) -> Self {
        let mut new = Self::default();
        if self.date == new.date {
            new.changeset = self.changeset.map_or_else(
                || Changeset::new(1),
                |cs| Some(cs.checked_add(1).unwrap_or(Changeset::MAX)),
            );
        }
        new
    }

    /// Check whether the current version introduces breaking changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use chronver::Version;
    ///
    /// assert!(Version::try_from("2020.03.05-break").unwrap().is_breaking());
    /// assert!(!Version::try_from("2020.03.05").unwrap().is_breaking());
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
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl TryFrom<&str> for Version {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        ensure!(value.is_ascii(), Self::Error::NonAscii);
        ensure!(value.len() >= DATE_LENGTH, Self::Error::TooShort);

        let (date, rem) = value.split_at(DATE_LENGTH);

        let (changeset, rem) = if let Some(rem) = rem.strip_prefix('.') {
            let pos = rem.find(|c: char| !c.is_ascii_digit()).unwrap_or(rem.len());
            let (changeset, rem) = rem.split_at(pos);
            (Some(changeset.parse()?), rem)
        } else {
            (None, rem)
        };

        let kind = if let Some(rem) = rem.strip_prefix('-') {
            rem.try_into()?
        } else {
            ensure!(rem.is_empty(), Self::Error::TrailingData);
            Kind::Regular
        };

        Ok(Self {
            date: date.parse()?,
            changeset,
            kind,
        })
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

/// The date which is the main component of a chronologic version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize),
    serde(try_from = "&str")
)]
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
    type Err = ParseDateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl TryFrom<&str> for Date {
    type Error = ParseDateError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (year, rem) = value
            .split_once('.')
            .ok_or(Self::Error::MissingMonthSeparator)?;
        let (month, day) = rem
            .split_once('.')
            .ok_or(Self::Error::MissingDaySeparator)?;

        let date = time::Date::from_calendar_date(
            year.parse()?,
            month
                .parse::<u8>()?
                .try_into()
                .map_err(Self::Error::invalid_month)?,
            day.parse()?,
        )
        .map_err(Self::Error::invalid_date)?;

        Ok(Self(date))
    }
}

impl From<time::Date> for Date {
    fn from(value: time::Date) -> Self {
        Self(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Date {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut ser = serializer.serialize_struct("Date", 3)?;
        ser.serialize_field("year", &self.0.year())?;
        ser.serialize_field("month", &u8::from(self.0.month()))?;
        ser.serialize_field("day", &self.0.day())?;
        ser.end()
    }
}

/// The changeset which is an incremental value in cases where multiple releases were done on the
/// same day.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize),
    serde(try_from = "&str")
)]
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
    type Err = ParseChangesetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl TryFrom<&str> for Changeset {
    type Error = ParseChangesetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        NonZero::new(value.parse()?)
            .ok_or(ParseChangesetError::Zero)
            .map(Self)
    }
}

impl Display for Changeset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Changeset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.get().serialize(serializer)
    }
}

/// The kind of release, usually [`Self::Regular`].
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize),
    serde(try_from = "&str")
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
    /// Tell whether this kind is [`Self::Regular`].
    const fn is_regular(&self) -> bool {
        matches!(self, Self::Regular)
    }
}

impl FromStr for Kind {
    type Err = ParseKindError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl TryFrom<&str> for Kind {
    type Error = ParseKindError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "" => Self::Regular,
            "break" => Self::Breaking,
            value if value.is_ascii() => Self::Feature {
                name: value.to_owned(),
            },
            _ => return Err(ParseKindError::NonAscii),
        })
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

#[cfg(feature = "serde")]
impl serde::Serialize for Kind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Regular => serializer.serialize_none(),
            Self::Breaking => serializer.serialize_some("break"),
            Self::Feature { name } => serializer.serialize_some(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use time::macros::date;

    use super::*;

    #[test]
    fn simple_version() {
        let version = Version::try_from("2019.01.06").unwrap();
        assert_eq!(Version::from(date!(2019 - 01 - 06)), version);
        assert_eq!("2019.01.06", version.to_string());
    }

    #[test]
    fn with_changeset() {
        let version = Version::try_from("2019.01.06.12").unwrap();
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06).into(),
                changeset: Changeset::new(12),
                kind: Kind::Regular,
            },
            version
        );
        assert_eq!("2019.01.06.12", version.to_string());
    }

    #[test]
    fn with_feature() {
        let version = Version::try_from("2019.01.06-test").unwrap();
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06).into(),
                changeset: None,
                kind: Kind::Feature {
                    name: "test".to_owned()
                }
            },
            version
        );
        assert_eq!("2019.01.06-test", version.to_string());
    }

    #[test]
    fn with_breaking() {
        let version = Version::try_from("2019.01.06-break").unwrap();
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06).into(),
                changeset: None,
                kind: Kind::Breaking,
            },
            version,
        );
        assert_eq!("2019.01.06-break", version.to_string());
    }

    #[test]
    fn with_changeset_and_feature() {
        let version = Version::try_from("2019.01.06.1-test").unwrap();
        assert_eq!(
            Version {
                date: date!(2019 - 01 - 06).into(),
                changeset: Changeset::new(1),
                kind: Kind::Feature {
                    name: "test".to_owned()
                }
            },
            version
        );
        assert_eq!("2019.01.06.1-test", version.to_string());
    }

    #[test]
    fn too_short() {
        let version = Version::try_from("2019");
        assert_eq!(ParseError::TooShort, version.unwrap_err());
    }

    #[test]
    fn invalid_date() {
        let version = Version::try_from("2019.30.01");
        assert!(matches!(version.unwrap_err(), ParseError::InvalidDate(_)));
    }

    #[test]
    fn invalid_changeset() {
        let version = Version::try_from("2019.01.06+111");
        assert_eq!(ParseError::TrailingData, version.unwrap_err());
    }

    #[test]
    fn invalid_changeset_number() {
        let version = Version::try_from("2019.01.06.a");
        assert!(matches!(
            version.unwrap_err(),
            ParseError::InvalidChangeset(_)
        ));
    }

    #[test]
    fn invalid_kind() {
        let version = Version::try_from("2019.01.06.1+test");
        assert_eq!(ParseError::TrailingData, version.unwrap_err());
    }

    #[test]
    fn increment_old() {
        let version = Version::try_from("2019.01.06").unwrap();
        assert_eq!(Version::default(), version.increment());
    }

    #[test]
    fn increment_same_date() {
        let version = Version::default();
        assert_eq!(
            Version {
                changeset: Changeset::new(1),
                ..Version::default()
            },
            version.increment()
        );
    }

    #[test]
    fn increment_same_date_twice() {
        let version = Version::default();
        assert_eq!(
            Version {
                changeset: Changeset::new(2),
                ..Version::default()
            },
            version.increment().increment()
        );
    }

    #[test]
    fn increment_breaking() {
        let version = Version::try_from("2019.01.06-break").unwrap();
        assert_eq!(Version::default(), version.increment());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialize() {
        let version = Version::try_from("2019.01.06.1-test");
        assert_eq!(
            serde_json::json!({
                "date": {
                    "year": 2019,
                    "month": 1,
                    "day": 6,
                },
                "changeset": 1,
                "kind": "test",
            }),
            serde_json::to_value(version.unwrap()).unwrap()
        );

        let version = Version::try_from("2019.01.06-break");
        assert_eq!(
            serde_json::json!({
                "date": {
                    "year": 2019,
                    "month": 1,
                    "day": 6,
                },
                "kind": "break",
            }),
            serde_json::to_value(version.unwrap()).unwrap()
        );

        let version = Version::try_from("2019.01.06");
        assert_eq!(
            serde_json::json!({
                "date": {
                    "year": 2019,
                    "month": 1,
                    "day": 6,
                },
            }),
            serde_json::to_value(version.unwrap()).unwrap()
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deserialize() {
        let version = Version::try_from("2019.01.06.1-test.2");
        assert_eq!(
            serde_json::from_str::<Version>("\"2019.01.06.1-test.2\"").unwrap(),
            version.unwrap()
        );

        let version = Version::try_from("2019.01.06.1-test");
        assert_eq!(
            serde_json::from_str::<Version>("\"2019.01.06.1-test\"").unwrap(),
            version.unwrap()
        );
    }
}
