use std::collections::HashMap;

use rocket::{Responder, get, serde::{Serialize, Deserialize, json::Json}};
use rocket_db_pools::{Connection, sqlx::{self, Row}};

use crate::{guards::DBPool, metadata::{self, ChartMetadata, MetadataValue}, routes::get_chart::ChartObject};

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Default, Deserialize)]
#[serde(tag = "SortOrder", content = "Direction", crate = "rocket::serde")]
enum SortOrder<'a> {
    #[default]
    TitleAlphabetical,
    ByMetaTag(SortDirection, &'a str),
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct ChartSearchParams<'a> {
    sort: Option<SortOrder<'a>>,
    count: Option<i32>,
    offset: Option<i32>,
    designator: Option<String>,
    #[serde(borrow)]
    filter_tags: Option<HashMap<&'a str, MetadataValue>>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct SearchSuccessResponse {
    result_count: i64,
    results: Vec<ChartObject>
}

#[derive(Responder)]
enum SearchChartResponse {
    #[response(status = 500)]
    InternalError(()),
    #[response(status = 200)]
    Ok(Json<SearchSuccessResponse>),
}

#[get("/search-charts", data = "<search_criteria>")]
pub async fn search_charts(
    mut db: Connection<DBPool>,
    search_criteria: Json<ChartSearchParams<'_>>,
) -> SearchChartResponse {
    let search_criteria = search_criteria.0;

    let sort_condition: &'static str = match &search_criteria.sort {
        None => "",
        Some(SortOrder::TitleAlphabetical) => "title",
        Some(SortOrder::ByMetaTag(direction, _)) => match direction {
            SortDirection::Ascending => "metadata->$4 ASC",
            SortDirection::Descending => "metadata->$4 DESC",
        },
    };
    let query = format!(
        "SELECT title, artist, chart_id, charts.chart_set_id, designator, metadata,
            COUNT(*) OVER() AS full_count         
        FROM charts
        JOIN chart_sets ON charts.chart_set_id = chart_sets.chart_set_id
        WHERE 
            $1 <@ metadata AND
            CASE
                WHEN $5 IS NOT NULL THEN designator = $5
                ELSE TRUE
            END
        OFFSET $2
        LIMIT $3
        {sort_condition}
    "
    );
    let query_results = match sqlx::query(&query)
        .bind(sqlx::types::Json(
            search_criteria.filter_tags.unwrap_or_default(),
        ))
        .bind(search_criteria.offset)
        .bind(search_criteria.count.unwrap_or(50))
        .bind(match search_criteria.sort {
            Some(SortOrder::ByMetaTag(_, tag)) => Some(tag),
            _ => None,
        })
        .bind(search_criteria.designator)
        .fetch_all(&mut **db)
        .await
    {
        Ok(results) => results,
        Err(error) => {
            eprintln!("DB error: {error}");
            return SearchChartResponse::InternalError(());
        }
    };

    let mut result_count = 0;

    let results = query_results
        .into_iter()
        .map(|query_row| {
            result_count = query_row.get("full_count");
            let metadata: sqlx::types::Json<metadata::ChartMetadata> = query_row.get("metadata");
            ChartObject {
                song_title: query_row.get("title"),
                song_artist: query_row.get("artist"),
                chart_id: query_row.get("chart_id"),
                chart_set_id: query_row.get("chart_set_id"),
                designator: query_row.get("designator"),
                metadata: metadata.0,
            }
        })
        .collect();


    SearchChartResponse::Ok(Json(SearchSuccessResponse { result_count, results }))
}

