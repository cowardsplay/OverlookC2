use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use sha2::{Digest, Sha256};
use hmac::{Hmac, Mac as HmacMac};
use anyhow::{Result, anyhow};

/// Encryption key wrapper
#[derive(Clone)]
pub struct EncryptionKey {
    key: Key<Aes256Gcm>,
    hmac_key: Vec<u8>,
}

impl EncryptionKey {
    /// Create a new encryption key from a string
    pub fn new(key_str: &str) -> Result<Self> {
        let mut hasher = Sha256::new();
        hasher.update(key_str.as_bytes());
        let key_bytes = hasher.finalize();
        
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let hmac_key = key_bytes.to_vec();
        
        Ok(Self {
            key: *key,
            hmac_key,
        })
    }
    
    /// Generate a random encryption key
    pub fn random() -> Result<Self> {
        let key_bytes: [u8; 32] = OsRng.gen();
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let hmac_key = key_bytes.to_vec();
        
        Ok(Self {
            key: *key,
            hmac_key,
        })
    }
    
    /// Encrypt data with authentication
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce_bytes: [u8; 12] = OsRng.gen();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        // Combine nonce and ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        // Add HMAC for integrity
        let mut mac = <Hmac<Sha256> as HmacMac>::new_from_slice(&self.hmac_key)
            .map_err(|e| anyhow!("HMAC creation failed: {}", e))?;
        mac.update(&result);
        let hmac = mac.finalize();
        result.extend_from_slice(hmac.into_bytes().as_slice());
        
        Ok(result)
    }
    
    /// Decrypt data with authentication
    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        if encrypted_data.len() < 44 { // 12 (nonce) + 16 (min ciphertext) + 32 (hmac)
            return Err(anyhow!("Encrypted data too short"));
        }
        
        // Extract HMAC
        let hmac_start = encrypted_data.len() - 32;
        let data_without_hmac = &encrypted_data[..hmac_start];
        let expected_hmac = &encrypted_data[hmac_start..];
        
        // Verify HMAC
        let mut mac = <Hmac<Sha256> as HmacMac>::new_from_slice(&self.hmac_key)
            .map_err(|e| anyhow!("HMAC creation failed: {}", e))?;
        mac.update(data_without_hmac);
        let computed_hmac = mac.finalize();
        
        // Simple byte comparison for HMAC verification
        if computed_hmac.into_bytes().as_slice() != expected_hmac {
            return Err(anyhow!("HMAC verification failed"));
        }
        
        // Extract nonce and ciphertext
        let nonce_bytes = &data_without_hmac[..12];
        let ciphertext = &data_without_hmac[12..];
        
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        
        Ok(plaintext)
    }
    
    /// Encrypt and base64 encode data
    pub fn encrypt_b64(&self, data: &[u8]) -> Result<String> {
        let encrypted = self.encrypt(data)?;
        Ok(general_purpose::STANDARD.encode(encrypted))
    }
    
    /// Decrypt base64 encoded data
    pub fn decrypt_b64(&self, b64_data: &str) -> Result<Vec<u8>> {
        let encrypted = general_purpose::STANDARD.decode(b64_data)
            .map_err(|e| anyhow!("Base64 decode failed: {}", e))?;
        self.decrypt(&encrypted)
    }
}

/// Generate a random string for use as encryption key
pub fn generate_random_key() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    general_purpose::STANDARD.encode(bytes)
}

/// Hash a password with salt
pub fn hash_password(password: &str, salt: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(password.as_bytes());
    hasher.finalize().to_vec()
}

/// Verify a password against a hash
pub fn verify_password(password: &str, salt: &[u8], hash: &[u8]) -> bool {
    let computed_hash = hash_password(password, salt);
    computed_hash == hash
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encryption_decryption() {
        let key = EncryptionKey::new("test-key").unwrap();
        let data = b"Hello, World!";
        
        let encrypted = key.encrypt(data).unwrap();
        let decrypted = key.decrypt(&encrypted).unwrap();
        
        assert_eq!(data, decrypted.as_slice());
    }
    
    #[test]
    fn test_b64_encryption_decryption() {
        let key = EncryptionKey::new("test-key").unwrap();
        let data = b"Hello, World!";
        
        let encrypted_b64 = key.encrypt_b64(data).unwrap();
        let decrypted = key.decrypt_b64(&encrypted_b64).unwrap();
        
        assert_eq!(data, decrypted.as_slice());
    }
    
    #[test]
    fn test_tamper_detection() {
        let key = EncryptionKey::new("test-key").unwrap();
        let data = b"Hello, World!";
        
        let mut encrypted = key.encrypt(data).unwrap();
        encrypted[0] ^= 1; // Tamper with the data
        
        let result = key.decrypt(&encrypted);
        assert!(result.is_err());
    }
} 