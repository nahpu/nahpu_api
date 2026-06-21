use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoordinateData {
    pub name_id: String,
    pub notes: Option<String>,
    pub decimal_longitude: Option<f64>,
    pub decimal_latitude: Option<f64>,
    pub elevation_in_meter: Option<f64>,
}
