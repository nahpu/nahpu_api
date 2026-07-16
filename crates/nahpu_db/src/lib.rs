pub mod io;
pub mod types;

/// Version of the compiled `nahpu_db` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::types::nahpu_sqlite::*;

    #[test]
    fn test_site_serialization() {
        let site = Site {
            id: 1,
            site_id: Some("Site-01".to_string()),
            project_uuid: Some("uuid-1234".to_string()),
            lead_staff_id: None,
            site_type: Some("Forest".to_string()),
            country: Some("USA".to_string()),
            state_province: Some("California".to_string()),
            county: None,
            municipality: None,
            media_id: None,
            locality: Some("Yosemite".to_string()),
            remark: None,
            habitat_type: None,
            habitat_condition: None,
            habitat_description: None,
        };

        let json = serde_json::to_string(&site).expect("Failed to serialize");
        assert!(json.contains("Site-01"));
        assert!(json.contains("siteId"));
    }
}
