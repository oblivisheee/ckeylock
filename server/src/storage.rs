use crate::crypto::{AES, hash};

use bincode::{de, error};
use dashmap::DashMap;
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Read, Seek as _, Write},
    path::Path,
};
use thiserror::Error;
use tracing::{debug, error, info, trace, warn};

pub struct Storage {
    data: DashMap<Vec<u8>, Vec<u8>>,
    file: File,
    aes: AES,
    checksum: Vec<u8>,
}

impl Storage {
    pub fn new(path: impl AsRef<Path>, aes: AES) -> Result<Self, StorageError> {
        info!("Initializing storage from path: {:?}", path.as_ref());
        if path.as_ref().exists() {
            Self::from_file(path, aes)
        } else {
            Self::new_empty(path, aes)
        }
    }

    pub fn new_empty(path: impl AsRef<Path>, aes: AES) -> Result<Self, StorageError> {
        info!("Creating new empty storage at path: {:?}", path.as_ref());
        let path = path.as_ref();
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(path)?;
        let dashmap: DashMap<Vec<u8>, Vec<u8>> = DashMap::new();
        let content = bincode::serde::encode_to_vec(&dashmap, bincode::config::standard())?;
        let checksum = hash(&content);
        let encrypted_content = aes
            .encrypt(&content, None)
            .map_err(|e| StorageError::Aes(e))?;
        file.write_all(&encrypted_content)?;
        info!("Empty storage created successfully.");
        Ok(Self {
            data: dashmap,
            file,
            aes,
            checksum: checksum.to_vec(),
        })
    }

    pub fn from_file(path: impl AsRef<Path>, aes: AES) -> Result<Self, StorageError> {
        info!("Loading storage from file at path: {:?}", path.as_ref());
        let path = path.as_ref();
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let mut reader = BufReader::new(&file);
        let mut content = Vec::new();
        reader.read_to_end(&mut content)?;
        let checksum = hash(&content);
        let decrypted_content = aes.decrypt(&content).map_err(|e| StorageError::Aes(e))?;
        let (decoded_data, _) =
            bincode::serde::decode_from_slice(&decrypted_content, bincode::config::standard())?;
        info!("Storage loaded successfully from file.");
        Ok(Self {
            data: decoded_data,
            file,
            aes,
            checksum: checksum.to_vec(),
        })
    }

    pub fn sync(&mut self) -> Result<(), StorageError> {
        debug!("Syncing storage to file.");
        let content = bincode::serde::encode_to_vec(&self.data, bincode::config::standard())?;
        let checksum = hash(&content).to_vec();
        if checksum != self.checksum {
            let encrypted_content = self
                .aes
                .encrypt(&content, None)
                .map_err(StorageError::Aes)?;
            self.file.set_len(0)?;
            self.file.seek(std::io::SeekFrom::Start(0))?;
            self.file.write_all(&encrypted_content)?;
            self.file.sync_all()?;
            self.checksum = checksum;
            info!("Storage synced successfully.");
        } else {
            debug!("No changes detected, skipping sync.");
        }
        Ok(())
    }

    pub fn set(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, StorageError> {
        debug!(
            "Setting key: {:?} with value of length: {}",
            key,
            value.len()
        );
        self.data.insert(key.clone(), value);
        self.sync()?;
        info!("Key {:?} set successfully.", key);
        Ok(key)
    }

    pub fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StorageError> {
        debug!("Getting value for key: {:?}", key);
        let value = self.data.get(&key).map(|v| v.clone());
        if value.is_some() {
            info!("Key {:?} found.", key);
        } else {
            warn!("Key {:?} not found.", key);
        }
        Ok(value)
    }

    pub fn delete(&mut self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StorageError> {
        debug!("Deleting key: {:?}", key);
        let value = self.data.remove(&key).map(|v| v.clone()).map(|(k, v)| k);
        self.sync()?;
        if value.is_some() {
            info!("Key {:?} deleted successfully.", key);
        } else {
            warn!("Key {:?} not found for deletion.", key);
        }
        Ok(value)
    }

    pub fn list(&self) -> Result<Vec<Vec<u8>>, StorageError> {
        debug!("Listing all keys in storage.");
        let keys: Vec<Vec<u8>> = self.data.iter().map(|v| v.key().clone()).collect();
        info!("Listed {} keys.", keys.len());
        Ok(keys)
    }

    pub fn exists(&self, key: Vec<u8>) -> Result<bool, StorageError> {
        debug!("Checking existence of key: {:?}", key);
        let exists = self.data.contains_key(&key);
        if exists {
            info!("Key {:?} exists.", key);
        } else {
            warn!("Key {:?} does not exist.", key);
        }
        Ok(exists)
    }

    pub fn count(&self) -> Result<usize, StorageError> {
        debug!("Counting keys in storage.");
        let count = self.data.len();
        info!("Storage contains {} keys.", count);
        Ok(count)
    }

    pub fn clear(&mut self) -> Result<(), StorageError> {
        debug!("Clearing all keys in storage.");
        self.data.clear();
        self.sync()?;
        info!("Storage cleared successfully.");
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Encode bincode error: {0}")]
    EncodeBincode(#[from] bincode::error::EncodeError),
    #[error("Decode bincode error: {0}")]
    DecodeBincode(#[from] bincode::error::DecodeError),
    #[error("AES error: {0}")]
    Aes(aes_gcm::Error),
}
