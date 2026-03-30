
use std::env;

use figment::{Figment};
use rocket::{
    routes,
    get,
    launch,
    serde::{Deserialize, Serialize},
};
use rocket_db_pools::{
    Database,
};

use crate::{
    guards::DBPool,
    routes::{
        create_chart::create_chart, create_chart_set::create_chart_set, create_chart_set_aux_file::create_chart_set_aux_file, get_chart::get_chart, get_chart_file::get_chart_file, get_chart_set::get_chart_set, get_chart_set_aux_file::get_chart_set_aux_file, search_charts::search_charts
    },
    storage::{FileStoreFairing, MemoryBackendInitializer, S3StorageInitializer},
};

mod storage;

mod metadata;

mod guards;
mod routes;
// this is like the blender default cube. you do not remove the
// blender default cube
#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Deserialize, Serialize, Default)]
#[serde(crate = "rocket::serde")]
enum AvailableStorageBackend {
    #[default]
    #[serde(rename = "local")]
    LocalStorage,
    #[serde(rename = "s3")]
    S3Storage,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(crate = "rocket::serde")]
struct ServerConfig {
    bucket_name: Option<String>,
    storage_backend: AvailableStorageBackend,
}

// TODO
// - make (chart_set_id, designator) unique
// - add PATCH, DELETE endpoints
// - a proper logging system

#[launch]
fn rocket() -> _ {
    let config: ServerConfig = Figment::new()
        .merge(figment::providers::Env::prefixed("APOLLO_"))
        .extract()
        .expect("Parsing server config failed");

    let storage_fairing = match config.storage_backend {
        AvailableStorageBackend::LocalStorage => {
            FileStoreFairing::new(MemoryBackendInitializer::new())
        }
        AvailableStorageBackend::S3Storage => {
            FileStoreFairing::new(S3StorageInitializer::from_bucket_name(
                config
                    .bucket_name
                    .expect("S3 storage enabled, but no bucket name provided"),
            ))
        }
    };

    let mut rocket_config = rocket::Config::figment();

    match env::var("PORT") {
        Ok(port) => {
            if let Ok(port) = port.parse::<u16>() {
                rocket_config = rocket_config.merge(("port", port));
            }
        }
        _ => {}
    }

    rocket::build()
        .mount(
            "/",
            routes![
                get_chart_set,
                get_chart,
                create_chart_set,
                create_chart,
                search_charts,
                get_chart_file,
                create_chart_set_aux_file,
                get_chart_set_aux_file
            ],
        )
        .configure(rocket_config)
        .attach(DBPool::init())
        .attach(storage_fairing)
}
