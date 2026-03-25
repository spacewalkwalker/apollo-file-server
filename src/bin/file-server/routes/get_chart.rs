use rocket::{Responder, get, serde::{Serialize, json::Json}};
use rocket_db_pools::{Connection, sqlx::{self, Row}};

use crate::{guards::DBPool, metadata};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ChartObject {
    pub song_title: String,
    pub song_artist: String,
    pub chart_id: i32,
    pub chart_set_id: i32,
    pub designator: String,
    pub metadata: metadata::ChartMetadata,
}

#[derive(Responder)]
enum GetChartResponse {
    #[response(status = 500)]
    InternalError(()),
    #[response(status = 404)]
    NotFound(()),
    Success(Json<ChartObject>)
}

#[get("/chart/<chart_id>")]
pub async fn get_chart(mut db: Connection<DBPool>, chart_id: i32) -> GetChartResponse {
    let Ok(Some(query_result)) = sqlx::query("SELECT title, artist, chart_id, chart_set_id, designator, metadata FROM charts JOIN chart_sets ON charts.chart_set_id = chart_sets.chart_set_id WHERE chart_id = $1").bind(chart_id)
        .fetch_optional(&mut **db).await else {
        return GetChartResponse::InternalError(());
    };
    let chart_id: i32 = query_result.get("chart_id");
    let chart_set_id: i32 = query_result.get("chart_set_id");
    let designator: String = query_result.get("designator");
    let song_title: String = query_result.get("title");
    let song_artist: String = query_result.get("artist");
    let metadata: sqlx::types::Json<metadata::ChartMetadata> = query_result.get("metadata");
    GetChartResponse::Success(Json(ChartObject {
        chart_id,
        chart_set_id,
        designator,
        song_title,
        song_artist,
        metadata: metadata.0,
    }))
}
