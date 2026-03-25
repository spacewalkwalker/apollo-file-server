use rocket::{get, serde::{Serialize, json::Json}};
use rocket_db_pools::{Connection, sqlx::{self, Row}};

use crate::guards::DBPool;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct GetChartSetResponse {
    chart_set_id: i32,
    title: String,
    artist: String,
}

#[get("/chart-set/<set_id>")]
pub async fn get_chart_set(
    mut db: Connection<DBPool>,
    set_id: i32,
) -> Option<Json<GetChartSetResponse>> {
    let query_results =
        sqlx::query("SELECT chart_set_id, title, artist FROM chart_sets WHERE chart_set_id = $1")
            .bind(set_id)
            // TODO properly error 500 instead of defaulting to 404
            .fetch_one(&mut **db)
            .await
            .ok()?;
    let chart_set_id: i32 = query_results.get("chart_set_id");
    let title: String = query_results.get("title");
    let artist: String = query_results.get("artist");
    Some(Json(GetChartSetResponse {
        title,
        artist,
        chart_set_id,
    }))
}

