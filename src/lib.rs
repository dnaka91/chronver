//! Chronologic version parsing.
//!
//! Chronologic versioning (see <https://chronver.org>) is a set of rules for assigning version
//! numbers.

#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]
#![warn(clippy::nursery)]

use anyhow::{ensure, Result};
use chrono::{Local, NaiveDate};
use thiserror::Error;

#[derive(Error, Debug, Clone, Eq, PartialEq)]
pub enum ChronVerError {
    #[error("Version string is too short")]
    TooShort,
    #[error("Invalid version string")]
    InvalidVersion(#[from] chrono::ParseError),
    #[error("Invalid changeset")]
    InvalidChangeset(#[from] std::num::ParseIntError),
    #[error("Invalid label")]
    InvalidLabel,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde_derive", derive(serde::Serialize, serde::Deserialize))]
pub struct Version {
    pub date: NaiveDate,
    pub changeset: u32,
    pub label: Option<Label>,
}

const DATE_LENGTH: usize = 10;
const DATE_FORMAT: &str = "%Y.%m.%d";

const BREAK_LABEL: &str = "break";

impl Version {
    pub fn parse(version: &str) -> Result<Self> {
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

    /// Checks whether the current version introduces breaking changes.
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

impl std::str::FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl std::fmt::Display for Version {
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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(
    feature = "serde_derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(untagged)
)]
pub enum Label {
    Text(String),
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

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Text(s) => f.write_str(s),
            Self::Feature { branch, changeset } => write!(f, "{}.{}", branch, changeset),
        }
    }
}

impl From<&str> for Label {
    #[must_use]
    fn from(s: &str) -> Self {
        Self::parse(s)
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
        assert_eq!(
            ChronVerError::TooShort,
            version.unwrap_err().downcast().unwrap()
        );
    }

    #[test]
    fn invalid_date() {
        let version = Version::parse("2019.30.01");
        assert!(match version.unwrap_err().downcast_ref::<ChronVerError>() {
            Some(ChronVerError::InvalidVersion(_)) => true,
            _ => false,
        });
    }

    #[test]
    fn invalid_changeset() {
        let version = Version::parse("2019.01.06+111");
        assert_eq!(
            ChronVerError::InvalidLabel,
            version.unwrap_err().downcast().unwrap()
        );
    }

    #[test]
    fn invalid_changeset_number() {
        let version = Version::parse("2019.01.06.a");
        assert!(match version.unwrap_err().downcast_ref::<ChronVerError>() {
            Some(ChronVerError::InvalidChangeset(_)) => true,
            _ => false,
        });
    }

    #[test]
    fn invalid_label() {
        let version = Version::parse("2019.01.06.1+test");
        assert_eq!(
            ChronVerError::InvalidLabel,
            version.unwrap_err().downcast().unwrap()
        );
    }

    #[cfg(feature = "serde_derive")]
    #[test]
    fn serialize() {
        let version = Version::parse("2019.01.06.1-test.2");
        println!(
            "{}",
            serde_json::to_string_pretty(&version.unwrap()).unwrap()
        );

        let version = Version::parse("2019.01.06.1-test");
        println!(
            "{}",
            serde_json::to_string_pretty(&version.unwrap()).unwrap()
        );
    }
}
