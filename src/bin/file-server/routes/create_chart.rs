use rocket::{FromForm, Responder, form::Form, fs::TempFile, post, serde::json::Json, tokio::io::AsyncReadExt};
use rocket_db_pools::{Connection, sqlx::{self, Row}};

use crate::{guards::{DBPool, UploadAuth}, metadata, routes::get_chart::ChartObject, storage::FileStoreHandle};

#[derive(FromForm)]
struct CreateChartForm<'a> {
    chart_set_id: i32,
    designator: &'a str,
    metadata: Json<metadata::ChartMetadata>,
    file: TempFile<'a>,
}

#[derive(Responder)]
enum CreateChartResponse {
    #[response(status = 500)]
    InternalError(()),
    #[response(status = 404)]
    NotFound(()),
    #[response(status = 201)]
    Created(Json<ChartObject>),
}

#[post("/chart", data = "<data>")]
pub async fn create_chart(
    mut db: Connection<DBPool>,
    file_store: &FileStoreHandle,
    data: Form<CreateChartForm<'_>>,
    _auth: UploadAuth
) -> CreateChartResponse {
    eprintln!("create_chart request opened");
    let Ok(mut chart_file_data_source) = data.file.open().await else {
        return CreateChartResponse::InternalError(());
    };

    let mut chart_file_data = Vec::new();
    let Ok(_) = chart_file_data_source
        .read_to_end(&mut chart_file_data)
        .await
    else {
        return CreateChartResponse::InternalError(());
    };

    eprintln!("create_chart file read");

    // TODO the request may fail. what do we do to the file in the bucket?

    let uploaded_handle = match file_store.store(chart_file_data).await {
        Ok(handle) => handle,
        Err(error) => {
            eprintln!("Error when storing file: {}", error);
            return CreateChartResponse::InternalError(());
        }
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
            return CreateChartResponse::NotFound(());
        }
        Err(error) => {
            eprintln!("Error when storing row in database: {:?}", error);
            return CreateChartResponse::InternalError(());
        }
    };

    eprintln!("entry added to database");

    let chart_id = insert_query_results.get("chart_id");

    CreateChartResponse::Created(Json(ChartObject {
        song_title: chart_set.get("title"),
        song_artist: chart_set.get("artist"),
        chart_set_id: data.chart_set_id,
        chart_id,
        designator: data.designator.to_string(),
        metadata: data.metadata.0.clone(),
    }))
}

