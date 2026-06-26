use serde::{Deserialize, Serialize};
use nahpu_db::types::nahpu_sqlite::{Narrative, Site, CollEvent, Specimen};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExportData {
    pub narrative: Option<Vec<Narrative>>,
    pub sites: Option<Vec<Site>>,
    pub events: Option<Vec<CollEvent>>,
    pub specimens: Option<Vec<Specimen>>,
}
