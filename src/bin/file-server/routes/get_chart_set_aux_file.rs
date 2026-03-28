use rocket::{
    Responder, get,
    http::{Header, hyper::header::CONTENT_DISPOSITION},
};
use rocket_db_pools::{
    Connection,
    sqlx::{self, Row},
};

use crate::{guards::DBPool, storage::FileStoreHandle};

#[derive(Responder)]
enum GetChartSetAuxFileResponse {
    #[response(status = 500)]
    InternalError(()),
    #[response(status = 404)]
    NotFound(()),
    Success(Vec<u8>, Header<'static>),
}

#[get("/chart-set/<chart_set_id>/file/<label>")]
pub async fn get_chart_set_aux_file(
    mut db: Connection<DBPool>,
    file_store: &FileStoreHandle,
    chart_set_id: i32,
    label: &str,
) -> GetChartSetAuxFileResponse {
    let Ok(query_result) = sqlx::query(
        "SELECT filename, file_id FROM chart_set_aux_files WHERE chart_set_id = $1 AND label = $2",
    )
    .bind(chart_set_id)
    .bind(label)
    .fetch_optional(&mut **db)
    .await
    else {
        return GetChartSetAuxFileResponse::InternalError(());
    };
    match query_result {
        Some(row) => {
            let file_id: &str = row.get("file_id");
            let filename: Option<String> = row.get("filename");
            let content_header = Header::new(
                CONTENT_DISPOSITION.as_str(),
                match filename {
                    None => "attachment".to_string(),
                    Some(filename) => format!("attachment; filename=\"{filename}\""),
                },
            );
            match file_store.retrieve(&file_id).await {
                Err(_) => GetChartSetAuxFileResponse::InternalError(()),
                Ok(None) => GetChartSetAuxFileResponse::NotFound(()),
                Ok(Some(file_data)) => {
                    GetChartSetAuxFileResponse::Success(file_data, content_header)
                }
            }
        }
        None => GetChartSetAuxFileResponse::NotFound(()),
    }
}
