pub mod io;
pub mod types;

/// Version of the compiled `nahpu_db` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::types::nahpu_sqlite::*;

    #[test]
    fn test_site_and_geography_serialization() {
        let site = Site {
            id: 1,
            site_id: Some("Site-01".to_string()),
            project_uuid: Some("uuid-1234".to_string()),
            lead_staff_id: None,
            site_type: Some("Forest".to_string()),
            geography_id: Some(10),
            media_id: None,
            remark: None,
        };
        let geography = Geography {
            id: 10,
            country: Some("USA".to_string()),
            island_group: None,
            state_province: Some("California".to_string()),
            county: None,
            municipality: None,
            locality: Some("Yosemite".to_string()),
            match_key: "usa|california|yosemite".to_string(),
        };

        let site_json = serde_json::to_string(&site).expect("Failed to serialize site");
        let geography_json =
            serde_json::to_string(&geography).expect("Failed to serialize geography");
        assert!(site_json.contains("Site-01"));
        assert!(site_json.contains("\"geographyId\":10"));
        assert!(geography_json.contains("Yosemite"));
        assert!(geography_json.contains("\"matchKey\""));
    }

    #[test]
    fn test_id_columns_round_trip_with_drift_names() {
        let specimen: Specimen = serde_json::from_str(
            r#"{"uuid": "s1", "speciesID": 3, "museumID": "MVZ 1", "collEventID": 7}"#,
        )
        .expect("specimen JSON with Drift column names should deserialize");
        assert_eq!(specimen.species_id, Some(3));
        assert_eq!(specimen.museum_id.as_deref(), Some("MVZ 1"));
        assert_eq!(specimen.coll_event_id, Some(7));
        let json = serde_json::to_string(&specimen).expect("specimen should serialize");
        assert!(json.contains("\"speciesID\":3"));
        assert!(json.contains("\"museumID\":\"MVZ 1\""));

        let site: Site = serde_json::from_str(r#"{"id": 1, "siteID": "Site-01"}"#)
            .expect("site JSON with Drift column names should deserialize");
        assert_eq!(site.site_id.as_deref(), Some("Site-01"));
        // Columns that already round-trip keep their camelCase names.
        assert!(
            serde_json::to_string(&site)
                .expect("site should serialize")
                .contains("\"projectUuid\"")
        );
    }
}
