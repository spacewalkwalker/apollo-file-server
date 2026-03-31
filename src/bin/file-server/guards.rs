use std::{fmt::Display, sync::atomic::{AtomicU64, Ordering}};

use rocket::{
    Build, Request, Rocket,
    fairing::{self, Fairing, Info},
    http::Status,
    request::{FromRequest, Outcome},
};
use rocket_db_pools::{Connection, Database, sqlx};

#[derive(Database)]
#[database("apollo")]
pub struct DBPool(sqlx::PgPool);

pub struct UploadAuth {}

#[derive(Debug)]
pub enum ApiKeyError {
    Missing,
    Invalid,
    Other,
}

#[rocket::async_trait]
impl<'a> FromRequest<'a> for UploadAuth {
    type Error = ApiKeyError;

    async fn from_request(req: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        let Outcome::Success(mut db) = req.guard::<Connection<DBPool>>().await else {
            return Outcome::Error((Status::InternalServerError, ApiKeyError::Other));
        };

        let Some(api_key) = req.headers().get_one("X-Apollo-Upload-Key") else {
            return Outcome::Error((Status::Forbidden, ApiKeyError::Missing));
        };

        let Ok(query_results) = sqlx::query("SELECT api_key FROM api_keys WHERE api_key = $1")
            .bind(api_key)
            .fetch_optional(&mut **db)
            .await
        else {
            return Outcome::Error((Status::InternalServerError, ApiKeyError::Other));
        };

        match query_results {
            Some(_) => Outcome::Success(UploadAuth {}),
            None => Outcome::Error((Status::Forbidden, ApiKeyError::Invalid)),
        }
    }
}

struct RequestIDCounter {
    counter: AtomicU64,
}


pub struct RequestIDFairing {
    start: u64,
}

impl RequestIDFairing {
    pub fn new() -> Self {
        Self { start: 0 }
    }
}

#[rocket::async_trait]
impl Fairing for RequestIDFairing {
    fn info(&self) -> Info {
        Info {
            name: "Initializing file store",
            kind: fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let rocket = rocket.manage(RequestIDCounter {
            counter: self.start.into(),
        });
        Ok(rocket)
    }
}

pub struct RequestID {
    pub id: u64,
}

impl Display for RequestID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[rocket::async_trait]
impl<'a> FromRequest<'a> for RequestID {
    type Error = ();

    async fn from_request(req: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        match req.rocket().state::<RequestIDCounter>() {
            Some(counter) => {
                let id = counter.counter.fetch_add(1, Ordering::SeqCst);
                Outcome::Success(RequestID { id })
            }
            None => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}
