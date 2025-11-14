use aes_gcm::aead::{Aead, OsRng, rand_core::RngCore};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use argon2::{Algorithm, Params, Version};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use base64::{Engine as _, engine::general_purpose};
use std::error::Error;
use std::fmt;
use zeroize::Zeroize;

#[derive(Debug)]
pub enum CryptoError {
    EncryptionFailed,
    DecryptionFailed,
    InvalidFormat,
    PasswordHashFailed,
    PasswordVerificationFailed,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CryptoError::EncryptionFailed => write!(f, "Encryption failed"),
            CryptoError::DecryptionFailed => write!(f, "Decryption failed"),
            CryptoError::InvalidFormat => write!(f, "Invalid encrypted format"),
            CryptoError::PasswordHashFailed => write!(f, "Password hashing failed"),
            CryptoError::PasswordVerificationFailed => write!(f, "Password verification failed"),
        }
    }
}

impl Error for CryptoError {}

pub struct EncryptBuilder<'a> {
    password: Option<&'a str>,
    plaintext: Option<&'a str>,
}

pub struct DecryptBuilder<'a> {
    password: Option<&'a str>,
    ciphertext: Option<&'a str>,
}

impl<'a> EncryptBuilder<'a> {
    pub fn new() -> Self {
        Self {
            password: None,
            plaintext: None,
        }
    }

    pub fn password(mut self, password: &'a str) -> Self {
        self.password = Some(password);
        self
    }

    pub fn plaintext(mut self, plaintext: &'a str) -> Self {
        self.plaintext = Some(plaintext);
        self
    }

    pub fn encrypt(self) -> Result<String, CryptoError> {
        let password = self.password.ok_or(CryptoError::EncryptionFailed)?;
        let plaintext = self.plaintext.ok_or(CryptoError::EncryptionFailed)?;

        encrypt_value_with_salt(password, plaintext)
    }
}

impl<'a> DecryptBuilder<'a> {
    pub fn new() -> Self {
        Self {
            password: None,
            ciphertext: None,
        }
    }

    pub fn password(mut self, password: &'a str) -> Self {
        self.password = Some(password);
        self
    }

    pub fn ciphertext(mut self, ciphertext: &'a str) -> Self {
        self.ciphertext = Some(ciphertext);
        self
    }

    pub fn decrypt(self) -> Result<String, CryptoError> {
        let password = self.password.ok_or(CryptoError::DecryptionFailed)?;
        let ciphertext = self.ciphertext.ok_or(CryptoError::DecryptionFailed)?;

        decrypt_value_with_salt(ciphertext, password)
    }
}

const ARGON2_MEMORY_KIB: u32 = 64 * 1024; // 64 MiB
const ARGON2_TIME_COST: u32 = 3;
const ARGON2_LANES: u32 = 1;

fn argon2id_derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError> {
    let params = Params::new(ARGON2_MEMORY_KIB, ARGON2_TIME_COST, ARGON2_LANES, None)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    let a2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; 32];
    a2.hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    Ok(key)
}

pub fn encrypt_value_with_salt(password: &str, plaintext: &str) -> Result<String, CryptoError> {
    // Random 16-byte salt and 12-byte nonce
    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);

    let mut key = argon2id_derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::EncryptionFailed)?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ct = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| CryptoError::EncryptionFailed)?;

    key.zeroize();

    // package: salt || nonce || ciphertext+tag
    let mut blob = Vec::with_capacity(16 + 12 + ct.len());
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ct);

    Ok(format!("ENC~v1~{}", general_purpose::STANDARD.encode(blob)))
}

pub fn decrypt_value_with_salt(enc: &str, password: &str) -> Result<String, CryptoError> {
    if !enc.starts_with("ENC~v1~") {
        return Err(CryptoError::InvalidFormat);
    }
    let b64 = &enc[7..];
    let data = general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| CryptoError::InvalidFormat)?;
    if data.len() < 16 + 12 + 16 {
        return Err(CryptoError::InvalidFormat);
    }

    let (salt, rest) = data.split_at(16);
    let (nonce_bytes, ciphertext) = rest.split_at(12);

    let mut key = argon2id_derive_key(password, salt).map_err(|_| CryptoError::DecryptionFailed)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::DecryptionFailed)?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let pt = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)?;

    key.zeroize();
    String::from_utf8(pt).map_err(|_| CryptoError::DecryptionFailed)
}

pub fn hash_password(password: &str) -> Result<String, CryptoError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| CryptoError::PasswordHashFailed)
}

pub fn verify_password(password: &str, hash: &str) -> Result<(), CryptoError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| CryptoError::InvalidFormat)?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| CryptoError::PasswordVerificationFailed)
}
