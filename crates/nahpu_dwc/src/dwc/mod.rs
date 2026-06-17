//! # Darwin Core Mapper
//!
//! This module contains the mapping logic from the Nahpu database schema
//! to the Darwin Core standard terms. This mapping is manually defined based on
//! the `tables_dwc_mapping.md` documentation.

/// A utility struct for mapping Nahpu schema names to Darwin Core terms.
pub struct DwcMapper;

impl DwcMapper {
    // --- Private mapping functions for each table ---
    fn map_project_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "uuid" => Some("dcterms:identifier"),
            "name" => Some("dwc:datasetName"),
            "description" => Some("dwc:datasetDescription"),
            "principalInvestigator" => Some("dwc:recordedBy"),
            "location" => Some("dwc:location"),
            "startDate" | "endDate" => Some("dwc:eventDate"),
            "created" => Some("dcterms:created"),
            "lastAccessed" => Some("dcterms:modified"),
            _ => None,
        }
    }

    fn map_site_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "siteId" => Some("dwc:locationID"),
            "projectUuid" => Some("dwc:datasetID"),
            "leadStaffId" => Some("dwc:recordedBy"),
            "country" => Some("dwc:country"),
            "stateProvince" => Some("dwc:stateProvince"),
            "county" => Some("dwc:county"),
            "municipality" => Some("dwc:municipality"),
            "locality" => Some("dwc:verbatimLocality"),
            "remark" => Some("dwc:locationRemarks"),
            "habitatType" | "habitatCondition" | "habitatDescription" => Some("dwc:habitat"),
            _ => None,
        }
    }

    fn map_coordinate_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "decimalLatitude" => Some("dwc:decimalLatitude"),
            "decimalLongitude" => Some("dwc:decimalLongitude"),
            "elevationInMeter" => Some("dwc:minimumElevationInMeters"),
            "datum" => Some("dwc:geodeticDatum"),
            "uncertaintyInMeters" => Some("dwc:coordinateUncertaintyInMeters"),
            "gpsUnit" | "notes" => Some("dwc:georeferenceRemarks"),
            _ => None,
        }
    }

    fn map_coll_event_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "id" => Some("dwc:eventID"),
            "startDate" | "endDate" => Some("dwc:eventDate"),
            "startTime" | "endTime" => Some("dwc:eventTime"),
            "primaryCollMethod" => Some("dwc:samplingProtocol"),
            "collMethodNotes" => Some("dwc:samplingEffort"),
            _ => None,
        }
    }

    fn map_coll_personnel_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "personnelId" => Some("dwc:recordedByID"),
            "name" => Some("dwc:recordedBy"),
            _ => None,
        }
    }

    fn map_coll_effort_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "method" | "brand" => Some("dwc:samplingProtocol"),
            "count" | "size" => Some("dwc:sampleSizeValue"),
            "notes" => Some("dwc:samplingEffort"),
            _ => None,
        }
    }

    fn map_narrative_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "date" => Some("dcterms:date"),
            "narrative" => Some("dwc:eventRemarks"),
            _ => None,
        }
    }

    fn map_media_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "category" => Some("dcterms:type"),
            "taken" => Some("dcterms:created"),
            "personnelId" => Some("dcterms:creator"),
            "caption" => Some("dcterms:description"),
            _ => None,
        }
    }

    fn map_associated_data_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "name" => Some("dcterms:title"),
            "type" => Some("dcterms:type"),
            "date" => Some("dcterms:created"),
            "description" => Some("dcterms:description"),
            "url" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_personnel_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "uuid" => Some("dcterms:identifier"),
            "name" => Some("dwc:recordedBy"),
            "affiliation" => Some("dwc:institutionCode"),
            _ => None,
        }
    }

    fn map_taxonomy_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "taxonClass" => Some("dwc:class"),
            "taxonOrder" => Some("dwc:order"),
            "taxonFamily" => Some("dwc:family"),
            "genus" => Some("dwc:genus"),
            "specificEpithet" => Some("dwc:specificEpithet"),
            "authors" => Some("dwc:scientificNameAuthorship"),
            "commonName" => Some("dwc:vernacularName"),
            "notes" => Some("dwc:taxonRemarks"),
            "citesStatus" | "redListCategory" => Some("dwc:threatStatus"),
            _ => None,
        }
    }

    fn map_specimen_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "uuid" => Some("dwc:occurrenceID"),
            "projectUuid" => Some("dwc:datasetID"),
            "speciesId" => Some("dwc:taxonID"),
            "iDConfidence" => Some("dwc:identificationQualifier"),
            "iDMethod" => Some("dwc:identificationRemarks"),
            "taxonGroup" => Some("dwc:higherClassification"),
            "condition" => Some("dwc:disposition"),
            "prepDate" => Some("dcterms:modified"),
            "collectionDate" | "captureDate" => Some("dwc:eventDate"),
            "collectionTime" | "captureTime" => Some("dwc:eventTime"),
            "trapType" => Some("dwc:samplingProtocol"),
            "catalogerId" => Some("dwc:recordedBy"),
            "fieldNumber" => Some("dwc:recordNumber"),
            "collEventId" => Some("dwc:eventID"),
            "museumId" => Some("dwc:institutionCode"),
            "preparatorId" => Some("dwc:preparations"),
            _ => None,
        }
    }

    fn map_specimen_part_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "barcodeId" => Some("dwc:otherCatalogNumbers"),
            "type" | "treatment" => Some("dwc:preparations"),
            "count" => Some("dwc:individualCount"),
            "museumPermanent" | "museumLoan" => Some("dwc:disposition"),
            "remark" => Some("dwc:occurrenceRemarks"),
            _ => None,
        }
    }

    // --- Public entry point ---
    /// Maps a table and column name from the Nahpu schema to the corresponding Darwin Core term.
    pub fn get_dwc_term(table_name: &str, column_name: &str) -> Option<&'static str> {
        match table_name {
            "project" => Self::map_project_column(column_name),
            "site" => Self::map_site_column(column_name),
            "coordinate" => Self::map_coordinate_column(column_name),
            "collEvent" => Self::map_coll_event_column(column_name),
            "collPersonnel" => Self::map_coll_personnel_column(column_name),
            "collEffort" => Self::map_coll_effort_column(column_name),
            "narrative" => Self::map_narrative_column(column_name),
            "media" => Self::map_media_column(column_name),
            "associatedData" => Self::map_associated_data_column(column_name),
            "personnel" => Self::map_personnel_column(column_name),
            "taxonomy" => Self::map_taxonomy_column(column_name),
            "specimen" => Self::map_specimen_column(column_name),
            "specimenPart" => Self::map_specimen_part_column(column_name),
            _ => None,
        }
    }
}

/*
NOTE on Measurement Tables (`mammalMeasurement`, `avianMeasurement`, `weather`):
These tables are best represented using the Darwin Core MeasurementOrFact extension.
Each column (e.g., `totalLength`, `weight`, `lowestDayTempC`) maps to a `dwc:measurementValue`,
and requires corresponding `dwc:measurementType` (e.g., "totalLength", "mass", "temperature")
and `dwc:measurementUnit` (e.g., "mm", "g", "Celsius") terms.
Direct mapping in this function is not suitable for these extension-based tables.
The `dwc:sex`, `dwc:lifeStage`, and `dwc:reproductiveCondition` terms should be used for relevant columns.
*/
