use nahpu_db::types::nahpu_sqlite::{CollEvent, Geography, Narrative, Site, Specimen};
use serde::{Deserialize, Serialize};

/// Contains the root structure representing all exportable database records.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExportData {
    /// Collection of narrative records.
    pub narrative: Option<Vec<Narrative>>,
    /// Collection of site records.
    pub sites: Option<Vec<Site>>,
    /// Collection of geography records referenced by sites.
    pub geographies: Option<Vec<Geography>>,
    /// Collection of collecting event records.
    pub events: Option<Vec<CollEvent>>,
    /// Collection of specimen records.
    pub specimens: Option<Vec<Specimen>>,
}
