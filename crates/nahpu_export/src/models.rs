use serde::{Deserialize, Serialize};
use nahpu_db::types::nahpu_sqlite::{Narrative, Site, CollEvent, Specimen};

/// Contains the root structure representing all exportable database records.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExportData {
    /// Collection of narrative records.
    pub narrative: Option<Vec<Narrative>>,
    /// Collection of site records.
    pub sites: Option<Vec<Site>>,
    /// Collection of collecting event records.
    pub events: Option<Vec<CollEvent>>,
    /// Collection of specimen records.
    pub specimens: Option<Vec<Specimen>>,
}
