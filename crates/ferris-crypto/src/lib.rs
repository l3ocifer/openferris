use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

const HKDF_INFO: &[u8] = b"openferris-data-encryption-v1";
const NONCE_LEN: usize = 12;

/// AES-256-GCM cipher for encrypting data at rest.
///
/// Key is derived from an Ed25519 secret key via HKDF-SHA256,
/// so the same identity always produces the same encryption key.
#[derive(Clone)]
pub struct Cipher {
    cipher: Aes256Gcm,
}

impl Cipher {
    /// Derive an AES-256 key from raw Ed25519 secret key bytes (32 bytes)
    /// using HKDF-SHA256.
    pub fn from_secret_key_bytes(secret_key: &[u8; 32]) -> Self {
        let hk = Hkdf::<Sha256>::new(None, secret_key);
        let mut aes_key = [0u8; 32];
        hk.expand(HKDF_INFO, &mut aes_key)
            .expect("32 bytes is a valid HKDF-SHA256 output length");

        let key = Key::<Aes256Gcm>::from_slice(&aes_key);
        Self {
            cipher: Aes256Gcm::new(key),
        }
    }

    /// Encrypt plaintext. Returns `nonce || ciphertext || tag`.
    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .expect("AES-256-GCM encryption should not fail");

        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        out
    }

    /// Decrypt data produced by `encrypt()`. Returns plaintext or an error.
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if data.len() < NONCE_LEN + 16 {
            return Err(CryptoError::TooShort);
        }

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

#[derive(Debug)]
pub enum CryptoError {
    TooShort,
    DecryptionFailed,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "ciphertext too short"),
            Self::DecryptionFailed => write!(f, "decryption failed (wrong key or corrupted data)"),
        }
    }
}

impl std::error::Error for CryptoError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cipher() -> Cipher {
        let key = ed25519_dalek::SigningKey::generate(&mut OsRng);
        Cipher::from_secret_key_bytes(&key.to_bytes())
    }

    #[test]
    fn round_trip() {
        let cipher = test_cipher();
        let plaintext = b"hello, ferris!";

        let encrypted = cipher.encrypt(plaintext);
        assert_ne!(&encrypted, plaintext);
        assert!(encrypted.len() > plaintext.len());

        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn different_nonce_each_time() {
        let cipher = test_cipher();
        let plaintext = b"same data";

        let enc1 = cipher.encrypt(plaintext);
        let enc2 = cipher.encrypt(plaintext);
        assert_ne!(enc1, enc2, "each encrypt should use a fresh nonce");

        assert_eq!(cipher.decrypt(&enc1).unwrap(), plaintext);
        assert_eq!(cipher.decrypt(&enc2).unwrap(), plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let cipher1 = test_cipher();
        let cipher2 = test_cipher();

        let encrypted = cipher1.encrypt(b"secret");
        let result = cipher2.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn too_short_data_fails() {
        let cipher = test_cipher();
        let result = cipher.decrypt(&[0u8; 10]);
        assert!(matches!(result, Err(CryptoError::TooShort)));
    }

    #[test]
    fn same_key_same_derivation() {
        let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let cipher1 = Cipher::from_secret_key_bytes(&signing_key.to_bytes());
        let cipher2 = Cipher::from_secret_key_bytes(&signing_key.to_bytes());

        let encrypted = cipher1.encrypt(b"consistency");
        let decrypted = cipher2.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, b"consistency");
    }

    #[test]
    fn empty_plaintext() {
        let cipher = test_cipher();
        let encrypted = cipher.encrypt(b"");
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert!(decrypted.is_empty());
    }
}
