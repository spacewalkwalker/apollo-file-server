use rocket::{
    FromForm, Responder,
    form::Form,
    fs::TempFile,
    http::{Header, hyper::header::LOCATION},
    post,
    tokio::io::AsyncReadExt,
    uri,
};
use rocket_db_pools::{Connection, sqlx};

use crate::{
    guards::{DBPool, UploadAuth}, routes::get_chart_set_aux_file::rocket_uri_macro_get_chart_set_aux_file,
    storage::FileStoreHandle,
};

#[derive(FromForm)]
struct CreateChartSetAuxFileForm<'a> {
    file: TempFile<'a>,
}
#[derive(Responder)]
enum CreateChartSetAuxFileResponse {
    #[response(status = 500)]
    InternalError(()),
    #[response(status = 404)]
    NotFound(()),
    #[response(status = 201)]
    Created((), Header<'static>),
}

#[post("/chart-set/<chart_set_id>/file/<label>", data = "<data>")]
pub async fn create_chart_set_aux_file(
    mut db: Connection<DBPool>,
    file_store: &FileStoreHandle,
    _auth: UploadAuth,
    chart_set_id: i32,
    label: &str,
    data: Form<CreateChartSetAuxFileForm<'_>>,
) -> CreateChartSetAuxFileResponse {
    let mut file_data = Vec::new();
    let Ok(mut file_data_source) = data.file.open().await else {
        return CreateChartSetAuxFileResponse::InternalError(());
    };
    let Ok(_) = file_data_source.read_to_end(&mut file_data).await else {
        return CreateChartSetAuxFileResponse::InternalError(());
    };
    let file_id = match file_store.store(file_data).await {
        Ok(handle) => handle,
        Err(_) => {
            return CreateChartSetAuxFileResponse::InternalError(());
        }
    };
    let file_name = {
        let name = data.file.name().unwrap_or("");
        match data.file.content_type().and_then(|ctype| ctype.extension()) {
            Some(ext) => format!("{name}.{ext}"),
            None => name.to_string()
        }
    };
    match sqlx::query(
        "INSERT INTO chart_set_aux_files (chart_set_id, label, filename, file_id) VALUES ($1, $2, $3, $4)")
        .bind(chart_set_id)
        .bind(&label)
        .bind(file_name)
        .bind(file_id)
        .execute(&mut **db).await {
        Err(sqlx::error::Error::Database(database_err))
            // check if this is a foreign key violation
            // which can only happen if the provided chart_set_id is not in the database
            if database_err.code().map(|code| code.into_owned()) == Some("23503".to_string()) =>
        {
            return CreateChartSetAuxFileResponse::NotFound(());
        }
        Err(_) => {
            return CreateChartSetAuxFileResponse::InternalError(());
        }
        Ok(_) => {
            let uri = uri!(get_chart_set_aux_file(chart_set_id = chart_set_id, label = label));
            let created_header = Header::new(LOCATION.as_str(), uri.to_string());
            return CreateChartSetAuxFileResponse::Created((), created_header);
        }
    }
}
