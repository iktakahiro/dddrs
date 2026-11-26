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

#[derive(Debug, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("Title must be between 1 and 100 characters")]
    Title,
    #[error("Description must be 1000 characters or less")]
    Description,
}

#[derive(Debug, PartialEq, Eq)]
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
}

#[derive(Debug, PartialEq, Eq)]
pub struct Description(String);

impl TryFrom<String> for Description {
    type Error = ValidationError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        if raw.chars().count() > 1000 {
            return Err(ValidationError::Description);
        }
        Ok(Self(raw))
    }
}

impl Description {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_characters_not_utf8_bytes() {
        assert!(Title::try_from(String::new()).is_err());
        assert!(Title::try_from("猫".repeat(100)).is_ok());
        assert!(Title::try_from("猫".repeat(101)).is_err());
        assert!(Description::try_from("猫".repeat(1000)).is_ok());
        assert!(Description::try_from("猫".repeat(1001)).is_err());
    }
}
