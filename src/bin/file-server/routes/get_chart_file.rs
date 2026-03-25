use rocket::{Responder, get};
use rocket_db_pools::{Connection, sqlx::{self, Row}};

use crate::{guards::DBPool, storage::FileStoreHandle};

#[derive(Responder)]
enum GetChartFileResponse {
    #[response(status = 500)]
    InternalError(()),
    #[response(status = 404)]
    NotFound(()),
    Success(Vec<u8>)
}

#[get("/chart/<chart_id>/file")]
pub async fn get_chart_file(mut db: Connection<DBPool>, file_store: &FileStoreHandle, chart_id: i32) -> GetChartFileResponse {
    let Ok(query_result) = sqlx::query("SELECT file_id FROM charts WHERE chart_id = $1").bind(chart_id)
        .fetch_one(&mut **db).await else {
        return GetChartFileResponse::InternalError(());
    };
    let file_id: &str = query_result.get("file_id");
    let Ok(file_data) = file_store.retrieve(file_id).await else {
        return GetChartFileResponse::InternalError(());
    };
    match file_data {
        None => GetChartFileResponse::NotFound(()),
        Some(data) => {
            GetChartFileResponse::Success(data)
        }
    }
}
