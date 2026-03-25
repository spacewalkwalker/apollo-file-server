use rocket::{uri, Responder, http::{Header, hyper::header::LOCATION}, post, serde::{Deserialize, json::Json}};
use rocket_db_pools::{Connection, sqlx::{self, Row}};

use crate::guards::{DBPool, UploadAuth};

use crate::routes::get_chart_set::rocket_uri_macro_get_chart_set;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct CreateChartSetReq<'a> {
    title: &'a str,
    artist: &'a str,
}

#[derive(Responder)]
enum CreateChartSetResponse {
    #[response(status = 500)]
    InternalError(()),
    #[response(status = 201)]
    Created((), Header<'static>),
}

#[post("/chart-set", data = "<set_data>")]
pub async fn create_chart_set(
    mut db: Connection<DBPool>,
    set_data: Json<CreateChartSetReq<'_>>,
    _auth: UploadAuth,
) -> CreateChartSetResponse {
    let result = sqlx::query(
        "INSERT INTO chart_sets (title, artist) VALUES ($1, $2) RETURNING chart_set_id",
    )
    .bind(set_data.title)
    .bind(set_data.artist)
    .fetch_one(&mut **db)
    .await;
    match result {
        Ok(inserted_row) => {
            let created_id: i32 = inserted_row.get("chart_set_id");
            let chart_set_uri = uri!(get_chart_set(set_id = created_id));
            let location_header = Header::new(LOCATION.as_str(), chart_set_uri.to_string());
            CreateChartSetResponse::Created((), location_header)
        }
        Err(error) => {
            // TODO a proper logging solution
            eprintln!("/chart-set/ error: {:?}", error);
            CreateChartSetResponse::InternalError(())
        }
    }
}
