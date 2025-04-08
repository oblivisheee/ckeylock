use crate::crypto::{AES, hash};
use dashmap::DashMap;
use lru::LruCache;
use std::sync::PoisonError;
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek as _, SeekFrom, Write},
    path::Path,
};
use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard};
use tracing::{debug, error, info, warn};

const LRU_CACHE_SIZE: usize = 100;
pub struct Storage {
    data: Box<DashMap<Vec<u8>, Vec<u8>>>,
    file: File,
    aes: AES,
    checksum: Vec<u8>,
    cache: Mutex<LruCache<Vec<u8>, Vec<u8>>>,
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
            data: Box::new(dashmap),
            file,
            aes,
            checksum: checksum.to_vec(),
            cache: Mutex::new(LruCache::new(
                std::num::NonZero::new(LRU_CACHE_SIZE).unwrap(),
            )), // Initialize LRU cache with a capacity of 100
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
            cache: Mutex::new(LruCache::new(
                std::num::NonZero::new(LRU_CACHE_SIZE).unwrap(),
            )),
        })
    }

    pub fn sync(&mut self) -> Result<(), StorageError> {
        debug!("Syncing storage to file.");
        let content = bincode::serde::encode_to_vec(&self.data, bincode::config::standard())?;
        let new_checksum = hash(&content).to_vec();

        if new_checksum != self.checksum {
            let encrypted_content = self
                .aes
                .encrypt(&content, None)
                .map_err(StorageError::Aes)?;

            let file = &mut self.file;
            file.set_len(0)?;
            file.seek(SeekFrom::Start(0))?;
            let mut writer = BufWriter::new(file);
            writer.write_all(&encrypted_content)?;
            writer.flush()?;
            drop(writer);
            self.file.sync_all()?;

            self.checksum = new_checksum;
            info!("Storage synced successfully.");
        } else {
            debug!("No changes detected, skipping sync.");
        }
        Ok(())
    }

    pub async fn set(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, StorageError> {
        debug!(
            "Setting key: {:?} with value of length: {}",
            hex::encode(&key),
            value.len()
        );
        self.data.insert(key.clone(), value.clone());
        self.cache.lock().await.put(key.clone(), value.clone());
        info!("Key {:?} set successfully.", hex::encode(&key));
        Ok(key)
    }

    pub async fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StorageError> {
        debug!("Getting value for key: {:?}", hex::encode(&key));
        if let Some(value) = self.cache.lock().await.get(&key) {
            info!("Cache hit for key: {:?}", hex::encode(&key));
            return Ok(Some(value.clone()));
        }
        // Fallback to DashMap
        let value = self.data.get(&key).map(|v| v.clone());
        if let Some(ref v) = value {
            self.cache.lock().await.put(key.clone(), v.clone()); // Update cache
            info!("Key {:?} found.", hex::encode(&key));
        } else {
            warn!("Key {:?} not found.", hex::encode(&key));
        }
        Ok(value)
    }

    pub async fn delete(&mut self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StorageError> {
        debug!("Deleting key: {:?}", hex::encode(&key));
        self.cache.lock().await.pop(&key);
        let value = self.data.remove(&key).map(|v| v.clone()).map(|(k, _)| k);
        self.sync()?;
        if value.is_some() {
            info!("Key {:?} deleted successfully.", hex::encode(&key));
        } else {
            warn!("Key {:?} not found for deletion.", hex::encode(&key));
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
        debug!("Checking existence of key: {:?}", hex::encode(&key));
        let exists = self.data.contains_key(&key);
        if exists {
            info!("Key {:?} exists.", hex::encode(&key));
        } else {
            warn!("Key {:?} does not exist.", hex::encode(&key));
        }
        Ok(exists)
    }

    pub fn count(&self) -> Result<usize, StorageError> {
        debug!("Counting keys in storage.");
        let count = self.data.len();
        info!("Storage contains {} keys.", count);
        Ok(count)
    }

    pub async fn clear(&mut self) -> Result<(), StorageError> {
        debug!("Clearing all keys in storage.");
        self.data.clear();
        self.cache.lock().await.clear();
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
