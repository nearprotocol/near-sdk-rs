use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use serde::{de, Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;

use crate::env::is_valid_account_id;

/// Account identifier. Provides access to user's state.
#[derive(
    Debug,
    Clone,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Hash,
    BorshSchema,
)]
pub struct AccountId(String);

impl AccountId {
    /// Returns reference to the account ID bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
    /// Returns reference to the account ID string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    /// Consumes `self` to return inner [`String`] representation.
    pub fn into_string(self) -> String {
        self.0
    }
    // TODO this should probably be marked as unstable or crate scoped
    /// Constructs new AccountId from `String` without checking validity.
    pub fn new_unchecked(id: String) -> Self {
        Self(id)
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<AccountId> for String {
    fn from(id: AccountId) -> Self {
        id.0
    }
}

impl AsRef<str> for AccountId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl<'de> Deserialize<'de> for AccountId {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Self::try_from(s).map_err(de::Error::custom)
    }
}

impl TryFrom<&str> for AccountId {
    type Error = ParseAccountIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<&[u8]> for AccountId {
    type Error = ParseAccountIdError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        core::str::from_utf8(value)
            .map_err(|_| ParseAccountIdError { kind: ParseAccountIdErrorKind::InvalidUtf8 })?
            .parse()
    }
}

fn validate_account_id(id: &str) -> Result<(), ParseAccountIdError> {
    if is_valid_account_id(id.as_bytes()) {
        Ok(())
    } else {
        Err(ParseAccountIdError { kind: ParseAccountIdErrorKind::InvalidAccountId })
    }
}

impl TryFrom<Vec<u8>> for AccountId {
    type Error = ParseAccountIdError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(
            String::from_utf8(value)
                .map_err(|_| ParseAccountIdError { kind: ParseAccountIdErrorKind::InvalidUtf8 })?,
        )
    }
}

impl TryFrom<String> for AccountId {
    type Error = ParseAccountIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_account_id(value.as_str())?;
        Ok(Self(value))
    }
}

impl std::str::FromStr for AccountId {
    type Err = ParseAccountIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        validate_account_id(value)?;
        Ok(Self(value.to_string()))
    }
}

#[derive(Debug)]
pub struct ParseAccountIdError {
    kind: ParseAccountIdErrorKind,
}

#[derive(Debug)]
enum ParseAccountIdErrorKind {
    InvalidAccountId,
    InvalidUtf8,
}

impl std::fmt::Display for ParseAccountIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ParseAccountIdErrorKind::InvalidAccountId => write!(f, "the account ID is invalid"),
            ParseAccountIdErrorKind::InvalidUtf8 => write!(f, "bytes are not valid utf-8"),
        }
    }
}

impl std::error::Error for ParseAccountIdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deser() {
        let key: AccountId = serde_json::from_str("\"alice.near\"").unwrap();
        assert_eq!(key.0, "alice.near".to_string());

        let key: Result<AccountId, _> = serde_json::from_str("Alice.near");
        assert!(key.is_err());
    }

    #[test]
    fn test_ser() {
        let key: AccountId = "alice.near".parse().unwrap();
        let actual: String = serde_json::to_string(&key).unwrap();
        assert_eq!(actual, "\"alice.near\"");
    }

    #[test]
    fn test_from_str() {
        let key = AccountId::try_from("alice.near").unwrap();
        assert_eq!(key.as_ref(), &"alice.near".to_string());
    }
}