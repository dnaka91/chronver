//! Different errors that can occur when interacting with types from this crate.

use std::{
    error::Error,
    fmt::{Debug, Display},
};

/// Errors that can occur when parsing raw strings into a [`Version`](crate::Version).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// The string contains invalid characters.
    NonAscii,
    /// The string was too short.
    TooShort,
    /// The _date_ component is invalid.
    InvalidDate(ParseDateError),
    /// The _changeset_ component is invalid.
    InvalidChangeset(ParseChangesetError),
    /// The _kind_ component is invalid.
    InvalidKind(ParseKindError),
    /// Unexpected trailing data.
    TrailingData,
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NonAscii | Self::TooShort | Self::TrailingData => None,
            Self::InvalidDate(inner) => Some(inner),
            Self::InvalidChangeset(inner) => Some(inner),
            Self::InvalidKind(inner) => Some(inner),
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonAscii => f.write_str("string contains non-ascii characters"),
            Self::TooShort => f.write_str("string is too short"),
            Self::InvalidDate(_) => f.write_str("invalid date component"),
            Self::InvalidChangeset(_) => f.write_str("invalid changeset component"),
            Self::InvalidKind(_) => f.write_str("invalid kind component"),
            Self::TrailingData => f.write_str("unexpected trailing data"),
        }
    }
}

impl From<ParseDateError> for ParseError {
    fn from(value: ParseDateError) -> Self {
        Self::InvalidDate(value)
    }
}

impl From<ParseChangesetError> for ParseError {
    fn from(value: ParseChangesetError) -> Self {
        Self::InvalidChangeset(value)
    }
}

impl From<ParseKindError> for ParseError {
    fn from(value: ParseKindError) -> Self {
        Self::InvalidKind(value)
    }
}

/// Errors that can occur when parsing raw strings into a [`Date`](crate::Date).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseDateError {
    /// Missing `.` separator for the month.
    MissingMonthSeparator,
    /// Missing `.` separator for the day.
    MissingDaySeparator,
    /// Malformed integer component.
    InvalidInt(InvalidIntError),
    /// Invalid month value.
    InvalidMonth(InvalidMonthError),
    /// Invalid date value.
    InvalidDate(InvalidDateError),
}

impl ParseDateError {
    /// Small helper to construct an [`Self::InvalidMonth`] error.
    pub(super) const fn invalid_month(inner: time::error::ComponentRange) -> Self {
        Self::InvalidMonth(InvalidMonthError(inner))
    }

    /// Small helper to construct an [`Self::InvalidDate`] error.
    pub(super) const fn invalid_date(inner: time::error::ComponentRange) -> Self {
        Self::InvalidDate(InvalidDateError(inner))
    }
}

impl Error for ParseDateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::MissingMonthSeparator | Self::MissingDaySeparator => None,
            Self::InvalidInt(inner) => Some(inner),
            Self::InvalidMonth(inner) => Some(inner),
            Self::InvalidDate(inner) => Some(inner),
        }
    }
}

impl Display for ParseDateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingMonthSeparator => f.write_str("missing separator for the month"),
            Self::MissingDaySeparator => f.write_str("missing separator for the day"),
            Self::InvalidInt(_) => f.write_str("malformed integer component"),
            Self::InvalidMonth(_) => f.write_str("invalid month value"),
            Self::InvalidDate(_) => f.write_str("invalid date value"),
        }
    }
}

impl From<std::num::ParseIntError> for ParseDateError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::InvalidInt(value.into())
    }
}

/// Failed parsing string into a valid date.
#[derive(Clone, Eq, PartialEq)]
pub struct InvalidDateError(time::error::ComponentRange);

impl Error for InvalidDateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl Display for InvalidDateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid date")
    }
}

impl Debug for InvalidDateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InvalidDateError").finish()
    }
}

/// Failed parsing numeric string into a valid month.
#[derive(Clone, Eq, PartialEq)]
pub struct InvalidMonthError(time::error::ComponentRange);

impl Error for InvalidMonthError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl Display for InvalidMonthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid month")
    }
}

impl Debug for InvalidMonthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InvalidMonthError").finish()
    }
}

/// Errors that can occur when parsing raw strings into a [`Changeset`](crate::Changeset).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseChangesetError {
    /// String is malformed.
    InvalidInt(InvalidIntError),
    /// Changeset value is zero.
    Zero,
}

impl Error for ParseChangesetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidInt(inner) => Some(inner),
            Self::Zero => None,
        }
    }
}

impl Display for ParseChangesetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInt(_) => f.write_str("string is malformed"),
            Self::Zero => f.write_str("changeset value is zero"),
        }
    }
}

impl From<std::num::ParseIntError> for ParseChangesetError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::InvalidInt(value.into())
    }
}

/// Failed parsing string into a valid integer.
#[derive(Clone, Eq, PartialEq)]
pub struct InvalidIntError(std::num::ParseIntError);

impl Error for InvalidIntError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl Display for InvalidIntError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid integer")
    }
}

impl Debug for InvalidIntError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InvalidIntError").finish()
    }
}

impl From<std::num::ParseIntError> for InvalidIntError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self(value)
    }
}

/// Errors that can occur when parsing raw strings into a [`Kind`](crate::Kind).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseKindError {
    /// String contains non-ascii characters.
    NonAscii,
}

impl Error for ParseKindError {}

impl Display for ParseKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonAscii => f.write_str("string contains non-ascii characters"),
        }
    }
}
