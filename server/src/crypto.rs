use aes_gcm::{
    Aes256Gcm, Error, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use sha3::Digest;
use std::sync::Arc;

#[derive(Clone)]
pub struct AES {
    cipher: Arc<Aes256Gcm>,
}

impl AES {
    pub fn new(key: &[u8; 32]) -> Self {
        Self {
            cipher: Arc::new(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key))),
        }
    }

    pub fn encrypt(&self, plaintext: &[u8], nonce: Option<&[u8]>) -> Result<Vec<u8>, Error> {
        let nonce = match nonce {
            Some(n) if n.len() == 12 => Nonce::from_slice(n).to_owned(),
            Some(_) => return Err(Error),
            None => Aes256Gcm::generate_nonce(&mut OsRng),
        };

        let ciphertext = self.cipher.encrypt(&nonce, plaintext)?;
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, Error> {
        if ciphertext.len() < 12 {
            return Err(Error);
        }

        let (nonce, encrypted_data) = ciphertext.split_at(12);
        self.cipher
            .decrypt(Nonce::from_slice(nonce), encrypted_data)
            .map_err(|_| Error)
    }
}

pub fn hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = sha3::Sha3_256::default();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}
