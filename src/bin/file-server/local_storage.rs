use std::{fs, hash::{DefaultHasher, Hash, Hasher}, io, num::ParseIntError, path::PathBuf, str::FromStr, sync::atomic::{AtomicU64, Ordering}};


// Identifies a single file in the local storage pool.
pub struct LocalStorageId {
    file_id: String
}

impl LocalStorageId {
    pub fn get_file_uri(&self) -> String {
        format!("file.store/{}", self.file_id)
    }
}

impl ToString for LocalStorageId {
    fn to_string(&self) -> String {
        self.file_id.to_string()
    }
}

impl From<String> for LocalStorageId {
    fn from(file_id: String) -> Self {
        Self {
            file_id 
        }
    }
}

pub struct LocalStorage {
    storage_location: PathBuf,
}

fn hash_data(data: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish().to_string()
}

impl LocalStorage {

    // in a production api these should be read/write handles,
    // but also a production api should probably use an actual file storage api
    // so that is moot anyways
    
    // also this is blocking so maybe the earth explodes if you use this
    // in an async fn idk

    pub fn store(&self, binary_data: &[u8]) -> io::Result<LocalStorageId> {
        let file_id = hash_data(binary_data);
        let file_path = self.storage_location.join(&file_id);
        fs::write(file_path, binary_data)?;
        let handle_to_return = LocalStorageId {
            file_id: file_id.to_string()
        };
        Ok(handle_to_return)
    }

    pub fn retrieve(&self, id: LocalStorageId) -> io::Result<Vec<u8>> {
        let file_path = self.storage_location.join(id.file_id.to_string());
        fs::read(file_path)
    }

    pub fn new(storage_location: PathBuf) -> Self {
        Self {
            storage_location,
        }
    }
}
