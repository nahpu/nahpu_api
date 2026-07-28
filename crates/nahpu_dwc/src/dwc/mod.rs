//! # Darwin Core Mapper
//!
//! This module contains the mapping logic from the Nahpu database schema
//! to the Darwin Core standard terms. This mapping is manually defined based on
//! the [Persistence data documentation](https://nahpu.app/en/contributing/code/database/).

/// A utility struct for mapping Nahpu schema names to Darwin Core terms.
pub struct DwcMapper;

/// Describes how one NAHPU source field is represented in a flat Darwin Core
/// export. A source can emit more than one column (for example a measurement
/// emits type, value, and unit) and visible headers are intentionally allowed
/// to repeat between source fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DwcMapping {
    pub headers: Vec<&'static str>,
    pub measurement_type: Option<&'static str>,
    pub measurement_unit: Option<&'static str>,
}

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
            "mammalAttribute" => Self::map_mammal_attribute_column(column_name),
            "birdAttribute" => Self::map_bird_attribute_column(column_name),
            _ => None,
        }
    }

    /// Resolves a NAHPU source key in `table::field` form to a Darwin Core term.
    ///
    /// This is the preferred entry point for flat exports because it keeps the
    /// source-key parsing and table aliases in the Darwin Core crate.
    pub fn get_dwc_term_for_source_key(source_key: &str) -> Option<&'static str> {
        let (table_name, column_name) = source_key.split_once("::")?;
        let table_name = match table_name {
            "event" => "collEvent",
            "mammalMeasurement" => "mammalAttribute",
            "avianMeasurement" => "birdAttribute",
            "herpMeasurement" => "herpAttribute",
            table_name => table_name,
        };
        Self::get_dwc_term(table_name, column_name)
    }

    /// Resolves a source key to its complete tabular Darwin Core mapping.
    ///
    /// Most sources emit a single direct term. Measurements deliberately emit
    /// a repeated MeasurementOrFact column group so that every selected NAHPU
    /// measurement retains its type and unit in a flat export.
    pub fn get_dwc_mapping_for_source_key(source_key: &str) -> Option<DwcMapping> {
        if let Some(mapping) = Self::measurement_mapping(source_key) {
            return Some(mapping);
        }

        match source_key {
            "coordinate::elevationInMeter" => Some(DwcMapping {
                headers: vec![
                    "dwc:minimumElevationInMeters",
                    "dwc:maximumElevationInMeters",
                ],
                measurement_type: None,
                measurement_unit: None,
            }),
            "specimenPart::type" => Some(DwcMapping {
                headers: vec!["dwc:materialEntityType", "dwc:objectQuantityType"],
                measurement_type: None,
                measurement_unit: None,
            }),
            "specimenPart::count" => Some(DwcMapping {
                headers: vec!["dwc:objectQuantity"],
                measurement_type: None,
                measurement_unit: None,
            }),
            _ => Self::get_dwc_term_for_source_key(source_key).map(|header| DwcMapping {
                headers: vec![header],
                measurement_type: None,
                measurement_unit: None,
            }),
        }
    }

    fn measurement_mapping(source_key: &str) -> Option<DwcMapping> {
        let legacy_alias = match source_key.split_once("::") {
            Some(("mammalMeasurement", column)) => Some(format!("mammalAttribute::{column}")),
            Some(("avianMeasurement", column)) => Some(format!("birdAttribute::{column}")),
            Some(("herpMeasurement", column)) => Some(format!("herpAttribute::{column}")),
            _ => None,
        };
        let source_key = legacy_alias.as_deref().unwrap_or(source_key);
        let (measurement_type, measurement_unit) = match source_key {
            "mammalAttribute::totalLength" => ("total length", Some("mm")),
            "mammalAttribute::tailLength" => ("tail length", Some("mm")),
            "mammalAttribute::hindFootLength" => ("hind foot length", Some("mm")),
            "mammalAttribute::earLength" => ("ear length", Some("mm")),
            "mammalAttribute::forearm" => ("forearm length", Some("mm")),
            "mammalAttribute::tibia" => ("tibia length", Some("mm")),
            "mammalAttribute::weight" => ("weight", Some("g")),
            "mammalAttribute::frequencyMax" => ("maximum frequency", Some("kHz")),
            "mammalAttribute::frequencyMin" => ("minimum frequency", Some("kHz")),
            "mammalAttribute::frequencyAtMaxEnergy" => ("frequency at maximum energy", Some("kHz")),
            "mammalAttribute::duration" => ("echolocation duration", Some("s")),
            "mammalAttribute::testisPosition" => ("testis position", None),
            "mammalAttribute::testisLength" => ("testis length", Some("mm")),
            "mammalAttribute::testisWidth" => ("testis width", Some("mm")),
            "mammalAttribute::epididymisAppearance" => ("epididymis appearance", None),
            "mammalAttribute::leftPlacentalScars" => ("left placental scars", None),
            "mammalAttribute::rightPlacentalScars" => ("right placental scars", None),
            "mammalAttribute::mammaeCondition" => ("mammae condition", None),
            "mammalAttribute::mammaeInguinalCount" => ("inguinal mammae count", None),
            "mammalAttribute::mammaeAxillaryCount" => ("axillary mammae count", None),
            "mammalAttribute::mammaeAbdominalCount" => ("abdominal mammae count", None),
            "mammalAttribute::vaginaOpening" => ("vagina opening", None),
            "mammalAttribute::pubicSymphysis" => ("pubic symphysis", None),
            "mammalAttribute::embryoLeftCount" => ("left embryo count", None),
            "mammalAttribute::embryoRightCount" => ("right embryo count", None),
            "mammalAttribute::embryoCR" => ("embryo crown-rump length", Some("mm")),
            "mammalAttribute::echolocation" => ("echolocation", None),
            "birdAttribute::weight" => ("weight", Some("g")),
            "birdAttribute::wingspan" => ("wingspan", Some("mm")),
            "birdAttribute::bursaWidth" => ("bursa width", Some("mm")),
            "birdAttribute::bursaLength" => ("bursa length", Some("mm")),
            "birdAttribute::testisLength" => ("testis length", Some("mm")),
            "birdAttribute::testisWidth" => ("testis width", Some("mm")),
            "birdAttribute::ovaryLength" => ("ovary length", Some("mm")),
            "birdAttribute::ovaryWidth" => ("ovary width", Some("mm")),
            "birdAttribute::oviductWidth" => ("oviduct width", Some("mm")),
            "birdAttribute::firstOvaSize" => ("first ova size", Some("mm")),
            "birdAttribute::secondOvaSize" => ("second ova size", Some("mm")),
            "birdAttribute::thirdOvaSize" => ("third ova size", Some("mm")),
            "birdAttribute::skullOssification" => ("skull ossification", Some("%")),
            "birdAttribute::irisColor" => ("iris color", None),
            "birdAttribute::irisHex" => ("iris color hex", None),
            "birdAttribute::billColor" => ("bill color", None),
            "birdAttribute::billHex" => ("bill color hex", None),
            "birdAttribute::maxillaColor" => ("maxilla color", None),
            "birdAttribute::maxillaHex" => ("maxilla color hex", None),
            "birdAttribute::mandibleColor" => ("mandible color", None),
            "birdAttribute::mandibleHex" => ("mandible color hex", None),
            "birdAttribute::toeColor" | "birdAttribute::footColor" => ("toe color", None),
            "birdAttribute::toeHex" | "birdAttribute::footHex" => ("toe color hex", None),
            "birdAttribute::tarsusColor" => ("tarsus color", None),
            "birdAttribute::tarsusHex" => ("tarsus color hex", None),
            "birdAttribute::broodPatch" => ("brood patch", None),
            "birdAttribute::hasBursa" => ("bursa present", None),
            "birdAttribute::fat" => ("fat score", None),
            "birdAttribute::stomachContent" => ("stomach content", None),
            "birdAttribute::testisRemark" => ("testis remarks", None),
            "birdAttribute::ovaryAppearance" => ("ovary appearance", None),
            "birdAttribute::oviductAppearance" => ("oviduct appearance", None),
            "birdAttribute::ovaryRemark" => ("ovary remarks", None),
            "birdAttribute::wingIsMolt" => ("wing molt present", None),
            "birdAttribute::wingMolt" => ("wing molt", None),
            "birdAttribute::tailIsMolt" => ("tail molt present", None),
            "birdAttribute::tailMolt" => ("tail molt", None),
            "birdAttribute::bodyMolt" => ("body molt", None),
            "birdAttribute::moltRemark" => ("molt remarks", None),
            "herpAttribute::weight" => ("weight", Some("g")),
            "herpAttribute::svl" => ("snout-vent length", Some("cm")),
            "weather::lowestDayTempC" => ("lowest day temperature", Some("°C")),
            "weather::highestDayTempC" => ("highest day temperature", Some("°C")),
            "weather::lowestNightTempC" => ("lowest night temperature", Some("°C")),
            "weather::highestNightTempC" => ("highest night temperature", Some("°C")),
            "weather::averageHumidity" => ("average humidity", Some("%")),
            "weather::dewPointTemp" => ("dew point temperature", Some("°C")),
            "weather::sunriseTime" => ("sunrise", Some("hh:mm:ss")),
            "weather::sunsetTime" => ("sunset", Some("hh:mm:ss")),
            "weather::moonPhase" => ("moon phase", None),
            _ => return None,
        };
        Some(DwcMapping {
            headers: vec![
                "dwc:measurementType",
                "dwc:measurementValue",
                "dwc:measurementUnit",
            ],
            measurement_type: Some(measurement_type),
            measurement_unit,
        })
    }

    fn map_project_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "uuid" => Some("dcterms:identifier"),
            "name" => Some("dwc:datasetName"),
            "startDate" | "endDate" => Some("dwc:eventDate"),
            "created" => Some("dcterms:created"),
            "lastAccessed" => Some("dcterms:modified"),
            _ => None,
        }
    }

    fn map_site_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "siteID" | "siteId" => Some("dwc:siteNumber"),
            "projectUuid" => Some("dwc:datasetID"),
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
            "siteID" | "siteId" => Some("dwc:locationID"),
            "decimalLatitude" => Some("dwc:decimalLatitude"),
            "decimalLongitude" => Some("dwc:decimalLongitude"),
            "elevationInMeter" => Some("dwc:minimumElevationInMeters"),
            "datum" => Some("dwc:geodeticDatum"),
            "uncertaintyInMeters" => Some("dwc:coordinateUncertaintyInMeters"),
            "notes" => Some("dwc:georeferenceRemarks"),
            _ => None,
        }
    }

    fn map_coll_event_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "id" => Some("dwc:eventID"),
            "projectUuid" => Some("dwc:datasetID"),
            "siteID" | "siteId" => Some("dwc:locationID"),
            "startDate" | "endDate" => Some("dwc:eventDate"),
            "startTime" | "endTime" => Some("dwc:eventTime"),
            "primaryCollMethod" => Some("dwc:samplingProtocol"),
            "collMethodNotes" => Some("dwc:samplingEffort"),
            "personnel" => Some("dwc:recordedBy"),
            _ => None,
        }
    }

    fn map_coll_personnel_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "eventID" | "eventId" => Some("dwc:eventID"),
            "personnelId" => Some("dwc:recordedByID"),
            "name" => Some("dwc:recordedBy"),
            _ => None,
        }
    }

    fn map_coll_effort_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "eventID" | "eventId" => Some("dwc:eventID"),
            "method" | "brand" => Some("dwc:samplingProtocol"),
            "notes" => Some("dwc:samplingEffort"),
            _ => None,
        }
    }

    fn map_narrative_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "projectUuid" => Some("dwc:datasetID"),
            "siteID" | "siteId" => Some("dwc:locationID"),
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
            _ => None,
        }
    }

    fn map_specimen_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "uuid" => Some("dwc:occurrenceID"),
            "projectUuid" => Some("dwc:datasetID"),
            "speciesID" | "speciesId" => Some("dwc:taxonID"),
            "scientificName" => Some("dwc:scientificName"),
            "iDMethod" => Some("dwc:identificationType"),
            "taxonGroup" => Some("dwc:higherClassification"),
            "collectionDate" | "captureDate" => Some("dwc:eventDate"),
            "collectionTime" | "captureTime" => Some("dwc:eventTime"),
            "trapType" | "methodID" | "methodId" | "collMethodID" | "collMethodId" => {
                Some("dwc:samplingProtocol")
            }
            "coordinateID" | "coordinateId" => Some("dwc:locationID"),
            "catalogerID" | "catalogerId" | "collPersonnelID" | "collPersonnelId" => {
                Some("dwc:recordedBy")
            }
            "fieldNumber" => Some("dwc:recordNumber"),
            "collEventID" | "collEventId" => Some("dwc:eventID"),
            "museumID" | "museumId" => Some("dwc:institutionCode"),
            "preparatorID" | "preparatorId" => Some("dwc:recordedBy"),
            _ => None,
        }
    }

    fn map_specimen_part_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "tissueID" | "tissueId" => Some("dwc:materialSampleID"),
            "barcodeID" | "barcodeId" => Some("dwc:otherCatalogNumbers"),
            "treatment" | "additionalTreatment" => Some("dwc:preparations"),
            "count" => Some("dwc:objectQuantity"),
            "dateTaken" => Some("dwc:eventDate"),
            "timeTaken" => Some("dwc:eventTime"),
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
            "eventID" | "eventId" => Some("dwc:eventID"),
            "notes" => Some("dwc:eventRemarks"),
            _ => None,
        }
    }

    fn map_mammal_attribute_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "sex" => Some("dwc:sex"),
            "age" => Some("dwc:lifeStage"),
            "reproductiveStage" => Some("dwc:reproductiveCondition"),
            "remark" => Some("dwc:occurrenceRemarks"),
            _ => None,
        }
    }

    fn map_bird_attribute_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "sex" => Some("dwc:sex"),
            "specimenRemark" => Some("dwc:occurrenceRemarks"),
            "habitatRemark" => Some("dwc:habitat"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DwcMapper;

    const CURRENT_DWC_TERMS_USED_BY_NAHPU: &[&str] = &[
        "dwc:class",
        "dwc:coordinateUncertaintyInMeters",
        "dwc:country",
        "dwc:county",
        "dwc:datasetID",
        "dwc:datasetName",
        "dwc:decimalLatitude",
        "dwc:decimalLongitude",
        "dwc:eventDate",
        "dwc:eventID",
        "dwc:eventRemarks",
        "dwc:eventTime",
        "dwc:family",
        "dwc:genus",
        "dwc:geodeticDatum",
        "dwc:georeferenceRemarks",
        "dwc:habitat",
        "dwc:higherClassification",
        "dwc:identificationType",
        "dwc:institutionCode",
        "dwc:lifeStage",
        "dwc:locationID",
        "dwc:locationRemarks",
        "dwc:materialSampleID",
        "dwc:measurementRemarks",
        "dwc:minimumElevationInMeters",
        "dwc:municipality",
        "dwc:occurrenceID",
        "dwc:occurrenceRemarks",
        "dwc:order",
        "dwc:otherCatalogNumbers",
        "dwc:objectQuantity",
        "dwc:preparations",
        "dwc:recordNumber",
        "dwc:recordedBy",
        "dwc:recordedByID",
        "dwc:reproductiveCondition",
        "dwc:samplingEffort",
        "dwc:samplingProtocol",
        "dwc:scientificNameAuthorship",
        "dwc:sex",
        "dwc:siteNumber",
        "dwc:specificEpithet",
        "dwc:stateProvince",
        "dwc:taxonID",
        "dwc:taxonRemarks",
        "dwc:verbatimLocality",
        "dwc:vernacularName",
    ];

    #[test]
    fn mapped_dwc_terms_match_the_current_official_term_names() {
        let source_keys = [
            "project::name",
            "project::startDate",
            "site::siteID",
            "site::projectUuid",
            "site::siteType",
            "site::country",
            "site::stateProvince",
            "site::county",
            "site::municipality",
            "site::locality",
            "site::remark",
            "site::habitatType",
            "coordinate::decimalLatitude",
            "coordinate::decimalLongitude",
            "coordinate::elevationInMeter",
            "coordinate::datum",
            "coordinate::uncertaintyInMeters",
            "collEvent::startDate",
            "collEvent::startTime",
            "collEvent::primaryCollMethod",
            "collEvent::collMethodNotes",
            "collPersonnel::name",
            "collPersonnel::personnelId",
            "collEffort::method",
            "collEffort::notes",
            "narrative::narrative",
            "taxonomy::id",
            "taxonomy::taxonClass",
            "taxonomy::taxonOrder",
            "taxonomy::taxonFamily",
            "taxonomy::genus",
            "taxonomy::specificEpithet",
            "taxonomy::authors",
            "taxonomy::commonName",
            "taxonomy::notes",
            "specimen::uuid",
            "specimen::speciesID",
            "specimen::iDMethod",
            "specimen::taxonGroup",
            "specimen::trapType",
            "specimen::coordinateID",
            "specimen::fieldNumber",
            "specimen::museumID",
            "specimen::preparatorID",
            "specimenPart::barcodeID",
            "specimenPart::tissueID",
            "specimenPart::count",
            "specimenPart::remark",
            "weather::notes",
            "mammalAttribute::sex",
            "mammalAttribute::age",
            "mammalAttribute::reproductiveStage",
            "mammalAttribute::remark",
            "birdAttribute::habitatRemark",
            "birdAttribute::specimenRemark",
        ];

        for source_key in source_keys {
            let Some(term) = DwcMapper::get_dwc_term_for_source_key(source_key) else {
                panic!("expected a mapping for {source_key}");
            };
            if term.starts_with("dwc:") {
                assert!(
                    CURRENT_DWC_TERMS_USED_BY_NAHPU.contains(&term),
                    "{term} is not an approved current Darwin Core term"
                );
            }
        }
    }

    #[test]
    fn unsupported_legacy_terms_are_not_emitted() {
        for source_key in [
            "project::description",
            "project::location",
            "taxonomy::citesStatus",
            "taxonomy::redListCategory",
            "taxonomy::countryStatus",
        ] {
            assert_eq!(DwcMapper::get_dwc_term_for_source_key(source_key), None);
        }
    }

    #[test]
    fn supports_schema_acronyms_and_legacy_aliases() {
        for (source_key, expected) in [
            ("site::siteID", "dwc:siteNumber"),
            ("coordinate::siteID", "dwc:locationID"),
            ("collPersonnel::eventID", "dwc:eventID"),
            ("collEffort::eventID", "dwc:eventID"),
            ("specimen::speciesID", "dwc:taxonID"),
            ("specimen::collEventID", "dwc:eventID"),
            ("specimenPart::tissueID", "dwc:materialSampleID"),
            ("site::siteId", "dwc:siteNumber"),
            ("specimen::speciesId", "dwc:taxonID"),
            ("specimenPart::tissueId", "dwc:materialSampleID"),
            ("event::id", "dwc:eventID"),
        ] {
            assert_eq!(
                DwcMapper::get_dwc_term_for_source_key(source_key),
                Some(expected)
            );
        }
    }

    #[test]
    fn measurement_sources_expand_to_unsuffixed_measurement_or_fact_columns() {
        let mapping = DwcMapper::get_dwc_mapping_for_source_key("mammalAttribute::tailLength")
            .expect("tail length should be mapped");
        assert_eq!(
            mapping.headers,
            vec![
                "dwc:measurementType",
                "dwc:measurementValue",
                "dwc:measurementUnit",
            ]
        );
        assert_eq!(mapping.measurement_type, Some("tail length"));
        assert_eq!(mapping.measurement_unit, Some("mm"));
    }

    #[test]
    fn bird_beak_colors_are_measurement_or_fact_values() {
        for (source_key, measurement_type) in [
            ("birdAttribute::maxillaColor", "maxilla color"),
            ("birdAttribute::maxillaHex", "maxilla color hex"),
            ("birdAttribute::mandibleColor", "mandible color"),
            ("birdAttribute::mandibleHex", "mandible color hex"),
        ] {
            let mapping = DwcMapper::get_dwc_mapping_for_source_key(source_key)
                .expect("bird beak color should be mapped");
            assert_eq!(mapping.measurement_type, Some(measurement_type));
            assert_eq!(mapping.measurement_unit, None);
        }
    }

    #[test]
    fn legacy_foot_color_and_canonical_toe_color_share_a_mapping() {
        for source_key in ["birdAttribute::toeColor", "birdAttribute::footColor"] {
            let mapping = DwcMapper::get_dwc_mapping_for_source_key(source_key)
                .expect("toe color should be mapped");
            assert_eq!(mapping.measurement_type, Some("toe color"));
        }
    }

    #[test]
    fn legacy_attribute_names_resolve_to_the_canonical_mapping() {
        for (legacy, canonical) in [
            (
                "mammalMeasurement::tailLength",
                "mammalAttribute::tailLength",
            ),
            ("avianMeasurement::wingspan", "birdAttribute::wingspan"),
            ("herpMeasurement::svl", "herpAttribute::svl"),
        ] {
            assert_eq!(
                DwcMapper::get_dwc_mapping_for_source_key(legacy),
                DwcMapper::get_dwc_mapping_for_source_key(canonical)
            );
        }
    }

    #[test]
    fn preparator_is_an_agent_not_a_preparation_method() {
        assert_eq!(
            DwcMapper::get_dwc_term_for_source_key("specimen::preparatorID"),
            Some("dwc:recordedBy")
        );
        assert_eq!(
            DwcMapper::get_dwc_term_for_source_key("specimenPart::treatment"),
            Some("dwc:preparations")
        );
    }
}
