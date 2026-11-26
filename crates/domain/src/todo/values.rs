use std::{fmt, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TodoId(Uuid);

impl TodoId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl FromStr for TodoId {
    type Err = uuid::Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(raw).map(Self)
    }
}

impl fmt::Display for TodoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("Title must be between 1 and 100 characters")]
    Title,
    #[error("Description must be between 1 and 1000 characters")]
    Description,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Title(String);

impl TryFrom<String> for Title {
    type Error = ValidationError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        if raw.is_empty() || raw.chars().count() > 100 {
            return Err(ValidationError::Title);
        }
        Ok(Self(raw))
    }
}

impl Title {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for Title {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Description(String);

impl Description {
    /// Normalize an optional raw description into a domain value.
    ///
    /// Both `None` and an empty string represent an absent description (`None`),
    /// avoiding duplicate representations of emptiness.
    pub fn from_optional(raw: Option<String>) -> Result<Option<Self>, ValidationError> {
        match raw {
            None => Ok(None),
            Some(value) if value.is_empty() => Ok(None),
            Some(value) => Self::try_from(value).map(Some),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for Description {
    type Error = ValidationError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        if raw.is_empty() || raw.chars().count() > 1000 {
            return Err(ValidationError::Description);
        }
        Ok(Self(raw))
    }
}

impl AsRef<str> for Description {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_characters_not_utf8_bytes() {
        assert!(Title::try_from(String::new()).is_err());
        assert!(Title::try_from("é".repeat(100)).is_ok());
        assert!(Title::try_from("é".repeat(101)).is_err());
        assert!(Description::try_from(String::new()).is_err());
        assert!(Description::try_from("é".repeat(1000)).is_ok());
        assert!(Description::try_from("é".repeat(1001)).is_err());
    }

    #[test]
    fn optional_description_normalizes_empty_string_to_none() {
        assert_eq!(Description::from_optional(None).unwrap(), None);
        assert_eq!(
            Description::from_optional(Some(String::new())).unwrap(),
            None
        );
        let desc = Description::from_optional(Some("notes".to_owned()))
            .unwrap()
            .unwrap();
        assert_eq!(desc.as_str(), "notes");
    }
}
