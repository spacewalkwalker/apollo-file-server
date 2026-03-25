use std::collections::HashMap;
use rocket::serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged, crate = "rocket::serde")]
pub enum MetadataValue {
    Text(String),
    Number(f64)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ChartMetadata(pub HashMap<String, MetadataValue>);

