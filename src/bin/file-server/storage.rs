use std::collections::HashMap;
use std::error::Error;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Mutex, RwLock};

#[trait_variant::make(Send)]
#[dynosaur::dynosaur(DynStorageBackend = dyn(box) StorageBackend, bridge(dyn))]
pub trait StorageBackend: Send + Sync {
    async fn store(&self, key: String, value: Vec<u8>) -> Result<(), Box<dyn Error + '_>>;
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn Error + '_>>;
}

pub struct MemoryBackend {
    data: RwLock<HashMap<String, Vec<u8>>>
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            data: Default::default() 
        }
    }
}

impl StorageBackend for MemoryBackend {
    async fn store(&self, key:String, value:Vec<u8>) -> Result<(), Box<dyn Error + '_>>  {
        let mut data = self.data.write()?;
        data.insert(key, value);
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn Error + '_>>  {
        let data = self.data.read()?;
        Ok(data.get(key).cloned())
    }
}

pub struct MemoryBackendInitializer {}

impl MemoryBackendInitializer {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct FileStore {
    backend: Box<DynStorageBackend<'static>>,
    hasher: fn(&[u8]) -> String
}

#[dynosaur::dynosaur(DynFileStoreInitializer = dyn(box) FileStoreInitializer)]
pub trait FileStoreInitializer: Send + Sync {
    fn create(&self) -> impl Future<Output = FileStore> + Send;
}

impl FileStoreInitializer for MemoryBackendInitializer {
    async fn create(&self) -> FileStore {
        FileStore {
            backend: DynStorageBackend::new_box(MemoryBackend::new()),
            hasher: hash_using_std
        }
    }

}

impl FileStore {
    pub fn new<S: StorageBackend + 'static>(backend: S, hasher: fn(&[u8]) -> String) -> Self {
        Self {
            backend: DynStorageBackend::new_box(backend),
            hasher
        }
    }
    pub async fn store(&self, data: Vec<u8>) -> Result<String, Box<dyn Error + '_>> {
        let hash = (self.hasher)(&data);
        self.backend.store(hash.clone(), data).await?;
        Ok(hash)
    }
    pub async fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn Error + '_>> {
        self.backend.get(key).await
    }
}

fn hash_using_std(data: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish().to_string()
}

#[cfg(feature = "aws_s3")]
mod s3_storage_impl {
    use std::error::Error;

    use aws_config::BehaviorVersion;
    use aws_sdk_s3::{
        Client, operation::get_object::GetObjectError, primitives::ByteStream, types::error::NoSuchKey
    };

    use crate::storage::{FileStore, FileStoreInitializer, StorageBackend, hash_using_std};

    pub struct S3StorageInitializer {
        bucket_name: String
    }

    impl S3StorageInitializer {
        pub fn from_bucket_name(bucket_name: String) -> Self {
            Self { bucket_name }
        }
    }

    impl FileStoreInitializer for S3StorageInitializer {
        async fn create(&self) -> FileStore {
            let s3_store = S3Storage::new(self.bucket_name.clone()).await;
            FileStore::new(s3_store, hash_using_std)
        }
    }

    pub struct S3Storage {
        bucket_name: String,
        s3_client: Client,
    }

    impl S3Storage {
        pub async fn new(bucket_name: String) -> Self {
            let behavior_version = BehaviorVersion::v2026_01_12();
            let config = aws_config::defaults(behavior_version).load().await;

            let s3_client = Client::new(&config);
            Self {
                bucket_name,
                s3_client,
            }
        }
    }

    impl StorageBackend for S3Storage {
        async fn store(&self, key: String, value: Vec<u8>) -> Result<(), Box<dyn Error + '_>> {
            let bytes = ByteStream::from(value);
            self.s3_client
                .put_object()
                .bucket(&self.bucket_name)
                .key(&key)
                .body(bytes)
                .send()
                .await
                .map_err(Box::new)?;
            Ok(())
        }
        async fn get(&self, id: &str) -> Result<Option<Vec<u8>>, Box<dyn Error + '_>> {
            let response = self
                .s3_client
                .get_object()
                .bucket(&self.bucket_name)
                .key(id)
                .send()
                .await;

            let response_bytes = match response {
                Err(sdk_error) => {
                    if let Some(GetObjectError::NoSuchKey(_)) = sdk_error.as_service_error() {
                        return Ok(None);
                    } else {
                        return Err(Box::new(sdk_error));
                    }
                }
                Ok(value) => {
                    value.body.collect().await?
                }
            };

            Ok(Some(response_bytes.to_vec()))
        }
    }
}

#[cfg(not(feature = "aws_s3"))]
mod s3_storage_impl {
    use std::{error::{self, Error}, fmt::Display};

    use crate::storage::{FileStore, FileStoreInitializer, StorageBackend};

    #[derive(Debug)]
    struct UnsupportedError {}

    impl Error for UnsupportedError {}

    impl Display for UnsupportedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Apollo was not compiled with S3 support")
        }
    }

    pub struct S3StorageInitializer {}

    impl S3StorageInitializer {
        pub fn from_bucket_name(bucket_name: String) -> Self {
            Self {}
        }
    }

    impl FileStoreInitializer for S3StorageInitializer {
        async fn create(&self) -> FileStore {
            panic!("Apollo was not compiled with S3 support")
        }
    }

    struct S3Storage {}

    impl StorageBackend for S3Storage {
        async fn store(&self, key: String, value: Vec<u8>) -> Result<(), Box<dyn Error + '_>> {
            Err(Box::new(UnsupportedError {}))
        }
        async fn get(&self, id: &str) -> Result<Option<Vec<u8>>, Box<dyn Error + '_>> {
            Err(Box::new(UnsupportedError {}))
        }
    }
}


use rocket::{Build, Rocket, State, fairing};
use rocket::fairing::{Fairing, Info};
pub use s3_storage_impl::*;

pub struct FileStoreFairing {
    init: Box<DynFileStoreInitializer<'static>> 
}

impl FileStoreFairing {
    pub fn new<I: FileStoreInitializer + 'static>(init: I) -> Self {
       Self {
            init: DynFileStoreInitializer::new_box(init)
       }
    }
}

pub type FileStoreHandle = State<FileStore>;

#[rocket::async_trait]
impl Fairing for FileStoreFairing {
    fn info(&self) -> Info {
        Info { name: "Initializing file store", kind: fairing::Kind::Ignite }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let file_store = (self.init).create().await;
        let rocket = rocket.manage(file_store);
        Ok(rocket)
    }
}
