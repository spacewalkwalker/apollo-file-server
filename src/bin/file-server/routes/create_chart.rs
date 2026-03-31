use chrono::Utc;
use rocket::{
    FromForm, Responder, form::Form, fs::TempFile, post, serde::json::Json, tokio::io::AsyncReadExt,
};
use rocket_db_pools::{
    Connection,
    sqlx::{self, Row},
};

use crate::{
    guards::{DBPool, RequestID, UploadAuth},
    metadata,
    routes::get_chart::ChartObject,
    storage::FileStoreHandle,
};

#[derive(FromForm)]
struct CreateChartForm<'a> {
    chart_set_id: i32,
    designator: &'a str,
    metadata: Json<metadata::ChartMetadata>,
    source_chart: Option<i32>,
    file: Option<TempFile<'a>>,
}

#[derive(Responder)]
enum CreateChartResponse {
    #[response(status = 500)]
    InternalError(()),
    #[response(status = 422)]
    Unprocessable(()),
    #[response(status = 400)]
    InvalidRequest(()),
    #[response(status = 201)]
    Created(Json<ChartObject>),
}

macro_rules! stdout_log {
    ($($val:expr),*) => {
        {
            let ctime = Utc::now().format("%Y-%m-%d %H:%M:%S:%f");
            println!("[{0}] {1}", ctime, format!($($val),*));
        }
    }
}

#[post("/chart", data = "<data>")]
pub async fn create_chart(
    mut db: Connection<DBPool>,
    id: RequestID,
    file_store: &FileStoreHandle,
    data: Form<CreateChartForm<'_>>,
    _auth: UploadAuth,
) -> CreateChartResponse {

    stdout_log!("[req {}] POST /chart", id);

    // TODO the request may fail. what do we do to the file in the bucket?
    let uploaded_handle = if let Some(chart_id) = data.source_chart {
        stdout_log!("[req {}] validating pre-existing chart id {}", id, chart_id);
        match sqlx::query("SELECT file_id FROM charts WHERE chart_id = $1")
            .bind(chart_id)
            .fetch_optional(&mut **db)
            .await
        {
            Ok(Some(row)) => row.get("file_id"),
            Ok(None) => {
                return CreateChartResponse::Unprocessable(());
            }
            Err(_) => {
                return CreateChartResponse::InternalError(());
            }
        }
    } else if let Some(file) = &data.file {
        let Ok(mut chart_file_data_source) = file.open().await else {
            return CreateChartResponse::InternalError(());
        };

        let mut chart_file_data = Vec::new();
        let Ok(_) = chart_file_data_source
            .read_to_end(&mut chart_file_data)
            .await
        else {
            return CreateChartResponse::InternalError(());
        };

        stdout_log!("[req {}] uploading new file", id);
        match file_store.store(chart_file_data).await {
            Ok(handle) => {
                stdout_log!("[req {}] file uploaded under handle {}", id, handle);
                handle
            }
            Err(error) => {
                return CreateChartResponse::InternalError(());
            }
        }
    } else {
        return CreateChartResponse::InvalidRequest(());
    };

    let chart_set =
        match sqlx::query("SELECT title, artist FROM chart_sets WHERE chart_set_id = $1")
            .bind(data.chart_set_id)
            .fetch_one(&mut **db)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                eprintln!("Error querying for existing chart set: {:?}", error);
                return CreateChartResponse::InternalError(());
            }
        };

    let metadata = sqlx::types::Json(data.metadata.0.clone());

    let insert_query_results = match sqlx::query(
        "INSERT INTO charts (chart_set_id, designator, file_id, metadata)
        VALUES ($1, $2, $3, $4) RETURNING chart_id",
    )
    .bind(data.chart_set_id)
    .bind(data.designator)
    .bind(&uploaded_handle)
    .bind(metadata)
    .fetch_one(&mut **db)
    .await
    {
        Ok(result) => result,
        Err(sqlx::error::Error::Database(database_err))
            // check if this is a foreign key violation
            // which can only happen if the provided chart_set_id is not in the database
            if database_err.code().map(|code| code.into_owned()) == Some("23503".to_string()) =>
        {
            return CreateChartResponse::Unprocessable(());
        }
        Err(error) => {
            eprintln!("Error when storing row in database: {:?}", error);
            return CreateChartResponse::InternalError(());
        }
    };


    let chart_id = insert_query_results.get("chart_id");

    stdout_log!("[req {}] inserted entry in DB with ID {}", id, chart_id);

    CreateChartResponse::Created(Json(ChartObject {
        song_title: chart_set.get("title"),
        song_artist: chart_set.get("artist"),
        chart_set_id: data.chart_set_id,
        chart_id,
        designator: data.designator.to_string(),
        metadata: data.metadata.0.clone(),
    }))
}
