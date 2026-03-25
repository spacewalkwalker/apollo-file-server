use rocket::{Request, http::Status, request::{FromRequest, Outcome}};
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
        let Outcome::Success(mut db) = Connection::<DBPool>::from_request(req).await else {
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
            None => Outcome::Error((Status::Forbidden, ApiKeyError::Invalid))
        }
    }
}

