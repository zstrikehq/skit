use crate::crypto;
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum SkitError {
    Io(io::Error),
    Crypto(crypto::CryptoError),
    SerdeJson(serde_json::Error),
    KeyNotFound,
    SafeNotFound(String),
    InvalidPassword(String),
    EmptyCommand,
    ParseError(String),
    AwsError(String),
}

impl fmt::Display for SkitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SkitError::Io(e) => write!(f, "IO error: {}", e),
            SkitError::Crypto(e) => write!(f, "Crypto error: {}", e),
            SkitError::SerdeJson(e) => write!(f, "JSON serialization error: {}", e),
            SkitError::KeyNotFound => write!(f, "Key not found in safe"),
            SkitError::SafeNotFound(path) => write!(f, "Safe not found: {}", path),
            SkitError::InvalidPassword(msg) => write!(f, "{}", msg),
            SkitError::EmptyCommand => write!(f, "No command provided to execute"),
            SkitError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            SkitError::AwsError(msg) => write!(f, "AWS error: {}", msg),
        }
    }
}

impl Error for SkitError {}

impl From<io::Error> for SkitError {
    fn from(error: io::Error) -> Self {
        SkitError::Io(error)
    }
}

impl From<crypto::CryptoError> for SkitError {
    fn from(error: crypto::CryptoError) -> Self {
        SkitError::Crypto(error)
    }
}

impl From<serde_json::Error> for SkitError {
    fn from(error: serde_json::Error) -> Self {
        SkitError::SerdeJson(error)
    }
}
