//! # Darwin Core Mapper
//!
//! This module contains the mapping logic from the Nahpu database schema
//! to the Darwin Core standard terms. This mapping is manually defined based on
//! the `tables_dwc_mapping.md` documentation.

/// A utility struct for mapping Nahpu schema names to Darwin Core terms.
pub struct DwcMapper;

impl DwcMapper {
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
            "narrativeMedia" => Self::map_narrative_media_column(column_name),
            "siteMedia" => Self::map_site_media_column(column_name),
            "specimenMedia" => Self::map_specimen_media_column(column_name),
            "personnelList" => Self::map_personnel_list_column(column_name),
            "weather" => Self::map_weather_column(column_name),
            "mammalMeasurement" => Self::map_mammal_measurement_column(column_name),
            "avianMeasurement" => Self::map_avian_measurement_column(column_name),
            _ => None,
        }
    }

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
            "siteType" => Some("dwc:locationRemarks"),
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
            "siteId" => Some("dwc:locationID"),
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
            "projectUuid" => Some("dwc:datasetID"),
            "siteId" => Some("dwc:locationID"),
            "startDate" | "endDate" => Some("dwc:eventDate"),
            "startTime" | "endTime" => Some("dwc:eventTime"),
            "primaryCollMethod" => Some("dwc:samplingProtocol"),
            "collMethodNotes" => Some("dwc:samplingEffort"),
            _ => None,
        }
    }

    fn map_coll_personnel_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "eventId" => Some("dwc:eventID"),
            "personnelId" => Some("dwc:recordedByID"),
            "name" => Some("dwc:recordedBy"),
            _ => None,
        }
    }

    fn map_coll_effort_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "eventId" => Some("dwc:eventID"),
            "method" | "brand" => Some("dwc:samplingProtocol"),
            "count" | "size" => Some("dwc:sampleSizeValue"),
            "notes" => Some("dwc:samplingEffort"),
            _ => None,
        }
    }

    fn map_narrative_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "projectUuid" => Some("dwc:datasetID"),
            "siteId" => Some("dwc:locationID"),
            "date" => Some("dcterms:date"),
            "narrative" => Some("dwc:eventRemarks"),
            _ => None,
        }
    }

    fn map_media_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "primaryId" | "secondaryId" => Some("dcterms:identifier"),
            "projectUuid" => Some("dwc:datasetID"),
            "category" => Some("dcterms:type"),
            "tag" => Some("dcterms:subject"),
            "taken" => Some("dcterms:created"),
            "camera" | "lenses" | "additionalExif" => Some("dcterms:description"),
            "personnelId" => Some("dcterms:creator"),
            "fileName" => Some("dcterms:title"),
            "caption" => Some("dcterms:description"),
            _ => None,
        }
    }

    fn map_associated_data_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
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
            "name" | "initial" => Some("dwc:recordedBy"),
            "affiliation" => Some("dwc:institutionCode"),
            "notes" => Some("dwc:measurementRemarks"),
            _ => None,
        }
    }

    fn map_taxonomy_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "id" => Some("dwc:taxonID"),
            "taxonClass" => Some("dwc:class"),
            "taxonOrder" => Some("dwc:order"),
            "taxonFamily" => Some("dwc:family"),
            "genus" => Some("dwc:genus"),
            "specificEpithet" => Some("dwc:specificEpithet"),
            "authors" => Some("dwc:scientificNameAuthorship"),
            "commonName" => Some("dwc:vernacularName"),
            "notes" => Some("dwc:taxonRemarks"),
            "citesStatus" | "redListCategory" | "countryStatus" => Some("dwc:threatStatus"),
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
            "prepDate" | "prepTime" => Some("dcterms:modified"),
            "collectionDate" | "captureDate" => Some("dwc:eventDate"),
            "collectionTime" | "captureTime" => Some("dwc:eventTime"),
            "trapType" | "methodId" | "collMethodId" => Some("dwc:samplingProtocol"),
            "coordinateId" => Some("dwc:locationID"),
            "catalogerId" | "collPersonnelId" => Some("dwc:recordedBy"),
            "fieldNumber" => Some("dwc:recordNumber"),
            "collEventId" => Some("dwc:eventID"),
            "museumId" => Some("dwc:institutionCode"),
            "preparatorId" => Some("dwc:preparations"),
            _ => None,
        }
    }

    fn map_specimen_part_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "personnelId" => Some("dwc:recordedBy"),
            "tissueId" => Some("dwc:materialSampleID"),
            "barcodeId" => Some("dwc:otherCatalogNumbers"),
            "type" | "treatment" | "additionalTreatment" => Some("dwc:preparations"),
            "count" => Some("dwc:individualCount"),
            "dateTaken" => Some("dwc:eventDate"),
            "timeTaken" => Some("dwc:eventTime"),
            "museumPermanent" | "museumLoan" => Some("dwc:disposition"),
            "remark" | "pmi" => Some("dwc:occurrenceRemarks"),
            _ => None,
        }
    }

    fn map_narrative_media_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "narrativeId" | "mediaId" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_site_media_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "siteId" => Some("dwc:locationID"),
            "mediaId" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_specimen_media_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "mediaId" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_personnel_list_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "projectUuid" => Some("dwc:datasetID"),
            "personnelUuid" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_weather_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "eventId" => Some("dwc:eventID"),
            "notes" => Some("dwc:eventRemarks"),
            _ => None,
        }
    }

    fn map_mammal_measurement_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "sex" => Some("dwc:sex"),
            "age" => Some("dwc:lifeStage"),
            "reproductiveStage" => Some("dwc:reproductiveCondition"),
            "remark" => Some("dwc:occurrenceRemarks"),
            _ => None,
        }
    }

    fn map_avian_measurement_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "sex" => Some("dwc:sex"),
            "specimenRemark" => Some("dwc:occurrenceRemarks"),
            "habitatRemark" => Some("dwc:habitat"),
            _ => None,
        }
    }
}
