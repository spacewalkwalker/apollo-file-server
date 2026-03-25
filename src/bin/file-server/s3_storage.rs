use std::{
    fmt::{Debug, Display},
    hash::{DefaultHasher, Hash, Hasher}, pin::Pin,
};

use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    Client,
    primitives::ByteStream,
};
use rocket::{Build, Rocket, State, fairing::{self, Fairing, Info}};

pub struct S3StorageId {
    file_id: String,
}

impl S3StorageId {
    pub fn get_file_id(&self) -> String {
        self.file_id.clone()
    }
}

pub struct S3Storage {
    bucket_name: String,
    s3_client: Client,
}

fn hash_data(data: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish().to_string()
}

#[derive(Debug)]
pub enum S3Error {
    S3Error(String),
}

impl Display for S3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            S3Error::S3Error(msg) => write!(f, "Error from S3: {msg}")
        }
    }
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
    pub async fn store(&self, binary_data: Vec<u8>) -> Result<S3StorageId, S3Error> {
        let data_hash = hash_data(&binary_data);
        let bytes = ByteStream::from(binary_data);
        self.s3_client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&data_hash)
            .body(bytes)
            .send()
            .await
            .map_err(|sdk_error| S3Error::S3Error(format!("{:?}", sdk_error)))?;
        Ok(S3StorageId { file_id: data_hash })
    }
    pub async fn retrieve(&self, id: S3StorageId) -> Result<Vec<u8>, S3Error> {
        let response = self
            .s3_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(&id.file_id)
            .send()
            .await
            .map_err(|sdk_error| S3Error::S3Error(format!("{:?}", sdk_error)))?;

        let response_bytes = response.body.collect().await
            .map_err(|byte_error| S3Error::S3Error(format!("{:?}", byte_error)))?;

        Ok(response_bytes.to_vec())
    }
}

struct S3StorageInitFairing {
    bucket_name: String
}

pub type S3StorageHandle = State<S3Storage>;

#[rocket::async_trait]
impl Fairing for S3StorageInitFairing {
    fn info(&self) -> Info {
        Info { name: "Initialize S3 client", kind: fairing::Kind::Ignite }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result { 
        let storage = S3Storage::new(self.bucket_name.clone()).await;
        let rocket = rocket.manage(storage);
        Ok(rocket)
    }
}

pub fn bucket_fairing(bucket_name: String) -> impl Fairing {
    S3StorageInitFairing {
        bucket_name
    }
}
